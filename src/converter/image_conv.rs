use image::ImageFormat;
use std::path::Path;

/// Convert an image using the `image` crate (pure Rust, no FFmpeg needed)
pub fn convert_image(
    input: &Path,
    output: &Path,
    quality: u8,
    resize: Option<(u32, u32)>,
) -> Result<(), String> {
    let img = image::open(input).map_err(|e| format!("Failed to open image: {e}"))?;

    let img = if let Some((w, h)) = resize {
        img.resize(w, h, image::imageops::FilterType::Lanczos3)
    } else {
        img
    };

    let format = output
        .extension()
        .and_then(|e| e.to_str())
        .and_then(ext_to_image_format)
        .ok_or_else(|| "Unsupported output image format".to_string())?;

    match format {
        ImageFormat::Jpeg => {
            let file = std::fs::File::create(output)
                .map_err(|e| format!("Failed to create output file: {e}"))?;
            let mut encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(file, quality);
            encoder
                .encode_image(&img)
                .map_err(|e| format!("JPEG encode failed: {e}"))?;
        }
        ImageFormat::WebP => {
            // The image crate's WebP encoder doesn't support quality param directly,
            // so we just save with default settings
            img.save(output)
                .map_err(|e| format!("WebP encode failed: {e}"))?;
        }
        ImageFormat::Ico => {
            // ICO's embedded PNG representation requires RGBA; saving an RGB
            // DynamicImage produces an ICO that common decoders reject.
            image::DynamicImage::ImageRgba8(img.to_rgba8())
                .save(output)
                .map_err(|e| format!("ICO encode failed: {e}"))?;
        }
        _ => {
            img.save(output)
                .map_err(|e| format!("Image save failed: {e}"))?;
        }
    }

    Ok(())
}

fn ext_to_image_format(ext: &str) -> Option<ImageFormat> {
    match ext.to_lowercase().as_str() {
        "png" => Some(ImageFormat::Png),
        "jpg" | "jpeg" => Some(ImageFormat::Jpeg),
        "webp" => Some(ImageFormat::WebP),
        "bmp" => Some(ImageFormat::Bmp),
        "tiff" | "tif" => Some(ImageFormat::Tiff),
        "gif" => Some(ImageFormat::Gif),
        "ico" => Some(ImageFormat::Ico),
        "avif" => Some(ImageFormat::Avif),
        _ => None,
    }
}

/// Check if a format can be handled by the image crate
pub fn can_handle_natively(input_ext: &str, output_ext: &str) -> bool {
    let native_input = matches!(
        input_ext.to_lowercase().as_str(),
        "png" | "jpg" | "jpeg" | "webp" | "bmp" | "tiff" | "tif" | "gif" | "ico"
    );
    let native_output = ext_to_image_format(output_ext).is_some();
    native_input && native_output
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    struct TestDir(PathBuf);

    impl TestDir {
        fn new() -> Self {
            let nonce = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("clock must be after epoch")
                .as_nanos();
            let path = std::env::temp_dir().join(format!("mediaforge-image-test-{nonce}"));
            fs::create_dir_all(&path).expect("test directory must be creatable");
            Self(path)
        }
    }

    impl Drop for TestDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn advertised_native_image_formats_round_trip() {
        let dir = TestDir::new();
        let input = dir.0.join("fixture.png");
        let fixture = image::RgbImage::from_fn(32, 24, |x, y| {
            image::Rgb([(x * 7) as u8, (y * 9) as u8, ((x + y) * 3) as u8])
        });
        fixture.save(&input).expect("PNG fixture must be writable");

        for extension in ["png", "jpg", "webp", "bmp", "tiff", "gif", "ico", "avif"] {
            let output = dir.0.join(format!("output.{extension}"));
            convert_image(&input, &output, 82, None)
                .unwrap_or_else(|error| panic!("{extension} conversion failed: {error}"));
            if extension == "avif" {
                assert!(
                    output.metadata().expect("AVIF output must exist").len() > 0,
                    "AVIF output was empty"
                );
            } else {
                let decoded = image::open(&output)
                    .unwrap_or_else(|error| panic!("{extension} output was unreadable: {error}"));
                assert_eq!(decoded.width(), 32, "{extension} width changed");
                assert_eq!(decoded.height(), 24, "{extension} height changed");
            }
        }
    }
}
