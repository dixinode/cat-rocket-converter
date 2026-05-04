use image::codecs::jpeg::JpegEncoder;
use image::imageops::FilterType;
use image::metadata::Orientation;
use image::{DynamicImage, ImageDecoder, ImageFormat, ImageReader};
use std::fs;
use std::io::Cursor;
use std::path::Path;
use std::process::Command;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResizeAlgorithm {
    Nearest,
    Bilinear,
    CatmullRom,
    Lanczos3,
    Gaussian,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputFormat {
    Jpeg,
    Png,
    Webp,
}

/// Load a supported image file into memory as a DynamicImage.
pub fn load_image(path: &str) -> Result<DynamicImage, String> {
    let ext = Path::new(path)
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();

    if matches!(ext.as_str(), "heic" | "heif") {
        return load_heic_image(path);
    }

    let mut img = image::open(path).map_err(|error| format!("failed to open image: {error}"))?;
    img.apply_orientation(read_image_orientation(path));
    Ok(img)
}

/// Resize an image to the exact width and height using the requested algorithm.
pub fn resize_image(
    img: DynamicImage,
    width: u32,
    height: u32,
    algorithm: ResizeAlgorithm,
) -> DynamicImage {
    img.resize_exact(width.max(1), height.max(1), filter_for(algorithm))
}

/// Encode an image to disk and return the final file size in bytes.
pub fn save_image(
    img: &DynamicImage,
    path: &str,
    format: OutputFormat,
    quality: u8,
) -> Result<u64, String> {
    let bytes = encode_image(img, format, quality)?;
    fs::write(path, &bytes).map_err(|error| format!("failed to write image: {error}"))?;
    Ok(bytes.len() as u64)
}

/// Estimate output size by encoding a small proxy image in memory and extrapolating the result.
pub fn estimate_file_size(
    img: &DynamicImage,
    format: OutputFormat,
    quality: u8,
    target_width: u32,
    target_height: u32,
) -> u64 {
    if target_width == 0 || target_height == 0 {
        return 0;
    }

    let resized = img.resize_exact(target_width, target_height, FilterType::Triangle);

    match encode_image(&resized, format, quality) {
        Ok(bytes) => {
            let pixel_count = u64::from(target_width) * u64::from(target_height);
            // Small constant overhead keeps the estimate from under-reporting tiny outputs.
            bytes.len() as u64 + pixel_count.min(4096)
        }
        Err(_) => 0,
    }
}

pub fn output_extension(format: OutputFormat) -> &'static str {
    match format {
        OutputFormat::Jpeg => "jpg",
        OutputFormat::Png => "png",
        OutputFormat::Webp => "webp",
    }
}

pub fn parse_output_format(value: &str) -> Result<OutputFormat, String> {
    match value.to_ascii_lowercase().as_str() {
        "jpeg" | "jpg" => Ok(OutputFormat::Jpeg),
        "png" => Ok(OutputFormat::Png),
        "webp" => Ok(OutputFormat::Webp),
        _ => Err(format!("unsupported output format: {value}")),
    }
}

pub fn parse_resize_algorithm(value: &str) -> Result<ResizeAlgorithm, String> {
    match value.to_ascii_lowercase().as_str() {
        "nearest" => Ok(ResizeAlgorithm::Nearest),
        "bilinear" => Ok(ResizeAlgorithm::Bilinear),
        "catmullrom" => Ok(ResizeAlgorithm::CatmullRom),
        "lanczos3" => Ok(ResizeAlgorithm::Lanczos3),
        "gaussian" => Ok(ResizeAlgorithm::Gaussian),
        _ => Err(format!("unsupported resize algorithm: {value}")),
    }
}

fn filter_for(algorithm: ResizeAlgorithm) -> FilterType {
    match algorithm {
        ResizeAlgorithm::Nearest => FilterType::Nearest,
        ResizeAlgorithm::Bilinear => FilterType::Triangle,
        ResizeAlgorithm::CatmullRom => FilterType::CatmullRom,
        ResizeAlgorithm::Lanczos3 => FilterType::Lanczos3,
        ResizeAlgorithm::Gaussian => FilterType::Gaussian,
    }
}

