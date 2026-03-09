use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaForgeConfig {
    pub theme: Theme,
    pub default_suffix: String,
    pub add_suffix: bool,
    pub preserve_folder_structure: bool,
    pub overwrite_existing: bool,
    pub custom_output_dir: Option<PathBuf>,
    pub max_concurrent_conversions: usize,
    pub ffmpeg_threads: Option<usize>,
    pub hw_accel: HwAccel,
    pub temp_dir: Option<PathBuf>,
    pub context_menu_enabled: bool,
    pub play_sound_on_complete: bool,
    pub show_notification: bool,
    pub max_folder_scan_depth: usize,
    pub image_quality: u8,
    pub video_crf: u8,
    pub video_preset: VideoPreset,
    pub video_resolution: ResolutionPreset,
    pub audio_bitrate: u32,
    pub audio_sample_rate: u32,
    pub audio_channels: AudioChannels,
}

impl Default for MediaForgeConfig {
    fn default() -> Self {
        Self {
            theme: Theme::Dark,
            default_suffix: "converted".to_string(),
            add_suffix: true,
            preserve_folder_structure: false,
            overwrite_existing: false,
            custom_output_dir: None,
            max_concurrent_conversions: num_cpus(),
            ffmpeg_threads: None,
            hw_accel: HwAccel::Auto,
            temp_dir: None,
            context_menu_enabled: false,
            play_sound_on_complete: false,
            show_notification: true,
            max_folder_scan_depth: 10,
            image_quality: 85,
            video_crf: 23,
            video_preset: VideoPreset::Medium,
            video_resolution: ResolutionPreset::Original,
            audio_bitrate: 192,
            audio_sample_rate: 44100,
            audio_channels: AudioChannels::Original,
        }
    }
}

impl MediaForgeConfig {
    pub fn load() -> Self {
        // Try loading from next to the exe first
        if let Ok(exe_path) = std::env::current_exe() {
            let config_path = exe_path.with_file_name("mediaforge.toml");
            if let Some(config) = Self::load_from(&config_path) {
                return config;
            }
        }

        // Fallback to %APPDATA%\MediaForge\
        if let Some(app_data) = dirs::config_dir() {
            let config_path = app_data.join("MediaForge").join("mediaforge.toml");
            if let Some(config) = Self::load_from(&config_path) {
                return config;
            }
        }

        Self::default()
    }

    fn load_from(path: &Path) -> Option<Self> {
        let content = std::fs::read_to_string(path).ok()?;
        toml::from_str(&content).ok()
    }

    pub fn save(&self) {
        let content = match toml::to_string_pretty(self) {
            Ok(c) => c,
            Err(_) => return,
        };

        // Try saving next to the exe
        if let Ok(exe_path) = std::env::current_exe() {
            let config_path = exe_path.with_file_name("mediaforge.toml");
            if std::fs::write(&config_path, &content).is_ok() {
                return;
            }
        }

        // Fallback to %APPDATA%\MediaForge\
        if let Some(app_data) = dirs::config_dir() {
            let dir = app_data.join("MediaForge");
            let _ = std::fs::create_dir_all(&dir);
            let config_path = dir.join("mediaforge.toml");
            let _ = std::fs::write(config_path, &content);
        }
    }

    pub fn ffmpeg_path(&self) -> String {
        self.resolve_tool_path("ffmpeg.exe", true)
    }

    pub fn ffprobe_path(&self) -> String {
        self.resolve_tool_path("ffprobe.exe", true)
    }

