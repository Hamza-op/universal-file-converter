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
        "png" | "jpg" | "jpeg" | "webp" | "bmp" | "tiff" | "tif" | "gif" | "ico" | "avif"
    );
    let native_output = ext_to_image_format(output_ext).is_some();
    native_input && native_output
}
