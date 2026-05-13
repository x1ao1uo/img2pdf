use std::collections::HashMap;
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

use walkdir::WalkDir;

#[derive(Debug, Clone)]
pub struct ImageGroup {
    pub name: String,
    #[allow(dead_code)]
    pub dir: PathBuf,
    pub files: Vec<PathBuf>,
}

pub fn discover_groups(root: &Path, recursive: bool) -> Result<Vec<ImageGroup>, Box<dyn Error>> {
    let clean_root = clean_root_dir(root)?;
    let files = discover_image_files(&clean_root, recursive)?;
    Ok(build_groups(&clean_root, files))
}

fn is_supported_image(path: &Path) -> bool {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();
    matches!(ext.as_str(), "jpg" | "jpeg" | "png")
}

fn clean_root_dir(root: &Path) -> Result<PathBuf, Box<dyn Error>> {
    let clean_root = fs::canonicalize(root)?;
    if !clean_root.is_dir() {
        return Err(format!("input path is not a directory: {}", clean_root.display()).into());
    }
    Ok(clean_root)
}

fn discover_image_files(root: &Path, recursive: bool) -> Result<Vec<PathBuf>, Box<dyn Error>> {
    if recursive {
        walk_image_files(root)
    } else {
        read_root_image_files(root)
    }
}

fn walk_image_files(root: &Path) -> Result<Vec<PathBuf>, Box<dyn Error>> {
    let mut files = Vec::new();
    for entry in WalkDir::new(root) {
        let entry = entry?;
        let path = entry.path();
        if path.is_file() && is_supported_image(path) {
            files.push(path.to_path_buf());
        }
    }
    files.sort();
    Ok(files)
}

fn read_root_image_files(root: &Path) -> Result<Vec<PathBuf>, Box<dyn Error>> {
    let mut files = Vec::new();
    for entry in fs::read_dir(root)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_file() && is_supported_image(&path) {
            files.push(path);
        }
    }
    files.sort();
    Ok(files)
}

fn build_groups(root: &Path, files: Vec<PathBuf>) -> Vec<ImageGroup> {
    let mut by_dir: HashMap<PathBuf, Vec<PathBuf>> = HashMap::new();
    for file in files {
        let dir = file.parent().unwrap_or(Path::new("")).to_path_buf();
        by_dir.entry(dir).or_default().push(file);
    }

    let mut dirs: Vec<PathBuf> = by_dir.keys().cloned().collect();
    dirs.sort();

    let mut used_names: HashMap<String, i32> = HashMap::new();
    let mut groups = Vec::new();

    for dir in dirs {
        let mut group_files = by_dir.remove(&dir).unwrap_or_default();
        group_files.sort();

        let name = unique_group_name(group_name(root, &dir), &mut used_names);
        groups.push(ImageGroup {
            name,
            dir: dir.clone(),
            files: group_files,
        });
    }

    groups
}

fn group_name(root: &Path, dir: &Path) -> String {
    let rel = dir
        .strip_prefix(root)
        .ok()
        .and_then(|p| p.to_str())
        .unwrap_or("");

    let name = if rel.is_empty() || rel == "." {
        dir.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("images")
            .to_string()
    } else {
        rel.replace(['/', '\\'], "_")
    };

    safe_name(&name)
}

fn safe_name(name: &str) -> String {
    let cleaned: String = name
        .trim()
        .chars()
        .map(replace_unsafe_rune)
        .filter(|&c| c != '\0')
        .collect();

    let trimmed: String = cleaned
        .trim_matches(|c: char| c == ' ' || c == '.' || c == '_')
        .to_string();

    if trimmed.is_empty() {
        "images".to_string()
    } else {
        trimmed
    }
}

fn unique_group_name(name: String, used: &mut HashMap<String, i32>) -> String {
    let count = used.entry(name.clone()).or_insert(0);
    *count += 1;
    if *count == 1 {
        name
    } else {
        format!("{}_{}", name, *count)
    }
}

fn replace_unsafe_rune(r: char) -> char {
    match r {
        '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
        _ if r.is_control() => '\0',
        _ => r,
    }
}
