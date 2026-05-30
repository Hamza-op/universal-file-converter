use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::Arc;
use std::io::{BufRead, BufReader};
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
    pub log_lines: VecDeque<String>,
}

impl Default for ConversionProgress {
    fn default() -> Self {
        Self {
            current_file_index: 0,
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
            log_lines: VecDeque::new(),
        }
    }
}

/// Messages from the conversion worker to the UI
#[derive(Debug, Clone)]
pub enum ConversionMessage {
    Started { total_files: usize },
    FileStarted { index: usize, name: String },
    FileProgress { index: usize, pct: f64, speed: String, eta: Option<f64> },
    FileDone { index: usize, success: bool, error: Option<String> },
    AllDone { succeeded: usize, failed: usize },
    Log(String),
}

/// Build the output path for a given input file
pub fn build_output_path(
    input: &Path,
    relative_path: Option<&Path>,
    output_dir: Option<&PathBuf>,
    format: &OutputFormat,
    add_suffix: bool,
    suffix: &str,
    overwrite: bool,
    preserve_structure: bool,
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

    let mut output = if preserve_structure && relative_path.is_some() && output_dir.is_some() {
        let rel = relative_path.unwrap();
        let rel_parent = rel.parent().unwrap_or(Path::new(""));
        let dest_dir = dir.join(rel_parent);
        dest_dir.join(format!("{}.{}", base_name, format.extension))
    } else {
        dir.join(format!("{}.{}", base_name, format.extension))
    };

    if !overwrite {
        let mut counter = 2u32;
        let parent_dir = output.parent().unwrap_or(Path::new(".")).to_path_buf();
        while output.exists() {
            output = parent_dir.join(format!("{}({}).{}", base_name, counter, format.extension));
            counter += 1;
        }
    }

    output
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

        let succeeded = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let failed = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let completed_count = Arc::new(std::sync::atomic::AtomicUsize::new(0));

        let batch_start = Instant::now();
        let tasks_queue = Arc::new(Mutex::new(tasks.into_iter().collect::<VecDeque<JobTask>>()));
        
        let num_workers = config.max_concurrent_conversions.clamp(1, 32).min(total);
        let mut workers = Vec::with_capacity(num_workers);

        for _ in 0..num_workers {
            let tasks_queue = Arc::clone(&tasks_queue);
            let cancel_flag = Arc::clone(&cancel_flag);
            let sender = sender.clone();
            let config = config.clone();
            let format = format.clone();
            let output_dir = output_dir.clone();
            let succeeded = Arc::clone(&succeeded);
            let failed = Arc::clone(&failed);
            let completed_count = Arc::clone(&completed_count);
            let batch_start = batch_start.clone();

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

                    let Some(task) = task else {
                        break; // Queue is empty
                    };

                    let _ = sender.send(ConversionMessage::FileStarted {
                        index: task.index,
                        name: task.filename.clone(),
                    });

                    let output_path = build_output_path(
                        &task.path,
                        task.relative_path.as_deref(),
                        output_dir.as_ref(),
                        &format,
                        config.add_suffix,
                        &config.default_suffix,
                        config.overwrite_existing,
                        config.preserve_folder_structure,
                    );

                    let result = if let Some(parent) = output_path.parent() {
                        if let Err(e) = std::fs::create_dir_all(parent) {
                            Err(format!(
                                "Failed to prepare output directory '{}': {e}",
                                parent.display()
                            ))
                        } else {
                            match format.category {
                                FormatCategory::Image => {
                                    // Check if we can use native image crate
                                    let input_ext = task
                                        .path
                                        .extension()
                                        .and_then(|e| e.to_str())
                                        .unwrap_or("");
                                    if image_conv::can_handle_natively(input_ext, format.extension) {
                                        image_conv::convert_image(
                                            &task.path,
                                            &output_path,
                                            config.image_quality,
                                            None,
                                        )
                                    } else {
                                        // Fall back to FFmpeg for exotic image formats
                                        run_ffmpeg_conversion(
                                            task.index,
                                            &task.path,
                                            &output_path,
                                            &format,
                                            &config,
                                            &task.metadata,
                                            &sender,
                                            &cancel_flag,
                                        )
                                    }
                                }
                                FormatCategory::Video | FormatCategory::Audio => run_ffmpeg_conversion(
                                    task.index,
                                    &task.path,
                                    &output_path,
                                    &format,
                                    &config,
                                    &task.metadata,
                                    &sender,
                                    &cancel_flag,
                                ),
                            }
                        }
                    } else {
                        Err("Failed to resolve output directory".to_string())
                    };

                    let success = result.is_ok();
                    if success {
                        succeeded.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                    } else {
                        failed.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                    }

                    let _ = sender.send(ConversionMessage::FileDone {
                        index: task.index,
                        success,
                        error: result.err(),
                    });

                    let completed = completed_count.fetch_add(1, std::sync::atomic::Ordering::SeqCst) + 1;

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

        let succ_final = succeeded.load(std::sync::atomic::Ordering::SeqCst);
        let fail_final = failed.load(std::sync::atomic::Ordering::SeqCst);
        let _ = sender.send(ConversionMessage::AllDone { succeeded: succ_final, failed: fail_final });
    });
}