    fn resolve_tool_path(&self, tool: &str, allow_embedded: bool) -> String {
        if let Ok(exe) = std::env::current_exe() {
            let next_to_exe = exe.with_file_name(tool);
            if next_to_exe.exists() {
                return next_to_exe.to_string_lossy().to_string();
            }
        }

        if allow_embedded {
            let (ffmpeg, ffprobe) = crate::converter::embed::get_ffmpeg_paths();
            let embedded = if tool.eq_ignore_ascii_case("ffmpeg.exe") {
                ffmpeg
            } else {
                ffprobe
            };
            if Path::new(&embedded).exists() {
                return embedded;
            }
        }

        tool.trim_end_matches(".exe").to_string()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Theme {
    Dark,
    Light,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HwAccel {
    Auto,
    Nvidia,
    Intel,
    Amd,
    Software,
}

impl HwAccel {
    pub const ALL: &'static [HwAccel] = &[
        HwAccel::Auto,
        HwAccel::Nvidia,
        HwAccel::Intel,
        HwAccel::Amd,
        HwAccel::Software,
    ];

    pub fn label(&self) -> &'static str {
        match self {
            HwAccel::Auto => "Auto Detect",
            HwAccel::Nvidia => "NVIDIA (NVENC)",
            HwAccel::Intel => "Intel (QSV)",
            HwAccel::Amd => "AMD (AMF)",
            HwAccel::Software => "Software Only",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum VideoPreset {
    Ultrafast,
    Superfast,
    Veryfast,
    Faster,
    Fast,
    Medium,
    Slow,
    Slower,
    Veryslow,
}

impl VideoPreset {
    pub const ALL: &'static [VideoPreset] = &[
        VideoPreset::Ultrafast,
        VideoPreset::Superfast,
        VideoPreset::Veryfast,
        VideoPreset::Faster,
        VideoPreset::Fast,
        VideoPreset::Medium,
        VideoPreset::Slow,
        VideoPreset::Slower,
        VideoPreset::Veryslow,
    ];

    pub fn as_str(&self) -> &'static str {
        match self {
            VideoPreset::Ultrafast => "ultrafast",
            VideoPreset::Superfast => "superfast",
            VideoPreset::Veryfast => "veryfast",
            VideoPreset::Faster => "faster",
            VideoPreset::Fast => "fast",
            VideoPreset::Medium => "medium",
            VideoPreset::Slow => "slow",
            VideoPreset::Slower => "slower",
            VideoPreset::Veryslow => "veryslow",
        }
    }
}

impl std::fmt::Display for VideoPreset {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ResolutionPreset {
    Original,
    Res4K,
    Res1080p,
    Res720p,
    Res480p,
    Custom,
}

impl ResolutionPreset {
    pub const ALL: &'static [ResolutionPreset] = &[
        ResolutionPreset::Original,
        ResolutionPreset::Res4K,
        ResolutionPreset::Res1080p,
        ResolutionPreset::Res720p,
        ResolutionPreset::Res480p,
    ];

    pub fn label(&self) -> &'static str {
        match self {
            ResolutionPreset::Original => "Original",
            ResolutionPreset::Res4K => "4K (3840\u{00D7}2160)",
            ResolutionPreset::Res1080p => "1080p (1920\u{00D7}1080)",
            ResolutionPreset::Res720p => "720p (1280\u{00D7}720)",
            ResolutionPreset::Res480p => "480p (854\u{00D7}480)",
            ResolutionPreset::Custom => "Custom",
        }
    }

    pub fn scale_filter(&self) -> Option<&'static str> {
        match self {
            ResolutionPreset::Original => None,
            ResolutionPreset::Res4K => Some("3840:-2"),
            ResolutionPreset::Res1080p => Some("1920:-2"),
            ResolutionPreset::Res720p => Some("1280:-2"),
            ResolutionPreset::Res480p => Some("854:-2"),
            ResolutionPreset::Custom => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AudioChannels {
    Original,
    Mono,
    Stereo,
}

impl AudioChannels {
    pub const ALL: &'static [AudioChannels] = &[
        AudioChannels::Original,
        AudioChannels::Mono,
        AudioChannels::Stereo,
    ];

    pub fn label(&self) -> &'static str {
        match self {
            AudioChannels::Original => "Original",
            AudioChannels::Mono => "Mono",
            AudioChannels::Stereo => "Stereo",
        }
    }

    pub fn channel_count(&self) -> Option<u32> {
        match self {
            AudioChannels::Original => None,
            AudioChannels::Mono => Some(1),
            AudioChannels::Stereo => Some(2),
        }
    }
}

fn num_cpus() -> usize {
    std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4)
}
