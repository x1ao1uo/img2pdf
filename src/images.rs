use std::collections::HashMap;
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

const IMAGES_PER_PDF: usize = 4;

#[derive(Debug, Clone)]
pub struct ImageGroup {
    pub name: String,
    pub files: Vec<PathBuf>,
}

pub fn discover_groups(root: &Path) -> Result<Vec<ImageGroup>, Box<dyn Error>> {
    let clean_root = clean_root_dir(root)?;
    let dirs = discover_group_dirs(&clean_root)?;
    build_groups(dirs)
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

fn discover_group_dirs(root: &Path) -> Result<Vec<PathBuf>, Box<dyn Error>> {
    let mut dirs = Vec::new();
    for entry in fs::read_dir(root)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            dirs.push(path);
        }
    }
    dirs.sort();
    Ok(dirs)
}

fn read_dir_image_files(dir: &Path) -> Result<Vec<PathBuf>, Box<dyn Error>> {
    let mut files = Vec::new();
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_file() && is_supported_image(&path) {
            files.push(path);
        }
    }
    files.sort();
    Ok(files)
}

fn build_groups(dirs: Vec<PathBuf>) -> Result<Vec<ImageGroup>, Box<dyn Error>> {
    let mut used_names: HashMap<String, i32> = HashMap::new();
    let mut groups = Vec::new();

    for dir in dirs {
        let group_files = read_dir_image_files(&dir)?;
        if group_files.is_empty() {
            continue;
        }
        let base_name = group_name(&dir);

        for (chunk_index, chunk) in group_files.chunks(IMAGES_PER_PDF).enumerate() {
            let name = unique_group_name(pdf_group_name(&base_name, chunk_index), &mut used_names);
            groups.push(ImageGroup {
                name,
                files: chunk.to_vec(),
            });
        }
    }

    Ok(groups)
}

fn group_name(dir: &Path) -> String {
    let name = dir
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("images");
    safe_name(name)
}

fn pdf_group_name(base_name: &str, chunk_index: usize) -> String {
    if chunk_index == 0 {
        base_name.to_string()
    } else {
        format!("{}{}", base_name, chunk_index + 1)
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    struct TempDir {
        path: PathBuf,
    }

    impl TempDir {
        fn new(name: &str) -> Self {
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system time should be after Unix epoch")
                .as_nanos();
            let path =
                std::env::temp_dir().join(format!("img2pdf-{name}-{}-{now}", std::process::id()));
            fs::create_dir_all(&path).expect("temp dir should be created");
            Self { path }
        }

        fn path(&self) -> &Path {
            &self.path
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.path);
        }
    }

    #[test]
    fn discover_groups_splits_sorted_images_into_four_image_pdf_groups() {
        let temp = TempDir::new("chunked-groups");
        let group_dir = temp.path().join("group-a");
        fs::create_dir_all(&group_dir).expect("group dir should be created");
        for name in [
            "09.jpg", "01.jpg", "03.png", "02.jpeg", "04.jpg", "08.png", "05.jpg", "07.jpeg",
            "06.jpg",
        ] {
            fs::write(group_dir.join(name), []).expect("test image placeholder should be written");
        }

        let groups = discover_groups(temp.path()).expect("groups should be discovered");

        assert_eq!(groups.len(), 3);
        let names: Vec<_> = groups.iter().map(|group| group.name.as_str()).collect();
        assert_eq!(names, ["group-a", "group-a2", "group-a3"]);

        let chunk_file_names: Vec<Vec<_>> = groups
            .iter()
            .map(|group| {
                group
                    .files
                    .iter()
                    .map(|path| path.file_name().and_then(|name| name.to_str()).unwrap())
                    .collect()
            })
            .collect();
        assert_eq!(
            chunk_file_names,
            [
                vec!["01.jpg", "02.jpeg", "03.png", "04.jpg"],
                vec!["05.jpg", "06.jpg", "07.jpeg", "08.png"],
                vec!["09.jpg"],
            ]
        );
    }

    #[test]
    fn discover_groups_uses_directory_basename_for_pdf_names() {
        let temp = TempDir::new("basename");
        for dir in ["a:b", "a?b"] {
            let dir_path = temp.path().join(dir);
            fs::create_dir_all(&dir_path).expect("image dir should be created");
            fs::write(dir_path.join("01.jpg"), [])
                .expect("test image placeholder should be written");
        }

        let groups = discover_groups(temp.path()).expect("groups should be discovered");
        let names: Vec<_> = groups.iter().map(|group| group.name.as_str()).collect();

        assert_eq!(names, ["a_b", "a_b_2"]);
    }

    #[test]
    fn discover_groups_ignores_root_images_and_nested_directories() {
        let temp = TempDir::new("direct-children-only");
        fs::write(temp.path().join("root.jpg"), [])
            .expect("root image placeholder should be written");

        let group_dir = temp.path().join("group-a");
        fs::create_dir_all(group_dir.join("nested")).expect("nested dir should be created");
        fs::write(group_dir.join("01.jpg"), []).expect("group image placeholder should be written");
        fs::write(group_dir.join("nested").join("02.jpg"), [])
            .expect("nested image placeholder should be written");

        let groups = discover_groups(temp.path()).expect("groups should be discovered");

        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].name, "group-a");
        let file_names: Vec<_> = groups[0]
            .files
            .iter()
            .map(|path| path.file_name().and_then(|name| name.to_str()).unwrap())
            .collect();
        assert_eq!(file_names, ["01.jpg"]);
    }
}
