use std::collections::{HashMap, HashSet, VecDeque};
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Instant;

use crossbeam_channel::Sender;
use parking_lot::Mutex;

use crate::config::MediaForgeConfig;
use crate::converter::ffmpeg::{self, FormatCategory, OutputFormat};
use crate::converter::image_conv;
use crate::converter::progress::{self, ProgressState};
use crate::media::detect::MediaType;
use crate::media::metadata;
use crate::platform::CommandExt;

#[derive(Debug, Clone)]
pub struct InputFile {
    pub path: PathBuf,
    pub media_type: MediaType,
    pub file_size: u64,
    pub selected: bool,
    pub metadata: metadata::MediaMetadata,
    pub status: FileStatus,
    /// Cached display name — computed once, avoids per-frame allocation
    pub cached_filename: String,
    /// Cached size string — computed once, avoids per-frame allocation
    pub cached_size_string: String,
    pub relative_path: Option<PathBuf>,
}

#[derive(Debug, Clone)]
pub struct JobTask {
    pub index: usize,
    pub path: PathBuf,
    pub filename: String,
    pub metadata: metadata::MediaMetadata,
    pub relative_path: Option<PathBuf>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum FileStatus {
    Pending,
    Converting,
    Done,
    Cancelled,
    Failed(String),
}

impl InputFile {
    pub fn new(path: PathBuf, media_type: MediaType) -> Self {
        let file_size = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
        let cached_filename = path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "unknown".to_string());
        let cached_size_string = format_size(file_size);
        Self {
            path,
            media_type,
            file_size,
            selected: true,
            metadata: metadata::MediaMetadata::default(),
            status: FileStatus::Pending,
            cached_filename,
            cached_size_string,
            relative_path: None,
        }
    }

    pub fn size_string(&self) -> &str {
        &self.cached_size_string
    }

    pub fn filename(&self) -> &str {
        &self.cached_filename
    }
}

pub fn format_size(bytes: u64) -> String {
    if bytes < 1024 {
        format!("{bytes} B")
    } else if bytes < 1024 * 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else if bytes < 1024 * 1024 * 1024 {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    } else {
        format!("{:.2} GB", bytes as f64 / (1024.0 * 1024.0 * 1024.0))
    }
}

#[derive(Debug, Clone)]
pub struct ConversionProgress {
    pub current_file_index: usize,
    pub current_file_position: usize,
    pub total_files: usize,
    pub current_file_name: String,
    pub current_file_pct: f64,
    pub overall_pct: f64,
    pub eta_secs: Option<f64>,
    pub speed_str: String,
    pub is_running: bool,
    pub is_complete: bool,
    pub succeeded: usize,
    pub failed: usize,
    pub cancelled: usize,
    pub log_lines: VecDeque<String>,
    pub(crate) file_progress: HashMap<usize, f64>,
}

impl Default for ConversionProgress {
    fn default() -> Self {
        Self {
            current_file_index: 0,
            current_file_position: 0,
            total_files: 0,
            current_file_name: String::new(),
            current_file_pct: 0.0,
            overall_pct: 0.0,
            eta_secs: None,
            speed_str: String::new(),
            is_running: false,
            is_complete: false,
            succeeded: 0,
            failed: 0,
            cancelled: 0,
            log_lines: VecDeque::new(),
            file_progress: HashMap::new(),
        }
    }
}

impl ConversionProgress {
    pub fn update_file_progress(&mut self, index: usize, pct: f64) {
        self.file_progress.insert(index, pct.clamp(0.0, 100.0));
        self.recalculate_overall_progress();
    }

