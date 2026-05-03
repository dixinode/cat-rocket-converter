use chrono::Local;
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RenamePattern {
    /// Prefix filenames with a datetime stamp and a zero-padded batch index.
    DateTimePrefix { prefix: String },
    /// Keep the original base name and append a suffix before the output extension.
    OriginalSuffix { suffix: String },
    /// Replace the base name with a sequential counter.
    Sequential { base: String, start: u32 },
    /// Preserve the original base name and only replace the extension.
    KeepOriginal,
}

/// Apply a rename pattern to a list of source file paths and return filenames without directories.
pub fn apply_rename_pattern(
    files: &[String],
    pattern: &RenamePattern,
    output_ext: &str,
) -> Vec<String> {
    let timestamp = Local::now().format("%Y%m%d_%H%M%S").to_string();

    files
        .iter()
        .enumerate()
        .map(|(index, path)| {
            let original_name = Path::new(path)
                .file_stem()
                .and_then(|value| value.to_str())
                .unwrap_or("image");

            match pattern {
                RenamePattern::DateTimePrefix { prefix } => {
                    format!("{}{timestamp}_{:03}.{output_ext}", prefix, index + 1)
                }
                RenamePattern::OriginalSuffix { suffix } => {
                    format!("{original_name}{suffix}.{output_ext}")
                }
                RenamePattern::Sequential { base, start } => {
                    format!("{base}{:03}.{output_ext}", start + index as u32)
                }
                RenamePattern::KeepOriginal => format!("{original_name}.{output_ext}"),
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use regex::Regex;

    fn files() -> Vec<String> {
        vec![
            "/tmp/photo.jpg".into(),
            "/tmp/second.png".into(),
            "/tmp/third.heic".into(),
            "/tmp/fourth.jpg".into(),
            "/tmp/fifth.jpg".into(),
        ]
    }

    #[test]
    fn test_datetime_pattern() {
        let names = apply_rename_pattern(
            &[files()[0].clone()],
            &RenamePattern::DateTimePrefix {
                prefix: "IMG_".into(),
            },
            "jpg",
        );

        let regex = Regex::new(r"IMG_\d{8}_\d{6}_\d{3}\.jpg").unwrap();
        assert!(regex.is_match(&names[0]));
    }

    #[test]
    fn test_datetime_batch() {
        let names = apply_rename_pattern(
            &files(),
            &RenamePattern::DateTimePrefix {
                prefix: "IMG_".into(),
            },
            "jpg",
        );

        let set: std::collections::HashSet<_> = names.iter().collect();
        assert_eq!(set.len(), 5);
    }

    #[test]
    fn test_suffix_pattern() {
        let names = apply_rename_pattern(
            &[files()[0].clone()],
            &RenamePattern::OriginalSuffix {
                suffix: "_compressed".into(),
            },
            "jpg",
        );

        assert_eq!(names[0], "photo_compressed.jpg");
    }

    #[test]
    fn test_sequential_pattern() {
        let names = apply_rename_pattern(
            &files()[0..3],
            &RenamePattern::Sequential {
                base: "export_".into(),
                start: 1,
            },
            "jpg",
        );

        assert_eq!(
            names,
            vec!["export_001.jpg", "export_002.jpg", "export_003.jpg"]
        );
    }

    #[test]
    fn test_keep_original() {
        let input = files();
        let names = apply_rename_pattern(&input, &RenamePattern::KeepOriginal, "jpg");
        assert_eq!(names[0], "photo.jpg");
        assert_eq!(names[1], "second.jpg");
    }

    #[test]
    fn test_extension_replaced() {
        let names =
            apply_rename_pattern(&[files()[1].clone()], &RenamePattern::KeepOriginal, "jpg");
        assert!(names[0].ends_with(".jpg"));
    }
}
