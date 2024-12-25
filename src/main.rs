use std::env;
use csv::ReaderBuilder;
use std::fs::{self, File};
use std::io::{self, Read};
use std::error::Error;
use std::path::{Path, PathBuf};
use std::collections::HashMap;
use prettytable::{Table, Row, Cell};
use indicatif::{ProgressBar, ProgressStyle};
use rayon::prelude::*;
use std::fs::DirEntry;

fn calculate_total_size(path: &Path) -> io::Result<u64> {
    let mut total_size = 0;
    let entries = match fs::read_dir(path) {
        Ok(entries) => entries,
        Err(e) if e.kind() == io::ErrorKind::PermissionDenied => {
            eprintln!("Skipping directory due to permission error: {}", path.display());
            return Ok(0);
        }
        Err(e) => return Err(e),
    };

    for entry in entries {
        match entry {
            Ok(entry) => {
                let metadata = match entry.metadata() {
                    Ok(metadata) => metadata,
                    Err(e) if e.kind() == io::ErrorKind::PermissionDenied => {
                        eprintln!("Skipping file due to permission error: {}", entry.path().display());
                        continue;
                    }
                    Err(e) => return Err(e),
                };

                if metadata.is_dir() {
                    total_size += calculate_total_size(&entry.path())?;
                } else {
                    total_size += metadata.len();
                }
            }
            Err(e) if e.kind() == io::ErrorKind::PermissionDenied => {
                eprintln!("Skipping entry due to permission error: {}", path.display());
                continue;
            }
            Err(e) => return Err(e),
        }
    }

    Ok(total_size)
}
// updated first layer full size 
fn get_first_layer_full_sizes(start_path: &Path) -> io::Result<HashMap<PathBuf, u64>> {
    let entries: Vec<DirEntry> = fs::read_dir(start_path)?
        .filter_map(|entry| entry.ok())
        .collect();

    let pb = ProgressBar::new(entries.len() as u64);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} {msg}")
            .progress_chars("##-"),
    );
    pb.enable_steady_tick(100);

    let folder_sizes: HashMap<PathBuf, u64> = entries
        .into_par_iter() // Converts Vec<DirEntry> to a Rayon parallel iterator
        .filter_map(|entry| {
            let metadata = entry.metadata().ok()?;
            if metadata.is_dir() {
                let size = calculate_total_size(&entry.path()).ok()?;
                Some((entry.path(), size))
            } else {
                Some((entry.path(), metadata.len()))
            }
        })
        .inspect(|_| pb.inc(1))
        .collect();

    pb.finish_and_clear();
    Ok(folder_sizes)
}

