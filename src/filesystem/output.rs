use std::path::{Path, PathBuf};
use crate::models::{OutputFormat, PhotoSource};

/// Resolves a safe, collision-resistant destination path for an extracted photo.
pub fn resolve_output_path(
    base_output_dir: &Path,
    source: &PhotoSource,
    format: OutputFormat,
    preserve_filenames: bool,
    preserve_folder_structure: bool,
    overwrite_existing: bool,
    batch_index: usize,
    base_input_dir: Option<&Path>,
) -> PathBuf {
    let ext = format.extension();

    // Determine target file name stem
    let stem = if preserve_filenames {
        match source {
            PhotoSource::LocalFile(p) => {
                let file_name = p.file_name().and_then(|n| n.to_str()).unwrap_or("photo");
                // Strip common extensions or keep raw stem
                if let Some(pos) = file_name.rfind('.') {
                    if pos > 0 {
                        &file_name[..pos]
                    } else {
                        file_name
                    }
                } else {
                    file_name
                }
            }
            PhotoSource::ZipEntry { entry_name, .. } => {
                let clean = entry_name.split('/').last().unwrap_or(entry_name.as_str());
                if let Some(pos) = clean.rfind('.') {
                    if pos > 0 {
                        &clean[..pos]
                    } else {
                        clean
                    }
                } else {
                    clean
                }
            }
        }
    } else {
        // Formatted photo name
        ""
    };

    let base_name = if stem.is_empty() {
        format!("PHOTO_{:04}", batch_index + 1)
    } else {
        stem.to_string()
    };

    // Calculate directory hierarchy if preserving structure
    let target_dir = if preserve_folder_structure {
        if let (Some(base_in), PhotoSource::LocalFile(source_path)) = (base_input_dir, source) {
            if let Ok(rel) = source_path.strip_prefix(base_in) {
                if let Some(parent) = rel.parent() {
                    base_output_dir.join(parent)
                } else {
                    base_output_dir.to_path_buf()
                }
            } else {
                base_output_dir.to_path_buf()
            }
        } else {
            base_output_dir.to_path_buf()
        }
    } else {
        base_output_dir.to_path_buf()
    };

    let candidate_filename = format!("{base_name}.{ext}");
    let candidate_path = target_dir.join(&candidate_filename);

    if overwrite_existing || !candidate_path.exists() {
        return candidate_path;
    }

    // Handle collision by appending (1), (2), etc.
    let mut counter = 1;
    loop {
        let collision_name = format!("{base_name} ({counter}).{ext}");
        let test_path = target_dir.join(&collision_name);
        if !test_path.exists() {
            return test_path;
        }
        counter += 1;
    }
}

/// Opens the specified folder in native Windows Explorer (or default system file manager).
pub fn open_folder_in_native_file_manager(path: &Path) {
    #[cfg(windows)]
    {
        let target = if path.is_file() {
            path.parent().unwrap_or(path)
        } else {
            path
        };
        let _ = std::process::Command::new("explorer")
            .arg(target)
            .spawn();
    }
    #[cfg(target_os = "macos")]
    {
        let _ = std::process::Command::new("open").arg(path).spawn();
    }
    #[cfg(target_os = "linux")]
    {
        let _ = std::process::Command::new("xdg-open").arg(path).spawn();
    }
}
