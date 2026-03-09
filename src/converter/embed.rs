use std::fs;
use std::path::Path;
use std::sync::OnceLock;


// Embed binaries
const FFMPEG_BIN: &[u8] = include_bytes!("../../bin/ffmpeg.exe");
const FFPROBE_BIN: &[u8] = include_bytes!("../../bin/ffprobe.exe");

static FFMPEG_PATHS: OnceLock<(String, String)> = OnceLock::new();

pub fn get_ffmpeg_paths() -> (String, String) {
    FFMPEG_PATHS.get_or_init(|| {
        let temp_dir = std::env::temp_dir().join("MediaForge_FFmpeg");
        if !temp_dir.exists() {
            let _ = fs::create_dir_all(&temp_dir);
        }
        
        let ffmpeg_path = temp_dir.join("ffmpeg.exe");
        let ffprobe_path = temp_dir.join("ffprobe.exe");

        ensure_file(&ffmpeg_path, FFMPEG_BIN);
        ensure_file(&ffprobe_path, FFPROBE_BIN);

        (
            ffmpeg_path.to_string_lossy().to_string(),
            ffprobe_path.to_string_lossy().to_string(),
        )
    }).clone()
}

fn ensure_file(path: &Path, content: &[u8]) {
    if path.exists() {
        // Check file size via metadata — avoids reading the entire binary into RAM
        if let Ok(meta) = std::fs::metadata(path) {
            if meta.len() == content.len() as u64 {
                return;
            }
        }
    }
    // Write or overwrite
    let _ = fs::write(path, content);
}
