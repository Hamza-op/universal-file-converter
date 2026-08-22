use std::fs;
#[cfg(target_os = "windows")]
use std::io::{BufReader, Read};
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
pub fn get_ffmpeg_paths(custom_temp_dir: Option<std::path::PathBuf>) -> Option<(String, String)> {
    Some(
        FFMPEG_PATHS
            .get_or_init(|| {
                let base_dir = custom_temp_dir.unwrap_or_else(std::env::temp_dir);
                let temp_dir = base_dir.join("MediaForge_FFmpeg");
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
            })
            .clone(),
    )
    .filter(|(ffmpeg, ffprobe)| !ffmpeg.is_empty() && !ffprobe.is_empty())
}

#[cfg(not(target_os = "windows"))]
pub fn get_ffmpeg_paths(_custom_temp_dir: Option<std::path::PathBuf>) -> Option<(String, String)> {
    None
}

#[cfg(target_os = "windows")]
fn ensure_file(path: &Path, content: &[u8]) -> std::io::Result<()> {
    if file_matches(path, content)? {
        return Ok(());
    }

    let staged_path = path.with_extension(format!("exe.mediaforge-{}-tmp", std::process::id()));
    fs::write(&staged_path, content)?;
    if path.exists() {
        fs::remove_file(path)?;
    }
    match fs::rename(&staged_path, path) {
        Ok(()) => Ok(()),
        Err(error) => {
            let _ = fs::remove_file(&staged_path);
            Err(error)
        }
    }
}

#[cfg(target_os = "windows")]
fn file_matches(path: &Path, expected: &[u8]) -> std::io::Result<bool> {
    let file = match fs::File::open(path) {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(error) => return Err(error),
    };
    if file.metadata()?.len() != expected.len() as u64 {
        return Ok(false);
    }

    let mut reader = BufReader::with_capacity(64 * 1024, file);
    let mut offset = 0;
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let read = reader.read(&mut buffer)?;
        if read == 0 {
            return Ok(offset == expected.len());
        }
        if expected.get(offset..offset + read) != Some(&buffer[..read]) {
            return Ok(false);
        }
        offset += read;
    }
}

#[cfg(all(test, target_os = "windows"))]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn same_size_tampering_is_repaired() {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock must be after epoch")
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("mediaforge-embed-test-{nonce}"));
        fs::create_dir_all(&dir).expect("test directory must be creatable");
        let path = dir.join("tool.exe");
        fs::write(&path, b"evil").expect("tampered fixture must be writable");

        ensure_file(&path, b"safe").expect("embedded file must be repaired");
        assert_eq!(
            fs::read(&path).expect("repaired file must be readable"),
            b"safe"
        );

        let _ = fs::remove_dir_all(dir);
    }
}
