use std::path::Path;
use std::process::Command;

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

#[derive(Debug, Clone, Default)]
pub struct MediaMetadata {
    pub duration_secs: Option<f64>,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub codec: Option<String>,
    pub bitrate: Option<u64>,
    pub sample_rate: Option<u32>,
    pub channels: Option<u32>,
    pub frame_rate: Option<f64>,
    pub frame_count: Option<u64>,
}

#[allow(dead_code)]
impl MediaMetadata {
    pub fn resolution_string(&self) -> Option<String> {
        match (self.width, self.height) {
            (Some(w), Some(h)) => Some(format!("{w}×{h}")),
            _ => None,
        }
    }

    pub fn duration_string(&self) -> Option<String> {
        self.duration_secs.map(|d| {
            let total_secs = d as u64;
            let hours = total_secs / 3600;
            let mins = (total_secs % 3600) / 60;
            let secs = total_secs % 60;
            if hours > 0 {
                format!("{hours}:{mins:02}:{secs:02}")
            } else {
                format!("{mins}:{secs:02}")
            }
        })
    }

    pub fn info_string(&self) -> String {
        let mut parts = Vec::new();
        if let Some(res) = self.resolution_string() {
            parts.push(res);
        }
        if let Some(dur) = self.duration_string() {
            parts.push(dur);
        }
        if let Some(codec) = &self.codec {
            parts.push(codec.clone());
        }
        if parts.is_empty() {
            String::new()
        } else {
            parts.join(" | ")
        }
    }
}

/// Extract metadata using ffprobe
pub fn probe_media(path: &Path, ffprobe_path: &str) -> MediaMetadata {
    let mut meta = MediaMetadata::default();

    let output = Command::new(ffprobe_path)
        .args([
            "-v", "quiet",
            "-print_format", "json",
            "-show_format",
            "-show_streams",
        ])
        .arg(path)
        .creation_flags(0x08000000) // CREATE_NO_WINDOW
        .output();

    let Ok(output) = output else {
        return meta;
    };

    if !output.status.success() {
        return meta;
    }

    let Ok(json_str) = String::from_utf8(output.stdout) else {
        return meta;
    };

    let Ok(json) = serde_json::from_str::<serde_json::Value>(&json_str) else {
        return meta;
    };

    // Parse format-level info
    if let Some(format) = json.get("format") {
        if let Some(dur) = format.get("duration").and_then(|v| v.as_str()) {
            meta.duration_secs = dur.parse().ok();
        }
        if let Some(br) = format.get("bit_rate").and_then(|v| v.as_str()) {
            meta.bitrate = br.parse().ok();
        }
    }

    // Parse first relevant stream
    if let Some(streams) = json.get("streams").and_then(|v| v.as_array()) {
        for stream in streams {
            let codec_type = stream.get("codec_type").and_then(|v| v.as_str()).unwrap_or("");

            if codec_type == "video" {
                meta.width = stream.get("width").and_then(|v| v.as_u64()).map(|v| v as u32);
                meta.height = stream.get("height").and_then(|v| v.as_u64()).map(|v| v as u32);
                meta.codec = stream.get("codec_name").and_then(|v| v.as_str()).map(String::from);

                if let Some(r_frame_rate) = stream.get("r_frame_rate").and_then(|v| v.as_str()) {
                    if let Some((num, den)) = r_frame_rate.split_once('/') {
                        if let (Ok(n), Ok(d)) = (num.parse::<f64>(), den.parse::<f64>()) {
                            if d > 0.0 {
                                meta.frame_rate = Some(n / d);
                            }
                        }
                    }
                }

                if let Some(nb_frames) = stream.get("nb_frames").and_then(|v| v.as_str()) {
                    meta.frame_count = nb_frames.parse().ok();
                }

                if meta.duration_secs.is_none() {
                    if let Some(dur) = stream.get("duration").and_then(|v| v.as_str()) {
                        meta.duration_secs = dur.parse().ok();
                    }
                }
                break;
            } else if codec_type == "audio" && meta.codec.is_none() {
                meta.codec = stream.get("codec_name").and_then(|v| v.as_str()).map(String::from);
                meta.sample_rate = stream
                    .get("sample_rate")
                    .and_then(|v| v.as_str())
                    .and_then(|v| v.parse().ok());
                meta.channels = stream.get("channels").and_then(|v| v.as_u64()).map(|v| v as u32);

                if meta.duration_secs.is_none() {
                    if let Some(dur) = stream.get("duration").and_then(|v| v.as_str()) {
                        meta.duration_secs = dur.parse().ok();
                    }
                }
            }
        }
    }

    meta
}

/// Extract image metadata using the image crate
pub fn probe_image(path: &Path) -> MediaMetadata {
    let mut meta = MediaMetadata::default();

    if let Ok(reader) = image::ImageReader::open(path) {
        if let Ok(dims) = reader.into_dimensions() {
            meta.width = Some(dims.0);
            meta.height = Some(dims.1);
        }
    }

    meta
}