fn load_heic_image(path: &str) -> Result<DynamicImage, String> {
    let temp_dir =
        tempfile::tempdir().map_err(|error| format!("failed to create temp dir: {error}"))?;
    let output_path = temp_dir.path().join("converted.png");
    let orientation = read_orientation_with_sips(path).unwrap_or(Orientation::NoTransforms);
    let rotation = sips_rotation_degrees(orientation);
    let display_dimensions = read_heic_display_dimensions(path);

    let mut command = Command::new("sips");
    command.arg("-s").arg("format").arg("png");

    if let Some(rotation) = rotation {
        command.arg("-r").arg(rotation.to_string());
    }

    let output = command
        .arg(path)
        .arg("--out")
        .arg(&output_path)
        .output()
        .map_err(|error| format!("failed to invoke sips for HEIC decode: {error}"))?;

    if !output.status.success() {
        return Err("failed to decode HEIC image".into());
    }

    let mut image =
        image::open(&output_path).map_err(|error| format!("failed to read decoded HEIC image: {error}"))?;

    if let Some((display_width, display_height)) = display_dimensions {
        if image.width() == display_height && image.height() == display_width {
            image = image.rotate90();
        }
    }

    Ok(image)
}

fn read_image_orientation(path: &str) -> Orientation {
    if let Ok(reader) = ImageReader::open(path) {
        if let Ok(mut decoder) = reader.into_decoder() {
            if let Ok(orientation) = decoder.orientation() {
                if orientation != Orientation::NoTransforms {
                    return orientation;
                }
            }
        }
    }

    read_orientation_with_sips(path).unwrap_or(Orientation::NoTransforms)
}

fn read_orientation_with_sips(path: &str) -> Option<Orientation> {
    let output = Command::new("sips")
        .arg("-g")
        .arg("orientation")
        .arg(path)
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let stdout = String::from_utf8(output.stdout).ok()?;
    parse_sips_orientation(&stdout)
}

fn parse_sips_orientation(output: &str) -> Option<Orientation> {
    let value = output
        .lines()
        .find_map(|line| line.split_once("orientation:"))?
        .1
        .trim();

    let exif_orientation = value.parse::<u8>().ok()?;
    Orientation::from_exif(exif_orientation)
}

fn read_heic_display_dimensions(path: &str) -> Option<(u32, u32)> {
    let output = Command::new("mdls")
        .arg("-name")
        .arg("kMDItemPixelWidth")
        .arg("-name")
        .arg("kMDItemPixelHeight")
        .arg(path)
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let stdout = String::from_utf8(output.stdout).ok()?;
    parse_mdls_dimensions(&stdout)
}

fn parse_mdls_dimensions(output: &str) -> Option<(u32, u32)> {
    let mut width = None;
    let mut height = None;

    for line in output.lines().map(str::trim).filter(|line| !line.is_empty()) {
        if let Some((key, value)) = line.split_once('=') {
            let parsed = value.trim().parse::<u32>().ok();
            match key.trim() {
                "kMDItemPixelWidth" => width = parsed,
                "kMDItemPixelHeight" => height = parsed,
                _ => {}
            }
        }
    }

    let width = width?;
    let height = height?;
    Some((width, height))
}

fn sips_rotation_degrees(orientation: Orientation) -> Option<u16> {
    match orientation {
        Orientation::Rotate90 => Some(90),
        Orientation::Rotate180 => Some(180),
        Orientation::Rotate270 => Some(270),
        _ => None,
    }
}

fn encode_image(img: &DynamicImage, format: OutputFormat, quality: u8) -> Result<Vec<u8>, String> {
    match format {
        OutputFormat::Jpeg => encode_jpeg(img, quality),
        OutputFormat::Png => {
            let mut bytes = Vec::new();
            img.write_to(&mut Cursor::new(&mut bytes), ImageFormat::Png)
                .map_err(|error| format!("failed to encode PNG: {error}"))?;
            Ok(bytes)
        }
        OutputFormat::Webp => {
            let encoder = webp::Encoder::from_image(img)
                .map_err(|error| format!("failed to prepare WebP encoder: {error}"))?;
            Ok(encoder.encode(quality as f32).to_vec())
        }
    }
}

