use anyhow::{Context, Result};
use clap::{Parser, ValueEnum};
use indicatif::{ProgressBar, ProgressStyle};
use prettytable::{Cell, Row, Table};
use rayon::prelude::*;
use serde::Serialize;
use std::env;
use std::fs::{self, File};
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use walkdir::{DirEntry, WalkDir};

#[derive(Debug, Clone, ValueEnum)]
enum SortBy {
    Size,
    Name,
    Type,
}

#[derive(Parser, Debug)]
#[command(name = "rdu", about = "Fast folder size + preview tool")]
struct Args {
    /// Path to scan (file or directory)
    #[arg(default_value = ".")]
    path: PathBuf,

    /// Show only top N entries
    #[arg(long, default_value_t = 50)]
    top: usize,

    /// Sorting strategy
    #[arg(long, value_enum, default_value_t = SortBy::Size)]
    sort: SortBy,

    /// Maximum depth for scanning (0 = first layer only, 1 = full subtree sizes for first layer items)
    #[arg(long, default_value_t = 1)]
    depth: usize,

    /// Follow symlinks (can be dangerous if cycles exist)
    #[arg(long, default_value_t = false)]
    follow_symlinks: bool,

    /// Skip common heavy folders (.git, node_modules, target, dist, build)
    #[arg(long, default_value_t = true)]
    smart_ignore: bool,

    /// Output JSON instead of table
    #[arg(long, default_value_t = false)]
    json: bool,

    /// Print extension summary (top 20)
    #[arg(long, default_value_t = false)]
    ext: bool,

    /// Minimum size filter in bytes
    #[arg(long)]
    min_size: Option<u64>,

    /// Print only first N lines when previewing a file
    #[arg(long, default_value_t = 400)]
    head: usize,
}

#[derive(Debug, Clone, Serialize)]
struct EntryInfo {
    path: PathBuf,
    is_dir: bool,
    size: u64,
    file_type: String,
}

fn format_root_name(path: &Path) -> String {
    // Handle roots like "C:\" on Windows
    if path.parent().is_none() {
        return path.display().to_string();
    }
    path.file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| path.display().to_string())
}

fn is_ignored(entry: &DirEntry, smart_ignore: bool) -> bool {
    if !smart_ignore {
        return false;
    }
    let name = entry.file_name().to_string_lossy().to_lowercase();
    matches!(
        name.as_str(),
        ".git" | "node_modules" | "target" | "dist" | "build" | ".idea" | ".vscode"
    )
}

fn calculate_tree_size(root: &Path, follow_symlinks: bool, smart_ignore: bool) -> u64 {
    WalkDir::new(root)
        .follow_links(follow_symlinks)
        .into_iter()
        .filter_entry(|e| !is_ignored(e, smart_ignore))
        .filter_map(|e| e.ok())
        .filter_map(|e| e.metadata().ok().map(|m| (e, m)))
        .filter(|(_, m)| m.is_file())
        .map(|(_, m)| m.len())
        .sum()
}

fn get_file_type(path: &std::path::Path) -> String {
    // Treat dotfiles like ".env" as DOT FILE
    if let Some(name) = path.file_name().and_then(|s| s.to_str()) {
        if name.starts_with('.') {
            return "DOT FILE".to_string();
        }
    }

    let  ext = path
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_lowercase();

    match ext.as_str() {
        "rs" => "RUST".to_string(),
        "py" => "PYTHON".to_string(),
        "js" => "JAVASCRIPT".to_string(),
        "ts" => "TYPESCRIPT".to_string(),
        "cpp" => "C++".to_string(),
        "c" => "C".to_string(),
        "cs" => "C#".to_string(),
        "rb" => "RUBY".to_string(),
        "sh" => "SHELL".to_string(),
        "toml" => "CARGO".to_string(),
        "exe" => "APPLICATION".to_string(),
        "pem" => "KEY".to_string(),
        "md" => "MARKDOWN".to_string(),
        "mp3" => "MP3 AUDIO".to_string(),
        "m4a" => "M4A AUDIO".to_string(),
        "mp4" => "MP4 VIDEO".to_string(),
        "mkv" => "MKV VIDEO".to_string(),
        "jpg" | "jpeg" => "JPEG IMAGE".to_string(),
        "png" => "PNG IMAGE".to_string(),
        "pdf" => "PDF".to_string(),
        "txt" => "PLAIN TEXT".to_string(),
        "" => "NO EXT".to_string(),
        _=> ext.to_uppercase(), // <-- return the String, don't borrow it
    }
}

