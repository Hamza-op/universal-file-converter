use std::fs;
use std::path::Path;
use std::sync::OnceLock;

// Embed binaries (Windows only)
#[cfg(target_os = "windows")]
const FFMPEG_BIN: &[u8] = include_bytes!("../../bin/ffmpeg.exe");
#[cfg(target_os = "windows")]
const FFPROBE_BIN: &[u8] = include_bytes!("../../bin/ffprobe.exe");

#[cfg(target_os = "windows")]
static FFMPEG_PATHS: OnceLock<(String, String)> = OnceLock::new();

#[cfg(target_os = "windows")]
pub fn get_ffmpeg_paths() -> Option<(String, String)> {
    Some(FFMPEG_PATHS.get_or_init(|| {
        let temp_dir = std::env::temp_dir().join("MediaForge_FFmpeg");
        if !temp_dir.exists() && fs::create_dir_all(&temp_dir).is_err() {
            return (String::new(), String::new());
        }
        
        let ffmpeg_path = temp_dir.join("ffmpeg.exe");
        let ffprobe_path = temp_dir.join("ffprobe.exe");

        if ensure_file(&ffmpeg_path, FFMPEG_BIN).is_err() {
            return (String::new(), String::new());
        }
        if ensure_file(&ffprobe_path, FFPROBE_BIN).is_err() {
            return (String::new(), String::new());
        }

        (
            ffmpeg_path.to_string_lossy().to_string(),
            ffprobe_path.to_string_lossy().to_string(),
        )
    }).clone()).filter(|(ffmpeg, ffprobe)| !ffmpeg.is_empty() && !ffprobe.is_empty())
}

#[cfg(not(target_os = "windows"))]
pub fn get_ffmpeg_paths() -> Option<(String, String)> {
    None
}

#[cfg(target_os = "windows")]
fn ensure_file(path: &Path, content: &[u8]) -> std::io::Result<()> {
    if path.exists() {
        // Check file size via metadata — avoids reading the entire binary into RAM
        if let Ok(meta) = std::fs::metadata(path) {
            if meta.len() == content.len() as u64 {
                return Ok(());
            }
        }
    }
    // Write or overwrite
    fs::write(path, content)
}