fn encode_jpeg(img: &DynamicImage, quality: u8) -> Result<Vec<u8>, String> {
    let mut bytes = Vec::new();
    let rgb = img.to_rgb8();
    let mut encoder = JpegEncoder::new_with_quality(&mut bytes, quality.clamp(1, 100));
    encoder
        .encode_image(&DynamicImage::ImageRgb8(rgb))
        .map_err(|error| format!("failed to encode JPEG: {error}"))?;
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{GenericImageView, Rgba, RgbaImage};

    fn sample_image(width: u32, height: u32) -> DynamicImage {
        let mut image = RgbaImage::new(width, height);
        for (x, y, pixel) in image.enumerate_pixels_mut() {
            *pixel = Rgba([(x % 255) as u8, (y % 255) as u8, 180, 255]);
        }
        DynamicImage::ImageRgba8(image)
    }

    fn write_fixture(path: &Path, format: ImageFormat) {
        sample_image(32, 24).save_with_format(path, format).unwrap();
    }

    #[test]
    fn test_apply_orientation_rotate_90() {
        let mut rotated = sample_image(40, 80);
        rotated.apply_orientation(Orientation::Rotate90);

        assert_eq!(rotated.dimensions(), (80, 40));
    }

    #[test]
    fn test_parse_sips_orientation() {
        let output = "/tmp/example.heic\n  orientation: 6\n";
        assert_eq!(parse_sips_orientation(output), Some(Orientation::Rotate90));
    }

    #[test]
    fn test_sips_rotation_degrees() {
        assert_eq!(sips_rotation_degrees(Orientation::Rotate90), Some(90));
        assert_eq!(sips_rotation_degrees(Orientation::Rotate180), Some(180));
        assert_eq!(sips_rotation_degrees(Orientation::Rotate270), Some(270));
        assert_eq!(sips_rotation_degrees(Orientation::NoTransforms), None);
    }

    #[test]
    fn test_parse_mdls_dimensions() {
        let output = "kMDItemPixelHeight = 5712\nkMDItemPixelWidth  = 4284\n";
        assert_eq!(parse_mdls_dimensions(output), Some((4284, 5712)));
    }

    #[test]
    fn test_load_jpeg() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("sample.jpg");
        write_fixture(&path, ImageFormat::Jpeg);
        let img = load_image(path.to_str().unwrap()).unwrap();
        assert!(img.width() > 0);
        assert!(img.height() > 0);
    }

    #[test]
    fn test_load_png() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("sample.png");
        write_fixture(&path, ImageFormat::Png);
        let img = load_image(path.to_str().unwrap()).unwrap();
        assert_eq!(img.dimensions(), (32, 24));
    }

    #[test]
    fn test_load_heic() {
        let dir = tempfile::tempdir().unwrap();
        let png_path = dir.path().join("sample.png");
        let heic_path = dir.path().join("sample.heic");
        write_fixture(&png_path, ImageFormat::Png);

        let status = Command::new("sips")
            .arg("-s")
            .arg("format")
            .arg("heic")
            .arg(&png_path)
            .arg("--out")
            .arg(&heic_path)
            .status()
            .unwrap();

        if status.success() {
            let img = load_image(heic_path.to_str().unwrap()).unwrap();
            assert!(img.width() > 0);
        }
    }

    #[test]
    fn test_resize_50pct() {
        let resized = resize_image(sample_image(1000, 500), 500, 250, ResizeAlgorithm::Lanczos3);
        assert_eq!(resized.dimensions(), (500, 250));
    }

    #[test]
    fn test_resize_aspect_exact() {
        let resized = resize_image(sample_image(1000, 500), 333, 167, ResizeAlgorithm::Lanczos3);
        assert_eq!(resized.dimensions(), (333, 167));
    }

    #[test]
    fn test_save_jpeg_q100() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("out.jpg");
        let size = save_image(
            &sample_image(200, 100),
            path.to_str().unwrap(),
            OutputFormat::Jpeg,
            100,
        )
        .unwrap();
        assert!(path.exists());
        assert!(size > 0);
    }

    #[test]
    fn test_save_jpeg_q10_smaller() {
        let dir = tempfile::tempdir().unwrap();
        let high = dir.path().join("high.jpg");
        let low = dir.path().join("low.jpg");
        let img = sample_image(1600, 900);
        let high_size = save_image(&img, high.to_str().unwrap(), OutputFormat::Jpeg, 100).unwrap();
        let low_size = save_image(&img, low.to_str().unwrap(), OutputFormat::Jpeg, 10).unwrap();
        assert!(low_size < high_size / 2);
    }

    #[test]
    fn test_save_png_lossless() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("out.png");
        let original = sample_image(100, 50);
        save_image(&original, path.to_str().unwrap(), OutputFormat::Png, 100).unwrap();
        let reloaded = image::open(path).unwrap();
        assert_eq!(original.get_pixel(0, 0), reloaded.get_pixel(0, 0));
    }

    #[test]
    fn test_save_webp() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("out.webp");
        let size = save_image(
            &sample_image(200, 100),
            path.to_str().unwrap(),
            OutputFormat::Webp,
            80,
        )
        .unwrap();
        assert!(path.exists());
        assert!(size > 0);
    }

    #[test]
    fn test_estimate_accuracy() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("out.jpg");
        let img = sample_image(1280, 720);
        let estimate = estimate_file_size(&img, OutputFormat::Jpeg, 80, 640, 360);
        let actual_image = resize_image(img, 640, 360, ResizeAlgorithm::Lanczos3);
        let actual = save_image(
            &actual_image,
            path.to_str().unwrap(),
            OutputFormat::Jpeg,
            80,
        )
        .unwrap();
        let delta = estimate.abs_diff(actual);
        assert!(delta <= actual / 4);
    }
}