    fn recalculate_overall_progress(&mut self) {
        if self.total_files == 0 {
            self.overall_pct = 0.0;
            return;
        }
        self.overall_pct =
            (self.file_progress.values().sum::<f64>() / self.total_files as f64).clamp(0.0, 100.0);
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConversionOutcome {
    Succeeded,
    Failed(String),
    Cancelled,
}

/// Messages from the conversion worker to the UI
#[derive(Debug, Clone)]
pub enum ConversionMessage {
    Started {
        total_files: usize,
    },
    FileStarted {
        index: usize,
        position: usize,
        name: String,
    },
    FileProgress {
        index: usize,
        pct: f64,
        speed: String,
        eta: Option<f64>,
    },
    FileDone {
        index: usize,
        outcome: ConversionOutcome,
    },
    AllDone {
        succeeded: usize,
        failed: usize,
        cancelled: usize,
    },
    Log(String),
}

/// Build the output path for a given input file
#[cfg(test)]
#[allow(clippy::too_many_arguments)]
fn build_output_path(
    input: &Path,
    relative_path: Option<&Path>,
    output_dir: Option<&PathBuf>,
    format: &OutputFormat,
    add_suffix: bool,
    suffix: &str,
    overwrite: bool,
    preserve_structure: bool,
) -> PathBuf {
    let mut reserved = HashSet::new();
    build_output_path_with_reserved(
        input,
        relative_path,
        output_dir,
        format,
        add_suffix,
        suffix,
        overwrite,
        preserve_structure,
        &mut reserved,
    )
}

#[allow(clippy::too_many_arguments)]
fn build_output_path_with_reserved(
    input: &Path,
    relative_path: Option<&Path>,
    output_dir: Option<&PathBuf>,
    format: &OutputFormat,
    add_suffix: bool,
    suffix: &str,
    overwrite: bool,
    preserve_structure: bool,
    reserved: &mut HashSet<String>,
) -> PathBuf {
    let dir = output_dir
        .cloned()
        .unwrap_or_else(|| input.parent().unwrap_or(Path::new(".")).to_path_buf());

    let stem = input
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "output".to_string());

    let base_name = if add_suffix {
        format!("{}({})", stem, suffix)
    } else {
        stem
    };

    let mut output =
        if let (true, Some(rel), Some(_)) = (preserve_structure, relative_path, output_dir) {
            let rel_parent = rel.parent().unwrap_or(Path::new(""));
            let dest_dir = dir.join(rel_parent);
            dest_dir.join(format!("{}.{}", base_name, format.extension))
        } else {
            dir.join(format!("{}.{}", base_name, format.extension))
        };

    let mut counter = 2u32;
    let parent_dir = output.parent().unwrap_or(Path::new(".")).to_path_buf();
    while paths_equivalent(input, &output)
        || reserved.contains(&output_key(&output))
        || (!overwrite && output.exists())
    {
        output = parent_dir.join(format!("{}({}).{}", base_name, counter, format.extension));
        counter += 1;
    }

    reserved.insert(output_key(&output));
    output
}

fn output_key(path: &Path) -> String {
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join(path)
    };
    let key = absolute.to_string_lossy().replace('/', "\\");
    if cfg!(windows) {
        key.to_lowercase()
    } else {
        key
    }
}

fn paths_equivalent(left: &Path, right: &Path) -> bool {
    output_key(left) == output_key(right)
}

static STAGING_COUNTER: AtomicU64 = AtomicU64::new(0);

struct StagedOutput {
    path: PathBuf,
    committed: bool,
}

impl StagedOutput {
    fn beside(final_path: &Path) -> Result<Self, String> {
        let parent = final_path
            .parent()
            .ok_or_else(|| "Failed to resolve output directory".to_string())?;
        let stem = final_path
            .file_stem()
            .and_then(|value| value.to_str())
            .unwrap_or("output");
        let extension = final_path
            .extension()
            .and_then(|value| value.to_str())
            .unwrap_or("tmp");
        let nonce = STAGING_COUNTER.fetch_add(1, Ordering::Relaxed);
        let filename = format!(
            ".{stem}.mediaforge-part-{}-{nonce}.{extension}",
            std::process::id()
        );
        Ok(Self {
            path: parent.join(filename),
            committed: false,
        })
    }

    fn commit(mut self, final_path: &Path, overwrite: bool) -> Result<(), String> {
        if final_path.exists() {
            if !overwrite {
                return Err(format!(
                    "Output '{}' appeared while conversion was running; the existing file was preserved",
                    final_path.display()
                ));
            }
            replace_existing(&self.path, final_path)?;
        } else {
            fs::rename(&self.path, final_path).map_err(|error| {
                format!(
                    "Failed to finalize output '{}': {error}",
                    final_path.display()
                )
            })?;
        }
        self.committed = true;
        Ok(())
    }
}

impl Drop for StagedOutput {
    fn drop(&mut self) {
        if !self.committed {
            let _ = fs::remove_file(&self.path);
        }
    }
}

