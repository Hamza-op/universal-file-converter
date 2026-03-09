use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MediaType {
    Image,
    Video,
    Audio,
    Unknown,
}

#[allow(dead_code)]
impl MediaType {

    pub fn label(&self) -> &'static str {
        match self {
            MediaType::Image => "Image",
            MediaType::Video => "Video",
            MediaType::Audio => "Audio",
            MediaType::Unknown => "Unknown",
        }
    }
}

const IMAGE_EXTENSIONS: &[&str] = &[
    "png", "jpg", "jpeg", "webp", "bmp", "tiff", "tif", "gif", "ico", "avif",
    "heic", "heif", "svg", "cr2", "nef", "arw", "dng", "psd", "tga", "ppm",
    "pgm", "pbm", "exr", "hdr", "qoi",
];

const VIDEO_EXTENSIONS: &[&str] = &[
    "mp4", "mkv", "avi", "mov", "wmv", "flv", "webm", "mpeg", "mpg", "3gp",
    "3g2", "m4v", "vob", "ogv", "ts", "mts", "m2ts", "asf", "dv", "f4v",
    "rm", "rmvb",
];

const AUDIO_EXTENSIONS: &[&str] = &[
    "mp3", "wav", "flac", "aac", "ogg", "wma", "m4a", "opus", "aiff", "aif",
    "amr", "ac3", "dts", "ape", "wv", "mka", "spx", "caf", "au", "ra",
];

pub fn detect_media_type(path: &Path) -> MediaType {
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        let ext_lower = ext.to_lowercase();
        if IMAGE_EXTENSIONS.contains(&ext_lower.as_str()) {
            return MediaType::Image;
        }
        if VIDEO_EXTENSIONS.contains(&ext_lower.as_str()) {
            return MediaType::Video;
        }
        if AUDIO_EXTENSIONS.contains(&ext_lower.as_str()) {
            return MediaType::Audio;
        }
    }

    // Fall back to magic byte detection
    if let Ok(kind) = infer::get_from_path(path) {
        if let Some(kind) = kind {
            let mime = kind.mime_type();
            if mime.starts_with("image/") {
                return MediaType::Image;
            }
            if mime.starts_with("video/") {
                return MediaType::Video;
            }
            if mime.starts_with("audio/") {
                return MediaType::Audio;
            }
        }
    }

    MediaType::Unknown
}

pub fn is_supported_extension(ext: &str) -> bool {
    let ext_lower = ext.to_lowercase();
    IMAGE_EXTENSIONS.contains(&ext_lower.as_str())
        || VIDEO_EXTENSIONS.contains(&ext_lower.as_str())
        || AUDIO_EXTENSIONS.contains(&ext_lower.as_str())
}

pub fn supported_extensions() -> Vec<&'static str> {
    let mut exts = Vec::with_capacity(
        IMAGE_EXTENSIONS.len() + VIDEO_EXTENSIONS.len() + AUDIO_EXTENSIONS.len(),
    );
    exts.extend_from_slice(IMAGE_EXTENSIONS);
    exts.extend_from_slice(VIDEO_EXTENSIONS);
    exts.extend_from_slice(AUDIO_EXTENSIONS);
    exts
}



/// Recursively scan a directory for supported media files
pub fn scan_directory(dir: &Path, max_depth: usize) -> Vec<std::path::PathBuf> {
    let mut results = Vec::new();
    scan_dir_recursive(dir, max_depth, 0, &mut results);
    results
}

fn scan_dir_recursive(dir: &Path, max_depth: usize, current_depth: usize, results: &mut Vec<std::path::PathBuf>) {
    if current_depth > max_depth {
        return;
    }
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            scan_dir_recursive(&path, max_depth, current_depth + 1, results);
        } else if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            if is_supported_extension(ext) {
                results.push(path);
            }
        }
    }
}
