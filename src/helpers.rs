
use std::env;
use csv::ReaderBuilder;
use std::fs::{self, File};
use std::io::{self, Read};
use std::error::Error;
use std::path::{Path, PathBuf};
use std::collections::HashMap;
use prettytable::{Table, Row, Cell};
use indicatif::{ProgressBar, ProgressStyle};

    pub fn calculate_total_size(path: &Path) -> io::Result<u64> {
        let mut total_size = 0;
        let entries = fs::read_dir(path)?;

        for entry in entries {
            match entry {
                Ok(entry) => {
                    let metadata = match entry.metadata() {
                        Ok(metadata) => metadata,
                        Err(e) if e.kind() == io::ErrorKind::PermissionDenied => continue,
                        Err(e) => return Err(e),
                    };

                    if metadata.is_dir() {
                        // Recursively calculate the size of subdirectories
                        total_size += calculate_total_size(&entry.path())?;
                    } else {
                        // Add file size to total
                        total_size += metadata.len();
                    }
                }
                Err(e) if e.kind() == io::ErrorKind::PermissionDenied => continue,
                Err(e) => return Err(e),
            }
        }

        Ok(total_size)
    }

    pub fn get_first_layer_full_sizes(start_path: &Path) -> io::Result<HashMap<PathBuf, u64>> {
        let mut folder_sizes = HashMap::new();
        // Get entries in the root directory
        let entries = fs::read_dir(start_path)?;
        let total_entries = entries.count();

        // Initialize the progress bar
        let pb = ProgressBar::new(total_entries as u64);
        pb.set_style(
            ProgressStyle::default_bar()
                .template(
                    "{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} {msg}",
                )
                .progress_chars("##-"),
        );
        pb.enable_steady_tick(100);
        // Get entries in the root directory
        let entries = fs::read_dir(start_path)?;

        for entry in entries {
            match entry {
                Ok(entry) => {
                    let metadata = match entry.metadata() {
                        Ok(metadata) => metadata,
                        Err(e) if e.kind() == io::ErrorKind::PermissionDenied => continue,
                        Err(e) => return Err(e),
                    };

                    if metadata.is_dir() {
                        // Calculate the full size of this first-level subdirectory (including all nested contents)
                        let subdir_size = calculate_total_size(&entry.path())?;
                        folder_sizes.insert(entry.path(), subdir_size);
                    } else {
                        // Add individual files directly in the root directory with their size
                        folder_sizes.insert(entry.path(), metadata.len());
                    }
                    pb.inc(1);
                }
                Err(e) if e.kind() == io::ErrorKind::PermissionDenied => continue,
                Err(e) => return Err(e),
            }
        }
        pb.finish_and_clear();
        Ok(folder_sizes)
    }

    pub fn format_size(size: u64) -> String {
        const KB: u64 = 1_024;
        const MB: u64 = KB * 1_024;
        const GB: u64 = MB * 1_024;

        if size >= GB {
            format!("{:.2} GB", size as f64 / GB as f64)
        } else if size >= MB {
            format!("{:.2} MB", size as f64 / MB as f64)
        } else if size >= KB {
            format!("{:.2} KB", size as f64 / KB as f64)
        } else {
            format!("{} B", size)
        }
    }

    pub fn get_file_type(path: &Path) -> String {
        let extension = path.extension().and_then(|s| s.to_str()).unwrap_or("");
        match extension {
            "rs" => "Rust".to_uppercase(),
            "py" => "Python".to_uppercase(),
            "js" => "JavaScript".to_uppercase(),
            "ts" => "TypeScript".to_uppercase(),
            "cpp" => "C++".to_uppercase(),
            "c" => "C".to_uppercase(),
            "cs" => "C#".to_uppercase(),
            "java" => "Java".to_uppercase(),
            "rb" => "Ruby".to_uppercase(),
            "php" => "PHP".to_uppercase(),
            "html" => "HTML".to_uppercase(),
            "css" => "CSS".to_uppercase(),
            "sh" => "Shell".to_uppercase(),
            "go" => "Go".to_uppercase(),
            "zig" => "Zig".to_uppercase(),
            "json" => "JSON".to_uppercase(),
            "xml" => "XML".to_uppercase(),
            "toml" => "Cargo".to_uppercase(),
            "yaml" => "Yaml".to_uppercase(),
            "exe" => "Application".to_uppercase(),
            "pem" => "Key".to_uppercase(),
            "dll" => "DLL".to_uppercase(),
            "md" => "Markdown".to_uppercase(),
            "mp3" => "Mp3 Audio".to_uppercase(),
            "m4a" => "M4a Audio".to_uppercase(),
            "mp4" => "MP4 Video".to_uppercase(),
            "mkv" => "MKV Video".to_uppercase(),
            "jpg" => "JPEG Image".to_uppercase(),
            "png" => "PNG Image".to_uppercase(),
            "pdf" => "PDF".to_uppercase(),
            "txt" => "Text".to_uppercase(),
            "" => "Dot File".to_uppercase(),
            _ => extension.to_uppercase(),
        }
    }

    pub fn format_root_name(path: &Path) -> String {
        let normalized_path = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
        match normalized_path.file_name() {
            Some(name) => name.to_string_lossy().to_string(),
            None => "UNKNOWN".to_string(),
        }
    }

