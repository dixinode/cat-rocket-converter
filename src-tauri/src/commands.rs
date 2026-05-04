use crate::converter::{
    estimate_file_size, load_image, output_extension, parse_output_format, parse_resize_algorithm,
    resize_image, save_image,
};
use crate::file_utils::{apply_rename_pattern, RenamePattern};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use tauri::LogicalSize;
use tauri::Emitter;

pub struct PendingOpenFiles(pub Mutex<Vec<String>>);

impl Default for PendingOpenFiles {
    fn default() -> Self {
        Self(Mutex::new(Vec::new()))
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImageInfo {
    width: u32,
    height: u32,
    file_size: u64,
    detected_format: String,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SizeEstimate {
    width: u32,
    height: u32,
    size_bytes: u64,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ConvertParams {
    pub files: Vec<String>,
    pub output_dir: String,
    pub format: String,
    pub quality: u8,
    pub pct_w: u32,
    pub pct_h: u32,
    pub resize_w: u32,
    pub resize_h: u32,
    pub lock_aspect: bool,
    pub algorithm: String,
    pub rename_pattern: RenamePatternDto,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct EstimateParams {
    pub file: String,
    pub format: String,
    pub quality: u8,
    pub resize_w: u32,
    pub resize_h: u32,
    pub algorithm: String,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum RenamePatternDto {
    Datetime { prefix: String },
    Suffix { suffix: String },
    Sequential { base: String, start: u32 },
    Keep,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct ProgressPayload {
    current: usize,
    total: usize,
}

/// Resize the outer window so the visible webview viewport becomes exactly 428x318.
#[tauri::command]
pub fn sync_window_viewport(
    window: tauri::WebviewWindow,
    viewport_width: f64,
    viewport_height: f64,
) -> Result<(), String> {
    let target_width = 428.0;
    let target_height = 318.0;
    let width_delta = target_width - viewport_width;
    let height_delta = target_height - viewport_height;

    if width_delta.abs() < 0.5 && height_delta.abs() < 0.5 {
        return Ok(());
    }

    let scale_factor = window
        .scale_factor()
        .map_err(|error| format!("failed to read scale factor: {error}"))?;
    let outer_size = window
        .outer_size()
        .map_err(|error| format!("failed to read outer size: {error}"))?;

    let outer_width = f64::from(outer_size.width) / scale_factor;
    let outer_height = f64::from(outer_size.height) / scale_factor;
    let next_width = (outer_width + width_delta).round().max(target_width);
    let next_height = (outer_height + height_delta).round().max(target_height);

    window
        .set_size(LogicalSize::new(next_width, next_height))
        .map_err(|error| format!("failed to resize window: {error}"))?;

    Ok(())
}

/// Return width, height, file size, and detected format for the selected image.
#[tauri::command]
pub fn get_image_info(path: String) -> Result<ImageInfo, String> {
    let image = load_image(&path)?;
    let metadata =
        fs::metadata(&path).map_err(|error| format!("failed to read metadata: {error}"))?;
    let detected_format = Path::new(&path)
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or("unknown")
        .to_ascii_lowercase();

    Ok(ImageInfo {
        width: image.width(),
        height: image.height(),
        file_size: metadata.len(),
        detected_format,
    })
}

/// Convert all selected files and emit progress after each completed item.
#[tauri::command]
pub fn convert_images(app: tauri::AppHandle, params: ConvertParams) -> Result<Vec<String>, String> {
    convert_images_with_progress(&params, |current, total| {
        let _ = app.emit("progress", ProgressPayload { current, total });
    })
}

/// Play a macOS system sound without blocking the UI while the window hides.
#[tauri::command]
pub fn play_system_sound(sound_name: String) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        let sound_path = PathBuf::from("/System/Library/Sounds").join(format!("{sound_name}.aiff"));

        if !sound_path.exists() {
            return Err(format!("system sound not found: {}", sound_path.display()));
        }

        Command::new("afplay")
            .arg(&sound_path)
            .spawn()
            .map_err(|error| format!("failed to play system sound: {error}"))?;
    }

    #[cfg(not(target_os = "macos"))]
    {
        let _ = sound_name;
    }

    Ok(())
}

/// Drain files that were opened via Finder before the frontend listener was ready.
#[tauri::command]
pub fn take_pending_open_files(
    pending_open_files: tauri::State<'_, PendingOpenFiles>,
) -> Result<Vec<String>, String> {
    let mut pending = pending_open_files
        .0
        .lock()
        .map_err(|error| format!("failed to lock pending files state: {error}"))?;

    Ok(std::mem::take(&mut *pending))
}

fn convert_images_with_progress<F>(
    params: &ConvertParams,
    emit_progress: F,
) -> Result<Vec<String>, String>
where
    F: Fn(usize, usize) + Sync,
{
    fs::create_dir_all(&params.output_dir)
        .map_err(|error| format!("failed to create output directory: {error}"))?;

    let format = parse_output_format(&params.format)?;
    let algorithm = parse_resize_algorithm(&params.algorithm)?;
    let rename_pattern = dto_to_pattern(&params.rename_pattern);
    let output_ext = output_extension(format);
    let filenames = apply_rename_pattern(&params.files, &rename_pattern, output_ext);
    let total = params.files.len();
    let progress = Arc::new(AtomicUsize::new(0));

    params
        .files
        .iter()
        .enumerate()
        .map(|(index, file)| {
            let output_path = PathBuf::from(&params.output_dir).join(&filenames[index]);
            let image = load_image(file)?;
            let (target_width, target_height) = target_dimensions(&image, params);
            let resized = resize_image(image, target_width, target_height, algorithm);
            save_image(
                &resized,
                output_path.to_str().ok_or("invalid output path")?,
                format,
                params.quality,
            )?;

            drop(resized);

            let current = progress.fetch_add(1, Ordering::SeqCst) + 1;
            emit_progress(current, total);

            Ok(output_path.to_string_lossy().to_string())
        })
        .collect()
}

fn target_dimensions(image: &image::DynamicImage, params: &ConvertParams) -> (u32, u32) {
    if params.lock_aspect {
        let width = scaled_dimension(image.width(), params.pct_w);
        let height = scaled_dimension(image.height(), params.pct_h);
        return (width, height);
    }

    (params.resize_w.max(1), params.resize_h.max(1))
}

fn scaled_dimension(original: u32, percent: u32) -> u32 {
    let scaled = (u64::from(original) * u64::from(percent.max(1)) + 50) / 100;
    scaled.max(1) as u32
}

/// Estimate output dimensions and file size for the current resize settings.
#[tauri::command]
pub fn estimate_output(params: EstimateParams) -> Result<SizeEstimate, String> {
    let format = parse_output_format(&params.format)?;
    let _algorithm = parse_resize_algorithm(&params.algorithm)?;
    let image = load_image(&params.file)?;

    Ok(SizeEstimate {
        width: params.resize_w,
        height: params.resize_h,
        size_bytes: estimate_file_size(
            &image,
            format,
            params.quality,
            params.resize_w,
            params.resize_h,
        ),
    })
}

/// Show a folder picker defaulting to the provided directory and return the chosen path.
#[tauri::command]
pub fn open_save_dialog(default_dir: String) -> Option<String> {
    rfd::FileDialog::new()
        .set_directory(default_dir)
        .pick_folder()
        .map(|path| path.to_string_lossy().to_string())
}

/// Return the parent directory for a file path so the save dialog can default there.
#[tauri::command]
pub fn get_parent_dir(file_path: String) -> String {
    PathBuf::from(file_path)
        .parent()
        .unwrap_or(Path::new("."))
        .to_string_lossy()
        .to_string()
}

fn dto_to_pattern(dto: &RenamePatternDto) -> RenamePattern {
    match dto {
        RenamePatternDto::Datetime { prefix } => RenamePattern::DateTimePrefix {
            prefix: prefix.clone(),
        },
        RenamePatternDto::Suffix { suffix } => RenamePattern::OriginalSuffix {
            suffix: suffix.clone(),
        },
        RenamePatternDto::Sequential { base, start } => RenamePattern::Sequential {
            base: base.clone(),
            start: *start,
        },
        RenamePatternDto::Keep => RenamePattern::KeepOriginal,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{DynamicImage, GenericImageView, Rgba, RgbaImage};

    fn sample_image(path: &Path, format: image::ImageFormat) {
        let mut image = RgbaImage::new(120, 80);
        for (x, y, pixel) in image.enumerate_pixels_mut() {
            *pixel = Rgba([(x % 255) as u8, (y % 255) as u8, 128, 255]);
        }

        DynamicImage::ImageRgba8(image)
            .save_with_format(path, format)
            .unwrap();
    }

    #[test]
    fn full_jpeg_pipeline() {
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("source.png");
        let output_dir = dir.path().join("out");
        sample_image(&source, image::ImageFormat::Png);

        let params = ConvertParams {
            files: vec![source.to_string_lossy().to_string()],
            output_dir: output_dir.to_string_lossy().to_string(),
            format: "jpeg".into(),
            quality: 80,
            pct_w: 50,
            pct_h: 50,
            resize_w: 60,
            resize_h: 40,
            lock_aspect: true,
            algorithm: "lanczos3".into(),
            rename_pattern: RenamePatternDto::Datetime {
                prefix: "IMG_".into(),
            },
        };

        let outputs = convert_images_with_progress(&params, |_, _| {}).unwrap();
        assert_eq!(outputs.len(), 1);
        let image = image::open(&outputs[0]).unwrap();
        assert_eq!(image.dimensions(), (60, 40));
    }

    #[test]
    fn full_png_pipeline() {
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("source.jpg");
        let output_dir = dir.path().join("out");
        sample_image(&source, image::ImageFormat::Jpeg);

        let params = ConvertParams {
            files: vec![source.to_string_lossy().to_string()],
            output_dir: output_dir.to_string_lossy().to_string(),
            format: "png".into(),
            quality: 80,
            pct_w: 100,
            pct_h: 100,
            resize_w: 120,
            resize_h: 80,
            lock_aspect: true,
            algorithm: "lanczos3".into(),
            rename_pattern: RenamePatternDto::Keep,
        };

        let outputs = convert_images_with_progress(&params, |_, _| {}).unwrap();
        assert_eq!(outputs.len(), 1);
        assert!(outputs[0].ends_with(".png"));
    }

    #[test]
    fn batch_of_five_files() {
        let dir = tempfile::tempdir().unwrap();
        let output_dir = dir.path().join("out");
        let mut files = Vec::new();
        for index in 0..5 {
            let source = dir.path().join(format!("sample-{index}.png"));
            sample_image(&source, image::ImageFormat::Png);
            files.push(source.to_string_lossy().to_string());
        }

        let params = ConvertParams {
            files,
            output_dir: output_dir.to_string_lossy().to_string(),
            format: "jpeg".into(),
            quality: 75,
            pct_w: 83,
            pct_h: 75,
            resize_w: 100,
            resize_h: 60,
            lock_aspect: false,
            algorithm: "bilinear".into(),
            rename_pattern: RenamePatternDto::Sequential {
                base: "export_".into(),
                start: 1,
            },
        };

        let outputs = convert_images_with_progress(&params, |_, _| {}).unwrap();
        assert_eq!(outputs.len(), 5);
        assert!(outputs.iter().any(|path| path.ends_with("export_001.jpg")));
        assert!(outputs.iter().all(|path| Path::new(path).exists()));
    }

    #[test]
    fn mixed_orientation_batch_preserves_each_aspect_when_locked() {
        let dir = tempfile::tempdir().unwrap();
        let output_dir = dir.path().join("out");

        let portrait = dir.path().join("portrait.png");
        DynamicImage::ImageRgba8(RgbaImage::new(80, 120))
            .save_with_format(&portrait, image::ImageFormat::Png)
            .unwrap();

        let landscape = dir.path().join("landscape.png");
        DynamicImage::ImageRgba8(RgbaImage::new(120, 80))
            .save_with_format(&landscape, image::ImageFormat::Png)
            .unwrap();

        let params = ConvertParams {
            files: vec![
                portrait.to_string_lossy().to_string(),
                landscape.to_string_lossy().to_string(),
            ],
            output_dir: output_dir.to_string_lossy().to_string(),
            format: "jpeg".into(),
            quality: 80,
            pct_w: 100,
            pct_h: 100,
            resize_w: 80,
            resize_h: 120,
            lock_aspect: true,
            algorithm: "lanczos3".into(),
            rename_pattern: RenamePatternDto::Sequential {
                base: "mix_".into(),
                start: 1,
            },
        };

        let outputs = convert_images_with_progress(&params, |_, _| {}).unwrap();
        let first = image::open(&outputs[0]).unwrap();
        let second = image::open(&outputs[1]).unwrap();
        assert_eq!(first.dimensions(), (80, 120));
        assert_eq!(second.dimensions(), (120, 80));
    }

    #[test]
    #[ignore = "save dialog requires a windowed main-thread environment on macOS"]
    fn cancel_save_dialog() {
        let result = open_save_dialog("/definitely/missing/path".into());
        if let Some(path) = result {
            assert!(!path.is_empty());
        }
    }
}