/// Max stderr lines to keep in memory during a single file conversion
const MAX_STDERR_LINES: usize = 80;

fn run_ffmpeg_conversion(
    index: usize,
    input: &Path,
    output: &Path,
    format: &OutputFormat,
    config: &MediaForgeConfig,
    input_meta: &metadata::MediaMetadata,
    sender: &Sender<ConversionMessage>,
    cancel_flag: &Arc<Mutex<bool>>,
) -> Result<(), String> {
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
        .map_err(|e| format!("Failed to launch conversion engine: {e}. Please ensure the application has permissions to run from the temporary directory."))?;

    // Reuse already cached metadata when available to reduce conversion startup latency.
    let mut total_duration_us = input_meta
        .duration_secs
        .map(|d| (d * 1_000_000.0) as u64)
        .unwrap_or(0);
    let mut total_frames = input_meta.frame_count.unwrap_or(0);
    if total_duration_us == 0 || total_frames == 0 {
        let meta = metadata::probe_media(input, &config.ffprobe_path());
        if total_duration_us == 0 {
            total_duration_us = meta.duration_secs.map(|d| (d * 1_000_000.0) as u64).unwrap_or(0);
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
                return Err("Cancelled".to_string());
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
                            let _ = sender.send(ConversionMessage::Log("Finalizing output file...".to_string()));
                            break;
                        }
                    }
                }
                Err(_) => break,
            }
        }
    }

    let stderr_output = stderr_thread.join().unwrap_or_default();

    let status = child.wait().map_err(|e| format!("FFmpeg process error: {e}"))?;

    if status.success() {
        Ok(())
    } else {
        // Extract a meaningful error from stderr
        let error_lines: Vec<&str> = stderr_output
            .lines()
            .filter(|l| {
                let l_lower = l.to_lowercase();
                l_lower.contains("error") || l_lower.contains("invalid") || l_lower.contains("failed")
            })
            .collect();

        let error_msg = if error_lines.is_empty() {
            format!("FFmpeg exited with code {}", status.code().unwrap_or(-1))
        } else {
            error_lines.last().unwrap_or(&"Unknown error").to_string()
        };

        // Fallback to software encoding if HW acceleration was requested and failed
        let is_hw = config.hw_accel != crate::config::HwAccel::Software;
        let is_hw_error = error_msg.to_lowercase().contains("nvenc")
            || error_msg.to_lowercase().contains("cuda")
            || error_msg.to_lowercase().contains("qsv")
            || error_msg.to_lowercase().contains("amf")
            || error_msg.to_lowercase().contains("hardware")
            || error_msg.to_lowercase().contains("driver")
            || error_msg.to_lowercase().contains("init_failed");

        if is_hw && is_hw_error {
            let _ = sender.send(ConversionMessage::Log("Hardware acceleration failed. Falling back to software encoding...".to_string()));
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

        Err(error_msg)
    }
}