fn format_size(size: u64) -> String {
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


fn get_file_type(path: &Path) -> String {
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


fn format_root_name(path: &Path) -> String {
    let normalized_path = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    match normalized_path.file_name() {
        Some(name) => name.to_string_lossy().to_string(),
        None => "UNKNOWN".to_string(),
    }
}

fn print_csv_table(file_path: &Path) -> Result<(), Box<dyn Error>> {
    let file = File::open(file_path)?;
    let mut rdr = ReaderBuilder::new().has_headers(true).from_reader(file);

    // Create a table for pretty printing
    let mut table = Table::new();

    // Get headers and add them to the table
    let headers = rdr.headers()?;
    let header_row: Vec<Cell> = headers.iter().map(|h| Cell::new(h)).collect();
    table.add_row(Row::new(header_row));

    // Read and add the top 10 rows to the table
    for result in rdr.records().take(10) {
        let record = result?;
        let cells: Vec<Cell> = record.iter().map(|field| Cell::new(field)).collect();
        table.add_row(Row::new(cells));
    }

    // Print the table
    table.printstd();

    Ok(())
}
fn print_file_content(path: &Path) -> io::Result<()> {
    let mut file = fs::File::open(path)?;
    let mut content = String::new();
    file.read_to_string(&mut content)?;
    //let extension = path.extension().and_then(|s| s.to_str()).unwrap_or("");

    println!("{}", &content);

    Ok(())
}

fn main() -> io::Result<()> {
    // Get input or use the current directory 
    let args: Vec<String> = env::args().collect();
    let folder_path = if args.len() > 1 {
        Path::new(&args[1]) 
    } else {
        Path::new(".") 
    };

  if folder_path.is_file() {
        // If input is a file, print its content
        if folder_path.extension().and_then(|e| e.to_str()) == Some("csv") {

            if let Err(e) = print_csv_table(&folder_path) {
            eprintln!("Error reading file: {}", e);
            }
        } else {
              if let Err(e) = print_file_content(&folder_path) {
            eprintln!("Error reading file: {}", e);
            }
        }
        return Ok(());
    } 

    let folder_sizes = match get_first_layer_full_sizes(folder_path) {
        Ok(folder_sizes) => folder_sizes,
        Err(e) => {
            eprintln!("Error calculating folder size: {}", e);
            return Err(e);
        }
    };

    // Calculate and display the total size of the root folder and its contents
    let total_size: u64 = folder_sizes.values().sum();
    println!("\nDir: 📂{}\tSize: {}\n", format_root_name(folder_path),format_size(total_size));

    // Separate into folders and files
    let mut folders: Vec<_> = folder_sizes.iter()
        .filter(|(path, _)| path.is_dir())
        .collect();
    let mut files: Vec<_> = folder_sizes.iter()
        .filter(|(path, _)| path.is_file())
        .collect();

    // Sort folders and files by size in descending order
    folders.sort_by_key(|(_, size)| std::cmp::Reverse(*size));
    files.sort_by_key(|(_, size)| std::cmp::Reverse(*size));

    // Create and configure the table
    let mut table = Table::new();
    table.add_row(Row::new(vec![
        Cell::new("#"),
        Cell::new("Path"),
        Cell::new("Type"),        
        Cell::new("Size"),
        
    ]));

    // Function to remove the current directory prefix
    let remove_prefix = |path: &Path| {
        path.strip_prefix(".\\")
            .or_else(|_| path.strip_prefix("./"))
            .unwrap_or(path)
            .display().to_string()
    };

    // Add folders to the table
    for (folder, size) in folders {
        table.add_row(Row::new(vec![
            Cell::new("📁"),
            Cell::new(&remove_prefix(folder)),
            Cell::new("-"),
            Cell::new(&format_size(*size)),

        ]));
    }

    // Add files to the table
    for (file, size) in files {
        table.add_row(Row::new(vec![
            Cell::new("📄"),
            Cell::new(&remove_prefix(file)),
            Cell::new(&get_file_type(file)),
            Cell::new(&format_size(*size)),
            
        ]));
    }

    // Print the table
    table.printstd();
    println!("");

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::{self, File};
    use std::io::Write;
    use std::path::Path;
    use std::env;

    #[test]
    fn test_format_size() {
        assert_eq!(format_size(0), "0 B");
        assert_eq!(format_size(1), "1 B");
        assert_eq!(format_size(1023), "1023 B");
        assert_eq!(format_size(1024), "1.00 KB");
        assert_eq!(format_size(1024 * 1024), "1.00 MB");
        assert_eq!(format_size(1024 * 1024 * 1024), "1.00 GB");
    }

    #[test]
    fn test_get_file_type() {
        assert_eq!(get_file_type(Path::new("test.rs")), "RUST");
        assert_eq!(get_file_type(Path::new("test.py")), "PYTHON");
        assert_eq!(get_file_type(Path::new("test.unknown")), "UNKNOWN");
        assert_eq!(get_file_type(Path::new("file")), "DOT FILE"); // No extension
    }

    #[test]
    fn test_format_root_name() {
        assert_eq!(format_root_name(Path::new("/path/to/file.txt")), "file.txt");
        assert_eq!(format_root_name(Path::new("/path/to/directory")), "directory");
        assert_eq!(format_root_name(Path::new("/path/to/unknown")), "unknown");
    }

    #[test]
    fn test_calculate_total_size() {
        let temp_dir = env::temp_dir().join("test_calculate_total_size");

        // Create temp directory and files
        fs::create_dir(&temp_dir).unwrap();
        let file_path = temp_dir.join("file1.txt");
        let mut file = File::create(&file_path).unwrap();
        file.write_all(b"Hello, world!").unwrap(); // 13 bytes

        let sub_dir = temp_dir.join("sub_dir");
        fs::create_dir(&sub_dir).unwrap();
        let sub_file_path = sub_dir.join("file2.txt");
        let mut sub_file = File::create(&sub_file_path).unwrap();
        sub_file.write_all(b"Hello, sub world!").unwrap(); // 17 bytes

        // Calculate the size of the temp directory
        let size = calculate_total_size(&temp_dir).unwrap();
        assert_eq!(size, 13 + 17); // 13 bytes + 17 bytes

        // Clean up
        fs::remove_dir_all(temp_dir).unwrap();
    }

    #[test]
    fn test_get_first_layer_full_sizes() {
        let temp_dir = env::temp_dir().join("test_get_first_layer_full_sizes");

        // Create temp directory and files
        fs::create_dir(&temp_dir).unwrap();
        let file_path = temp_dir.join("file1.txt");
        let mut file = File::create(&file_path).unwrap();
        file.write_all(b"Hello, world!").unwrap(); // 13 bytes

        let sub_dir = temp_dir.join("sub_dir");
        fs::create_dir(&sub_dir).unwrap();
        let sub_file_path = sub_dir.join("file2.txt");
        let mut sub_file = File::create(&sub_file_path).unwrap();
        sub_file.write_all(b"Hello, sub world!").unwrap(); // 17 bytes

        // Get the first layer folder sizes
        let folder_sizes = get_first_layer_full_sizes(&temp_dir).unwrap();

        assert_eq!(folder_sizes.get(&file_path).unwrap(), &13); // File size
        assert_eq!(folder_sizes.get(&sub_dir).unwrap(), &(17)); // Size of sub_dir's content

        // Clean up
        fs::remove_dir_all(temp_dir).unwrap();
    }
}