fn replace_existing(staged_path: &Path, final_path: &Path) -> Result<(), String> {
    let backup = final_path.with_file_name(format!(
        ".{}.mediaforge-backup-{}",
        final_path
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or("output"),
        STAGING_COUNTER.fetch_add(1, Ordering::Relaxed)
    ));

    fs::rename(final_path, &backup).map_err(|error| {
        format!(
            "Failed to preserve existing output '{}': {error}",
            final_path.display()
        )
    })?;

    if let Err(error) = fs::rename(staged_path, final_path) {
        let restore_error = fs::rename(&backup, final_path).err();
        return Err(match restore_error {
            Some(restore_error) => format!(
                "Failed to replace output '{}': {error}; restoring the original also failed: {restore_error}",
                final_path.display()
            ),
            None => format!("Failed to replace output '{}': {error}", final_path.display()),
        });
    }

    let _ = fs::remove_file(&backup);
    Ok(())
}

#[derive(Debug)]
enum JobFailure {
    Failed(String),
    Cancelled,
}

impl From<String> for JobFailure {
    fn from(value: String) -> Self {
        Self::Failed(value)
    }
}

/// Start the conversion pipeline in a background thread
pub fn start_conversion(
    tasks: Vec<JobTask>,
    format: OutputFormat,
    config: MediaForgeConfig,
    output_dir: Option<PathBuf>,
    sender: Sender<ConversionMessage>,
    cancel_flag: Arc<Mutex<bool>>,
) {
    std::thread::spawn(move || {
        let total = tasks.len();

        let _ = sender.send(ConversionMessage::Started { total_files: total });

        let succeeded = Arc::new(AtomicUsize::new(0));
        let failed = Arc::new(AtomicUsize::new(0));
        let cancelled = Arc::new(AtomicUsize::new(0));
        let completed_count = Arc::new(AtomicUsize::new(0));

        let batch_start = Instant::now();
        let mut reserved_outputs = HashSet::new();
        let planned_tasks = tasks
            .into_iter()
            .enumerate()
            .map(|(position, task)| {
                let output_path = build_output_path_with_reserved(
                    &task.path,
                    task.relative_path.as_deref(),
                    output_dir.as_ref(),
                    &format,
                    config.add_suffix,
                    &config.default_suffix,
                    config.overwrite_existing,
                    config.preserve_folder_structure,
                    &mut reserved_outputs,
                );
                (task, output_path, position + 1)
            })
            .collect::<VecDeque<_>>();
        let tasks_queue = Arc::new(Mutex::new(planned_tasks));

        let num_workers = config.max_concurrent_conversions.clamp(1, 32).min(total);
        let mut workers = Vec::with_capacity(num_workers);

        for _ in 0..num_workers {
            let tasks_queue = Arc::clone(&tasks_queue);
            let cancel_flag = Arc::clone(&cancel_flag);
            let sender = sender.clone();
            let config = config.clone();
            let format = format.clone();
            let succeeded = Arc::clone(&succeeded);
            let failed = Arc::clone(&failed);
            let cancelled = Arc::clone(&cancelled);
            let completed_count = Arc::clone(&completed_count);
            let worker = std::thread::spawn(move || {
                loop {
                    // Check cancel
                    if *cancel_flag.lock() {
                        break;
                    }

                    // Pop task
                    let task = {
                        let mut queue = tasks_queue.lock();
                        queue.pop_front()
                    };

                    let Some((task, output_path, position)) = task else {
                        break; // Queue is empty
                    };

                    let _ = sender.send(ConversionMessage::FileStarted {
                        index: task.index,
                        position,
                        name: task.filename.clone(),
                    });

                    let result =
                        convert_task(&task, &output_path, &format, &config, &sender, &cancel_flag);

                    let outcome = match result {
                        Ok(()) => {
                            succeeded.fetch_add(1, Ordering::SeqCst);
                            ConversionOutcome::Succeeded
                        }
                        Err(JobFailure::Failed(error)) => {
                            failed.fetch_add(1, Ordering::SeqCst);
                            ConversionOutcome::Failed(error)
                        }
                        Err(JobFailure::Cancelled) => {
                            cancelled.fetch_add(1, Ordering::SeqCst);
                            ConversionOutcome::Cancelled
                        }
                    };

                    let _ = sender.send(ConversionMessage::FileDone {
                        index: task.index,
                        outcome,
                    });

                    let completed = completed_count.fetch_add(1, Ordering::SeqCst) + 1;

                    // Update overall progress
                    let overall_pct = (completed as f64 / total as f64) * 100.0;
                    let elapsed = batch_start.elapsed().as_secs_f64();
                    let eta = progress::calculate_eta(elapsed, overall_pct);
                    let _ = sender.send(ConversionMessage::FileProgress {
                        index: task.index,
                        pct: 100.0,
                        speed: String::new(),
                        eta,
                    });
                }
            });

            workers.push(worker);
        }

        // Wait for all workers to finish
        for worker in workers {
            let _ = worker.join();
        }

        // Workers stop before dequeuing more work once cancellation is requested.
        // Report every remaining item explicitly so the UI never leaves stale
        // "Pending" rows or inconsistent totals.
        let remaining = {
            let mut queue = tasks_queue.lock();
            queue.drain(..).collect::<Vec<_>>()
        };
        for (task, _, _) in remaining {
            cancelled.fetch_add(1, Ordering::SeqCst);
            let _ = sender.send(ConversionMessage::FileDone {
                index: task.index,
                outcome: ConversionOutcome::Cancelled,
            });
        }

        let succ_final = succeeded.load(Ordering::SeqCst);
        let fail_final = failed.load(Ordering::SeqCst);
        let cancelled_final = cancelled.load(Ordering::SeqCst);
        let _ = sender.send(ConversionMessage::AllDone {
            succeeded: succ_final,
            failed: fail_final,
            cancelled: cancelled_final,
        });
    });
}

