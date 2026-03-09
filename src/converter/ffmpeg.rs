use std::path::Path;
use std::process::{Command, Stdio};

use crate::config::{HwAccel, MediaForgeConfig};

/// Trait to add creation_flags on Windows Command
trait CommandExt {
    fn creation_flags(&mut self, flags: u32) -> &mut Self;
}

impl CommandExt for Command {
    #[cfg(target_os = "windows")]
    fn creation_flags(&mut self, flags: u32) -> &mut Self {
        use std::os::windows::process::CommandExt as WinCmdExt;
        WinCmdExt::creation_flags(self, flags);
        self
    }

    #[cfg(not(target_os = "windows"))]
    fn creation_flags(&mut self, _flags: u32) -> &mut Self {
        self
    }
}

/// Output format descriptors — uses static strings to avoid heap allocations
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutputFormat {
    pub label: &'static str,
    pub extension: &'static str,
    pub category: FormatCategory,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FormatCategory {
    Image,
    Video,
    Audio,
}

pub fn image_output_formats() -> &'static [OutputFormat] {
    static FORMATS: &[OutputFormat] = &[
        OutputFormat { label: "PNG", extension: "png", category: FormatCategory::Image },
        OutputFormat { label: "JPG", extension: "jpg", category: FormatCategory::Image },
        OutputFormat { label: "WebP", extension: "webp", category: FormatCategory::Image },
        OutputFormat { label: "BMP", extension: "bmp", category: FormatCategory::Image },
        OutputFormat { label: "TIFF", extension: "tiff", category: FormatCategory::Image },
        OutputFormat { label: "GIF", extension: "gif", category: FormatCategory::Image },
        OutputFormat { label: "ICO", extension: "ico", category: FormatCategory::Image },
        OutputFormat { label: "AVIF", extension: "avif", category: FormatCategory::Image },
    ];
    FORMATS
}

pub fn video_output_formats() -> &'static [OutputFormat] {
    static FORMATS: &[OutputFormat] = &[
        OutputFormat { label: "MP4 (H.264)", extension: "mp4", category: FormatCategory::Video },
        OutputFormat { label: "MP4 (H.265)", extension: "mp4", category: FormatCategory::Video },
        OutputFormat { label: "MKV", extension: "mkv", category: FormatCategory::Video },
        OutputFormat { label: "AVI", extension: "avi", category: FormatCategory::Video },
        OutputFormat { label: "MOV", extension: "mov", category: FormatCategory::Video },
        OutputFormat { label: "WebM (VP9)", extension: "webm", category: FormatCategory::Video },
        OutputFormat { label: "WMV", extension: "wmv", category: FormatCategory::Video },
        OutputFormat { label: "FLV", extension: "flv", category: FormatCategory::Video },
        OutputFormat { label: "MPEG", extension: "mpeg", category: FormatCategory::Video },
        OutputFormat { label: "3GP", extension: "3gp", category: FormatCategory::Video },
        OutputFormat { label: "TS", extension: "ts", category: FormatCategory::Video },
        OutputFormat { label: "GIF", extension: "gif", category: FormatCategory::Video },
        OutputFormat { label: "OGV", extension: "ogv", category: FormatCategory::Video },
    ];
    FORMATS
}

pub fn audio_output_formats() -> &'static [OutputFormat] {
    static FORMATS: &[OutputFormat] = &[
        OutputFormat { label: "MP3", extension: "mp3", category: FormatCategory::Audio },
        OutputFormat { label: "WAV", extension: "wav", category: FormatCategory::Audio },
        OutputFormat { label: "FLAC", extension: "flac", category: FormatCategory::Audio },
        OutputFormat { label: "AAC", extension: "aac", category: FormatCategory::Audio },
        OutputFormat { label: "OGG", extension: "ogg", category: FormatCategory::Audio },
        OutputFormat { label: "OPUS", extension: "opus", category: FormatCategory::Audio },
        OutputFormat { label: "WMA", extension: "wma", category: FormatCategory::Audio },
        OutputFormat { label: "AIFF", extension: "aiff", category: FormatCategory::Audio },
        OutputFormat { label: "AC3", extension: "ac3", category: FormatCategory::Audio },
        OutputFormat { label: "M4A", extension: "m4a", category: FormatCategory::Audio },
    ];
    FORMATS
}