fn collect_first_layer_entries(start: &Path, args: &Args) -> Result<Vec<EntryInfo>> {
    let read_dir = fs::read_dir(start)
        .with_context(|| format!("Failed to read directory {}", start.display()))?;

    let entries: Vec<fs::DirEntry> = read_dir.filter_map(|e| e.ok()).collect();

    let pb = ProgressBar::new(entries.len() as u64);
    pb.set_style(
        ProgressStyle::with_template(
            "{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} {msg}",
        )
        .unwrap()
        .progress_chars("##-"),
    );
    pb.enable_steady_tick(std::time::Duration::from_millis(100));
    let pb = std::sync::Arc::new(pb);

    let results: Vec<EntryInfo> = entries
        .into_par_iter()
        .filter_map(|entry| {
            let path = entry.path();
            let md = entry.metadata().ok()?;

            // ignore at first layer too
            if args.smart_ignore {
                if let Some(name) = path.file_name().and_then(|s| s.to_str()) {
                    let n = name.to_lowercase();
                    if matches!(
                        n.as_str(),
                        ".git" | "node_modules" | "target" | "dist" | "build" | ".idea" | ".vscode"
                    ) {
                        pb.inc(1);
                        return None;
                    }
                }
            }

            let is_dir = md.is_dir();
            let size = if is_dir {
                if args.depth == 0 {
                    0
                } else {
                    calculate_tree_size(&path, args.follow_symlinks, args.smart_ignore)
                }
            } else {
                md.len()
            };

            pb.inc(1);

            Some(EntryInfo {
                file_type: if is_dir { "-".into() } else { get_file_type(&path) },
                path,
                is_dir,
                size,
            })
        })
        .collect();

    pb.finish_and_clear();
    Ok(results)
}

fn apply_filters(mut v: Vec<EntryInfo>, args: &Args) -> Vec<EntryInfo> {
    if let Some(min) = args.min_size {
        v.retain(|e| e.size >= min);
    }

    match args.sort {
        SortBy::Size => v.sort_by_key(|e| std::cmp::Reverse(e.size)),
        SortBy::Name => v.sort_by_key(|e| e.path.to_string_lossy().to_lowercase()),
        SortBy::Type => v.sort_by_key(|e| e.file_type.clone()),
    }

    v.truncate(args.top);
    v
}

fn print_table(entries: &[EntryInfo], root: &Path) {
    #[derive(Clone)]
    struct RowData {
        icon: String,
        path: String,
        typ: String,
        size: String,
    }

    // 1) Prepare owned row strings that will live long enough
    let rows: Vec<RowData> = entries
        .iter()
        .map(|e| {
            let icon = if e.is_dir { "📁" } else { "📄" }.to_string();

            let path = e
                .path
                .strip_prefix(root)
                .unwrap_or(&e.path)
                .display()
                .to_string();

            let typ = e.file_type.clone();

            let size = humansize::format_size(e.size, humansize::BINARY);

            RowData { icon, path, typ, size }
        })
        .collect();

    // 2) Build the table using references to the owned strings
    let mut table = Table::new();
    table.add_row(Row::new(vec![
        Cell::new("#"),
        Cell::new("Path"),
        Cell::new("Type"),
        Cell::new("Size"),
    ]));

    for r in &rows {
        table.add_row(Row::new(vec![
            Cell::new(&r.icon),
            Cell::new(&r.path),
            Cell::new(&r.typ),
            Cell::new(&r.size),
        ]));
    }

    table.printstd();
}

fn print_ext_summary(all: &[EntryInfo]) {
    use std::collections::BTreeMap;
    let mut map: BTreeMap<String, (u64, u64)> = BTreeMap::new(); // ext -> (count, bytes)

    for e in all.iter().filter(|e| !e.is_dir) {
        let ext = e
            .path
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_lowercase();
        let key = if ext.is_empty() { "(none)".into() } else { ext };
        let entry = map.entry(key).or_insert((0, 0));
        entry.0 += 1;
        entry.1 += e.size;
    }

    println!("\nBy extension (top 20):");
    for (ext, (count, bytes)) in map.iter().rev().take(20) {
        println!(
            "{:>10}  {:>6} files  {}",
            ext,
            count,
            humansize::format_size(*bytes, humansize::BINARY)
        );
    }
}

fn print_json(entries: &[EntryInfo]) -> Result<()> {
    println!("{}", serde_json::to_string_pretty(entries)?);
    Ok(())
}

fn print_file_head(path: &Path, head: usize) -> io::Result<()> {
    let mut file = fs::File::open(path)?;
    let mut content = String::new();
    file.read_to_string(&mut content)?;

    for (i, line) in content.lines().take(head).enumerate() {
        println!("{:>5} {}", i + 1, line);
    }
    Ok(())
}

fn main() -> Result<()> {
    // Keep your old behavior: allow passing path as first argument
    // But clap already handles it; this is just to keep compatibility.
    let _ = env::args();

    let args = Args::parse();
    let path = args.path.clone();

    if path.is_file() {
        // file preview (csv preview can be re-added; keeping simple here)
        print_file_head(&path, args.head)
            .with_context(|| format!("Failed to read file {}", path.display()))?;
        return Ok(());
    }

    let entries = collect_first_layer_entries(&path, &args)?;

    let total: u64 = entries.iter().map(|e| e.size).sum();
    println!(
        "\nDir: 📂{}\tSize: {}\n",
        format_root_name(&path),
        humansize::format_size(total, humansize::BINARY)
    );

    if args.ext {
        print_ext_summary(&entries);
    }

    let entries = apply_filters(entries, &args);

    if args.json {
        print_json(&entries)?;
    } else {
        print_table(&entries, &path);
    }

    Ok(())
}