fn convert_task(
    task: &JobTask,
    output_path: &Path,
    format: &OutputFormat,
    config: &MediaForgeConfig,
    sender: &Sender<ConversionMessage>,
    cancel_flag: &Arc<Mutex<bool>>,
) -> Result<(), JobFailure> {
    if *cancel_flag.lock() {
        return Err(JobFailure::Cancelled);
    }

    let parent = output_path
        .parent()
        .ok_or_else(|| JobFailure::Failed("Failed to resolve output directory".to_string()))?;
    fs::create_dir_all(parent).map_err(|error| {
        JobFailure::Failed(format!(
            "Failed to prepare output directory '{}': {error}",
            parent.display()
        ))
    })?;

    let staged = StagedOutput::beside(output_path).map_err(JobFailure::Failed)?;
    let conversion = match format.category {
        FormatCategory::Image => {
            let input_ext = task
                .path
                .extension()
                .and_then(|extension| extension.to_str())
                .unwrap_or("");
            if image_conv::can_handle_natively(input_ext, format.extension) {
                image_conv::convert_image(&task.path, &staged.path, config.image_quality, None)
                    .map_err(JobFailure::Failed)
            } else {
                run_ffmpeg_conversion(
                    task.index,
                    &task.path,
                    &staged.path,
                    format,
                    config,
                    &task.metadata,
                    sender,
                    cancel_flag,
                )
            }
        }
        FormatCategory::Video | FormatCategory::Audio => run_ffmpeg_conversion(
            task.index,
            &task.path,
            &staged.path,
            format,
            config,
            &task.metadata,
            sender,
            cancel_flag,
        ),
    };

    conversion?;
    if *cancel_flag.lock() {
        return Err(JobFailure::Cancelled);
    }
    staged
        .commit(output_path, config.overwrite_existing)
        .map_err(JobFailure::Failed)
}

/// Max stderr lines to keep in memory during a single file conversion
const MAX_STDERR_LINES: usize = 80;