/// Build FFmpeg command arguments for a video conversion
pub fn build_video_args(
    input: &Path,
    output: &Path,
    format_label: &str,
    config: &MediaForgeConfig,
) -> Vec<String> {
    let mut args = vec![
        "-i".to_string(),
        input.to_string_lossy().to_string(),
        "-progress".to_string(),
        "pipe:1".to_string(),
        "-stats_period".to_string(),
        "0.3".to_string(),
    ];

    // Video codec
    let video_codec = match format_label {
        "MP4 (H.265)" => "libx265",
        "WebM (VP9)" => "libvpx-vp9",
        "GIF" => {
            // Video to GIF special handling
            args.extend([
                "-vf".to_string(),
                "fps=15,scale=480:-1:flags=lanczos,split[s0][s1];[s0]palettegen[p];[s1][p]paletteuse".to_string(),
                "-loop".to_string(),
                "0".to_string(),
            ]);
            args.extend(["-y".to_string(), output.to_string_lossy().to_string()]);
            return args;
        }
        "OGV" => "libtheora",
        _ => "libx264",
    };

    args.extend(["-c:v".to_string(), video_codec.to_string()]);

    // CRF
    args.extend(["-crf".to_string(), config.video_crf.to_string()]);

    // Preset (only for x264/x265)
    if video_codec == "libx264" || video_codec == "libx265" {
        args.extend(["-preset".to_string(), config.video_preset.as_str().to_string()]);
    }

    // Resolution
    if let Some(scale) = config.video_resolution.scale_filter() {
        args.extend(["-vf".to_string(), format!("scale={scale}")]);
    }

    // Audio codec
    let audio_codec = match format_label {
        "WebM (VP9)" | "OGV" => "libvorbis",
        _ => "aac",
    };
    args.extend([
        "-c:a".to_string(),
        audio_codec.to_string(),
        "-b:a".to_string(),
        format!("{}k", config.audio_bitrate),
    ]);

    // Hardware acceleration
    match config.hw_accel {
        HwAccel::Nvidia if video_codec == "libx264" => {
            if let Some(pos) = args.iter().position(|a| a == "libx264") {
                args[pos] = "h264_nvenc".to_string();
            }
        }
        HwAccel::Nvidia if video_codec == "libx265" => {
            if let Some(pos) = args.iter().position(|a| a == "libx265") {
                args[pos] = "hevc_nvenc".to_string();
            }
        }
        _ => {}
    }

    if let Some(threads) = config.ffmpeg_threads {
        args.extend(["-threads".to_string(), threads.to_string()]);
    }

    args.extend(["-y".to_string(), output.to_string_lossy().to_string()]);
    args
}

/// Build FFmpeg command arguments for an audio conversion
pub fn build_audio_args(
    input: &Path,
    output: &Path,
    format_label: &str,
    config: &MediaForgeConfig,
) -> Vec<String> {
    let mut args = vec![
        "-nostats".to_string(),
        "-i".to_string(),
        input.to_string_lossy().to_string(),
        "-progress".to_string(),
        "pipe:1".to_string(),
        "-stats_period".to_string(),
        "0.3".to_string(),
        "-vn".to_string(),
    ];

    // Audio codec based on output format
    let codec = match format_label {
        "MP3" => "libmp3lame",
        "WAV" => "pcm_s16le",
        "FLAC" => "flac",
        "AAC" | "M4A" => "aac",
        "OGG" => "libvorbis",
        "OPUS" => "libopus",
        "WMA" => "wmav2",
        "AIFF" => "pcm_s16be",
        "AC3" => "ac3",
        _ => "copy",
    };

    args.extend(["-c:a".to_string(), codec.to_string()]);

    // Bitrate (not for lossless formats)
    if !matches!(format_label, "WAV" | "FLAC" | "AIFF") {
        args.extend(["-b:a".to_string(), format!("{}k", config.audio_bitrate)]);
    }

    // Sample rate
    args.extend(["-ar".to_string(), config.audio_sample_rate.to_string()]);

    // Channels
    if let Some(ch) = config.audio_channels.channel_count() {
        args.extend(["-ac".to_string(), ch.to_string()]);
    }

    if let Some(threads) = config.ffmpeg_threads {
        args.extend(["-threads".to_string(), threads.to_string()]);
    }

    args.extend(["-y".to_string(), output.to_string_lossy().to_string()]);
    args
}

/// Get FFmpeg version string
pub fn get_ffmpeg_version(ffmpeg_path: &str) -> Option<String> {
    let output = Command::new(ffmpeg_path)
        .arg("-version")
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .creation_flags(0x08000000) // CREATE_NO_WINDOW
        .output()
        .ok()?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    stdout.lines().next().map(|l| l.trim().to_string())
}