#[allow(clippy::too_many_arguments)]
fn run_ffmpeg_conversion(
    index: usize,
    input: &Path,
    output: &Path,
    format: &OutputFormat,
    config: &MediaForgeConfig,
    input_meta: &metadata::MediaMetadata,
    sender: &Sender<ConversionMessage>,
    cancel_flag: &Arc<Mutex<bool>>,
) -> Result<(), JobFailure> {
    let file_start = Instant::now();
    let ffmpeg_path = config.ffmpeg_path();

    let args = match format.category {
        FormatCategory::Video => ffmpeg::build_video_args(input, output, format.label, config),
        FormatCategory::Audio => ffmpeg::build_audio_args(input, output, format.label, config),
        FormatCategory::Image => {
            vec![
                "-i".to_string(),
                input.to_string_lossy().to_string(),
                "-y".to_string(),
                output.to_string_lossy().to_string(),
            ]
        }
    };

    let _ = sender.send(ConversionMessage::Log(format!(
        "$ ffmpeg {}",
        args.join(" ")
    )));

    let mut child = Command::new(&ffmpeg_path)
        .args(&args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .creation_flags(0x08000000) // CREATE_NO_WINDOW
        .spawn()
        .map_err(|error| {
            JobFailure::Failed(format!(
                "Failed to launch conversion engine: {error}. Please ensure the application has permissions to run from the temporary directory."
            ))
        })?;

    // Reuse already cached metadata when available to reduce conversion startup latency.
    let mut total_duration_us = input_meta
        .duration_secs
        .map(|d| (d * 1_000_000.0) as u64)
        .unwrap_or(0);
    let mut total_frames = input_meta.frame_count.unwrap_or(0);
    if total_duration_us == 0 || total_frames == 0 {
        let meta = metadata::probe_media(input, &config.ffprobe_path());
        if total_duration_us == 0 {
            total_duration_us = meta
                .duration_secs
                .map(|d| (d * 1_000_000.0) as u64)
                .unwrap_or(0);
        }
        if total_frames == 0 {
            total_frames = meta.frame_count.unwrap_or(0);
        }
    }

    // Collect stderr in a bounded ring buffer to prevent unbounded memory growth
    let stderr_thread = if let Some(stderr) = child.stderr.take() {
        let sender = sender.clone();
        std::thread::spawn(move || {
            let reader = BufReader::new(stderr);
            let mut tail_lines: VecDeque<String> = VecDeque::with_capacity(MAX_STDERR_LINES);
            for line in reader.lines().map_while(Result::ok) {
                let _ = sender.send(ConversionMessage::Log(line.clone()));
                if tail_lines.len() >= MAX_STDERR_LINES {
                    tail_lines.pop_front();
                }
                tail_lines.push_back(line);
            }
            // Join only the tail for error extraction
            tail_lines.into_iter().collect::<Vec<_>>().join("\n")
        })
    } else {
        std::thread::spawn(String::new)
    };

    // Read progress from stdout line by line
    if let Some(stdout) = child.stdout.take() {
        let mut reader = BufReader::new(stdout);
        let mut accumulated = String::with_capacity(512);
        let mut line_buf = String::new();

        loop {
            line_buf.clear();

            // Check cancel before dropping into potentially blocking read
            if *cancel_flag.lock() {
                let _ = child.kill();
                let _ = child.wait();
                let _ = stderr_thread.join();
                return Err(JobFailure::Cancelled);
            }

            match reader.read_line(&mut line_buf) {
                Ok(0) => break, // EOF
                Ok(_) => {
                    let line = line_buf.trim_end();
                    accumulated.push_str(line);
                    accumulated.push('\n');

                    if line.starts_with("progress=") {
                        let prog = progress::parse_progress(&accumulated);

                        // Clear accumulated after parsing — prevents unbounded growth
                        accumulated.clear();

                        // Use duration-based progress, fallback to frame-count
                        let _pct = if total_duration_us > 0 {
                            prog.percentage(total_duration_us)
                        } else if total_frames > 0 {
                            ((prog.frame as f64 / total_frames as f64) * 100.0).clamp(0.0, 100.0)
                        } else {
                            0.0
                        }
                        .min(99.0);

                        let elapsed = file_start.elapsed().as_secs_f64();
                        let eta = progress::calculate_eta(elapsed, _pct);
                        let speed_str = if prog.speed > 0.0 {
                            format!("{:.1}x", prog.speed)
                        } else {
                            "N/A".to_string()
                        };
                        let _ = sender.send(ConversionMessage::FileProgress {
                            index,
                            pct: _pct,
                            speed: speed_str,
                            eta,
                        });

                        if prog.progress_state == ProgressState::End {
                            let _ = sender.send(ConversionMessage::Log(
                                "Finalizing output file...".to_string(),
                            ));
                            break;
                        }
                    }
                }
                Err(_) => break,
            }
        }
    }

    let stderr_output = stderr_thread.join().unwrap_or_default();

    let status = child
        .wait()
        .map_err(|error| JobFailure::Failed(format!("FFmpeg process error: {error}")))?;

    if status.success() {
        Ok(())
    } else {
        let error_msg = extract_ffmpeg_error(&stderr_output, status.code());

        // Fallback to software encoding if HW acceleration was requested and failed
        let is_hw = config.hw_accel != crate::config::HwAccel::Software;
        let stderr_lower = stderr_output.to_lowercase();
        let is_hw_error = stderr_lower.contains("nvenc")
            || stderr_lower.contains("cuda")
            || stderr_lower.contains("qsv")
            || stderr_lower.contains("amf")
            || stderr_lower.contains("hardware")
            || stderr_lower.contains("driver")
            || stderr_lower.contains("init_failed");

        if is_hw && is_hw_error {
            let _ = sender.send(ConversionMessage::Log(
                "Hardware acceleration failed. Falling back to software encoding...".to_string(),
            ));
            let mut sw_config = config.clone();
            sw_config.hw_accel = crate::config::HwAccel::Software;
            return run_ffmpeg_conversion(
                index,
                input,
                output,
                format,
                &sw_config,
                input_meta,
                sender,
                cancel_flag,
            );
        }

        Err(JobFailure::Failed(error_msg))
    }
}

fn extract_ffmpeg_error(stderr: &str, exit_code: Option<i32>) -> String {
    stderr
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .enumerate()
        .max_by_key(|(position, line)| {
            let lower = line.to_lowercase();
            let relevance = if lower.contains("not supported")
                || lower.contains("unsupported")
                || lower.contains("no such file")
                || lower.contains("permission denied")
                || lower.contains("unknown encoder")
                || lower.contains("could not open")
            {
                3
            } else if lower.contains("invalid")
                || lower.contains("error")
                || lower.contains("failed")
            {
                2
            } else {
                1
            };
            (relevance, *position)
        })
        .map(|(_, line)| line.to_string())
        .unwrap_or_else(|| format!("FFmpeg exited with code {}", exit_code.unwrap_or(-1)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::converter::ffmpeg::{FormatCategory, OutputFormat};
    use std::path::{Path, PathBuf};

    #[test]
    fn test_format_size() {
        assert_eq!(format_size(0), "0 B");
        assert_eq!(format_size(512), "512 B");
        assert_eq!(format_size(1024), "1.0 KB");
        assert_eq!(format_size(1024 * 1024), "1.0 MB");
        assert_eq!(format_size(1024 * 1024 * 1024 * 3), "3.00 GB");
    }

    #[test]
    fn test_build_output_path_simple() {
        let input = Path::new("files").join("video.mp4");
        let format = OutputFormat {
            label: "MP3",
            extension: "mp3",
            category: FormatCategory::Audio,
        };

        // Case 1: same directory, no suffix, overwrite = true
        let out1 = build_output_path(&input, None, None, &format, false, "converted", true, false);
        assert_eq!(out1, Path::new("files").join("video.mp3"));

        // Case 2: same directory, with suffix, overwrite = true
        let out2 = build_output_path(&input, None, None, &format, true, "custom", true, false);
        assert_eq!(out2, Path::new("files").join("video(custom).mp3"));
    }

    #[test]
    fn test_build_output_path_custom_dir_and_structure() {
        let input = Path::new("source").join("media").join("video.mp4");
        let rel_path = Path::new("media").join("video.mp4");
        let output_dir = PathBuf::from("destination");
        let format = OutputFormat {
            label: "MP3",
            extension: "mp3",
            category: FormatCategory::Audio,
        };

        // Case 3: custom output dir, preserve structure, overwrite = true
        let out3 = build_output_path(
            &input,
            Some(&rel_path),
            Some(&output_dir),
            &format,
            false,
            "converted",
            true,
            true,
        );
        assert_eq!(
            out3,
            Path::new("destination").join("media").join("video.mp3")
        );
    }

    #[test]
    fn test_reserved_output_paths_are_unique_within_a_batch() {
        let format = OutputFormat {
            label: "MP3",
            extension: "mp3",
            category: FormatCategory::Audio,
        };
        let output_dir = PathBuf::from("destination");
        let mut reserved = HashSet::new();

        let first = build_output_path_with_reserved(
            Path::new("source-a").join("track.wav").as_path(),
            None,
            Some(&output_dir),
            &format,
            false,
            "converted",
            true,
            false,
            &mut reserved,
        );
        let second = build_output_path_with_reserved(
            Path::new("source-b").join("track.flac").as_path(),
            None,
            Some(&output_dir),
            &format,
            false,
            "converted",
            true,
            false,
            &mut reserved,
        );

        assert_eq!(first, output_dir.join("track.mp3"));
        assert_eq!(second, output_dir.join("track(2).mp3"));
    }

    #[test]
    fn test_output_never_reuses_the_input_path() {
        let input = Path::new("files").join("video.mp4");
        let format = OutputFormat {
            label: "MP4 (H.264)",
            extension: "mp4",
            category: FormatCategory::Video,
        };

        let output =
            build_output_path(&input, None, None, &format, false, "converted", true, false);

        assert_eq!(output, Path::new("files").join("video(2).mp4"));
    }

    #[test]
    fn aggregate_progress_does_not_move_backwards_between_workers() {
        let mut progress = ConversionProgress {
            total_files: 2,
            ..Default::default()
        };
        progress.update_file_progress(0, 80.0);
        assert_eq!(progress.overall_pct, 40.0);
        progress.update_file_progress(1, 20.0);
        assert_eq!(progress.overall_pct, 50.0);
        progress.update_file_progress(0, 100.0);
        assert_eq!(progress.overall_pct, 60.0);
    }

    #[test]
    fn ffmpeg_error_extraction_prefers_actionable_diagnostics() {
        let stderr = "Error while opening encoder\nSpecified sample rate 44100 is not supported\nNothing was written into output file\n";
        assert_eq!(
            extract_ffmpeg_error(stderr, Some(1)),
            "Specified sample rate 44100 is not supported"
        );
    }

    #[cfg(target_os = "windows")]
    mod end_to_end {
        use super::*;
        use crate::config::{HwAccel, MediaForgeConfig};
        use crate::media::metadata::MediaMetadata;
        use crate::platform::CommandExt;
        use std::process::Command;
        use std::time::{Duration, SystemTime, UNIX_EPOCH};

        struct TestDir(PathBuf);

        impl TestDir {
            fn new(name: &str) -> Self {
                let nonce = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .expect("clock must be after epoch")
                    .as_nanos();
                let path = std::env::temp_dir().join(format!(
                    "mediaforge-test-{name}-{}-{nonce}",
                    std::process::id()
                ));
                fs::create_dir_all(&path).expect("test directory must be creatable");
                Self(path)
            }
        }

        impl Drop for TestDir {
            fn drop(&mut self) {
                let _ = fs::remove_dir_all(&self.0);
            }
        }

        fn bundled_ffmpeg() -> PathBuf {
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("bin")
                .join("ffmpeg.exe")
        }

        fn generate_fixture(path: &Path) {
            let status = Command::new(bundled_ffmpeg())
                .args([
                    "-hide_banner",
                    "-loglevel",
                    "error",
                    "-f",
                    "lavfi",
                    "-i",
                    "testsrc2=size=96x64:rate=15",
                    "-f",
                    "lavfi",
                    "-i",
                    "sine=frequency=440:sample_rate=44100",
                    "-t",
                    "0.5",
                    "-c:v",
                    "libx264",
                    "-pix_fmt",
                    "yuv420p",
                    "-c:a",
                    "aac",
                    "-y",
                ])
                .arg(path)
                .creation_flags(0x08000000)
                .status()
                .expect("fixture FFmpeg must launch");
            assert!(status.success(), "fixture generation failed");
        }

        fn task(path: PathBuf) -> JobTask {
            JobTask {
                index: 0,
                filename: path
                    .file_name()
                    .expect("fixture has a file name")
                    .to_string_lossy()
                    .into_owned(),
                path,
                metadata: MediaMetadata::default(),
                relative_path: None,
            }
        }

        fn config(output_dir: &Path, overwrite: bool) -> MediaForgeConfig {
            MediaForgeConfig {
                add_suffix: false,
                overwrite_existing: overwrite,
                custom_output_dir: Some(output_dir.to_path_buf()),
                max_concurrent_conversions: 1,
                hw_accel: HwAccel::Software,
                show_notification: false,
                ..Default::default()
            }
        }

        fn run_job(
            input: PathBuf,
            output_dir: &Path,
            overwrite: bool,
            cancelled_at_start: bool,
        ) -> (usize, usize, usize) {
            let (sender, receiver) = crossbeam_channel::unbounded();
            let cancel_flag = Arc::new(Mutex::new(cancelled_at_start));
            let format = OutputFormat {
                label: "MPEG",
                extension: "mpeg",
                category: FormatCategory::Video,
            };
            start_conversion(
                vec![task(input)],
                format,
                config(output_dir, overwrite),
                Some(output_dir.to_path_buf()),
                sender,
                cancel_flag,
            );

            loop {
                if let ConversionMessage::AllDone {
                    succeeded,
                    failed,
                    cancelled,
                } = receiver
                    .recv_timeout(Duration::from_secs(30))
                    .expect("conversion must finish within timeout")
                {
                    return (succeeded, failed, cancelled);
                }
            }
        }

        fn assert_no_staging_files(dir: &Path) {
            let leftovers = fs::read_dir(dir)
                .expect("output directory must be readable")
                .flatten()
                .filter(|entry| entry.file_name().to_string_lossy().contains("mediaforge-"))
                .collect::<Vec<_>>();
            assert!(leftovers.is_empty(), "staging files were not cleaned up");
        }

        #[test]
        fn every_advertised_ffmpeg_format_converts_a_real_fixture() {
            let dir = TestDir::new("format-matrix");
            let input = dir.0.join("source.mp4");
            generate_fixture(&input);
            let config = MediaForgeConfig {
                hw_accel: HwAccel::Software,
                ..Default::default()
            };

            for (position, format) in crate::converter::ffmpeg::video_output_formats()
                .iter()
                .chain(crate::converter::ffmpeg::audio_output_formats())
                .enumerate()
            {
                let output = dir
                    .0
                    .join(format!("{position}-output.{}", format.extension));
                let args = match format.category {
                    FormatCategory::Video => crate::converter::ffmpeg::build_video_args(
                        &input,
                        &output,
                        format.label,
                        &config,
                    ),
                    FormatCategory::Audio => crate::converter::ffmpeg::build_audio_args(
                        &input,
                        &output,
                        format.label,
                        &config,
                    ),
                    FormatCategory::Image => unreachable!("format matrix excludes images"),
                };
                let process = Command::new(bundled_ffmpeg())
                    .args(args)
                    .stdout(Stdio::null())
                    .creation_flags(0x08000000)
                    .output()
                    .expect("format conversion must launch");
                assert!(
                    process.status.success(),
                    "{} conversion failed: {}",
                    format.label,
                    String::from_utf8_lossy(&process.stderr)
                );
                assert!(
                    output.metadata().expect("format output must exist").len() > 0,
                    "{} output was empty",
                    format.label
                );
            }
        }

        #[test]
        fn mpeg_conversion_succeeds_with_real_ffmpeg() {
            let dir = TestDir::new("mpeg-success");
            let input = dir.0.join("source.mp4");
            let output_dir = dir.0.join("output");
            generate_fixture(&input);

            assert_eq!(run_job(input, &output_dir, false, false), (1, 0, 0));
            let output = output_dir.join("source.mpeg");
            assert!(output.metadata().expect("output must exist").len() > 0);
            assert_no_staging_files(&output_dir);
        }

        #[test]
        fn failed_conversion_preserves_existing_output_and_cleans_staging() {
            let dir = TestDir::new("failure-cleanup");
            let input = dir.0.join("broken.mp4");
            let output_dir = dir.0.join("output");
            fs::create_dir_all(&output_dir).expect("output directory must be creatable");
            fs::write(&input, b"not media").expect("invalid fixture must be writable");
            let output = output_dir.join("broken.mpeg");
            fs::write(&output, b"original output").expect("existing output must be writable");

            assert_eq!(run_job(input, &output_dir, true, false), (0, 1, 0));
            assert_eq!(
                fs::read(&output).expect("existing output must remain readable"),
                b"original output"
            );
            assert_no_staging_files(&output_dir);
        }

        #[test]
        fn cancellation_reports_every_queued_file_without_creating_output() {
            let dir = TestDir::new("cancelled");
            let input = dir.0.join("source.mp4");
            let output_dir = dir.0.join("output");
            generate_fixture(&input);

            assert_eq!(run_job(input, &output_dir, false, true), (0, 0, 1));
            assert!(!output_dir.join("source.mpeg").exists());
        }
    }
}
