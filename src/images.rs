//! 图片模块，负责图片读取、排序和转换。
//!
//! 职责：
//! 1. 在指定目录下发现直接子目录作为「分组」；
//! 2. 在每个分组内按文件名排序收集支持格式的图片（jpg/jpeg/png）；
//! 3. 将每个分组按 `IMAGES_PER_PDF` 张切块，生成 `ImageGroup` 列表供 PDF 模块使用。
//!
//! 分块命名规则：
//! - 第一个分块直接使用目录名；
//! - 后续分块在目录名后追加 `2`、`3`…；
//! - 若同名分组在不同子目录重复出现，则在末尾追加 `_2`、`_3`…。

use std::collections::HashMap;
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

/// 每个 PDF 包含的最大图片数（与 PDF 模块的 A4 2x2 网格对应）。
const IMAGES_PER_PDF: usize = 4;

/// 一个待生成的 PDF 分组：包含分块名称与该分组内的图片路径列表。
#[derive(Debug, Clone)]
pub struct ImageGroup {
    /// PDF 文件名（不含扩展名）。
    pub name: String,
    /// 该分组内的图片绝对路径，按文件名排序。
    pub files: Vec<PathBuf>,
}

/// 在 `root` 目录下发现直接子目录并按 `IMAGES_PER_PDF` 张图片切块。
///
/// # 参数
/// - `root`: 扫描的根目录路径。
///
/// # 返回
/// - 成功时返回所有分组的 `Vec<ImageGroup>`；
/// - 路径不存在或不是目录时返回 I/O 错误。
///
/// # 行为
/// - 仅扫描 root 的直接子目录，root 自身和更深层目录中的图片会被忽略；
/// - 子目录中没有受支持格式图片的目录会被跳过；
/// - 同一目录的图片按文件名字典序排序后再切块。
pub fn discover_groups(root: &Path) -> Result<Vec<ImageGroup>, Box<dyn Error>> {
    // 1. 规范化输入路径，确保存在且为目录
    let clean_root = clean_root_dir(root)?;
    // 2. 收集直接子目录并按路径排序，保证输出顺序确定
    let dirs = discover_group_dirs(&clean_root)?;
    // 3. 对每个子目录读取图片并切块
    build_groups(dirs)
}

/// 判断 `path` 是否为受支持的图片格式（按扩展名）。
fn is_supported_image(path: &Path) -> bool {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();
    matches!(ext.as_str(), "jpg" | "jpeg" | "png")
}

/// 规范化输入路径并校验其为目录。返回绝对路径，错误时抛出。
fn clean_root_dir(root: &Path) -> Result<PathBuf, Box<dyn Error>> {
    let clean_root = fs::canonicalize(root)?;
    if !clean_root.is_dir() {
        return Err(format!("input path is not a directory: {}", clean_root.display()).into());
    }
    Ok(clean_root)
}

/// 收集 `root` 下所有直接子目录，并按路径字符串排序。
fn discover_group_dirs(root: &Path) -> Result<Vec<PathBuf>, Box<dyn Error>> {
    let mut dirs = Vec::new();
    for entry in fs::read_dir(root)? {
        let entry = entry?;
        let path = entry.path();
        // 仅保留目录，文件（如 root 自身的图片）被忽略
        if path.is_dir() {
            dirs.push(path);
        }
    }
    dirs.sort();
    Ok(dirs)
}

/// 读取目录内所有受支持格式的图片文件，按文件名字典序返回。
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

/// 把一组目录构造成按 `IMAGES_PER_PDF` 切块后的 `ImageGroup` 列表。
///
/// 同时通过 `used_names` 处理不同子目录同名时的去重命名（追加 `_2`、`_3`…）。
fn build_groups(dirs: Vec<PathBuf>) -> Result<Vec<ImageGroup>, Box<dyn Error>> {
    // 跨子目录重名时追加计数器，避免 PDF 文件名冲突
    let mut used_names: HashMap<String, i32> = HashMap::new();
    let mut groups = Vec::new();

    for dir in dirs {
        let group_files = read_dir_image_files(&dir)?;
        // 跳过没有任何图片的空目录
        if group_files.is_empty() {
            continue;
        }
        let base_name = group_name(&dir);

        // 按 IMAGES_PER_PDF 张一组切块
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

/// 从目录路径中提取用于命名的部分，并清洗掉文件名中的非法字符。
fn group_name(dir: &Path) -> String {
    let name = dir
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("images");
    safe_name(name)
}

/// 根据基础名与分块下标，生成 PDF 文件名（不含扩展名）。
///
/// - 第 1 个分块直接使用基础名；
/// - 后续分块在基础名后追加 `(chunk_index+1)`。
fn pdf_group_name(base_name: &str, chunk_index: usize) -> String {
    if chunk_index == 0 {
        base_name.to_string()
    } else {
        format!("{}{}", base_name, chunk_index + 1)
    }
}

/// 清洗目录名中的非法字符与首尾空白点线，使其能作为安全的文件名。
fn safe_name(name: &str) -> String {
    let cleaned: String = name
        .trim()
        .chars()
        .map(replace_unsafe_rune)
        .filter(|&c| c != '\0')
        .collect();

    // 去除首尾的空格/点/下划线，避免生成 ".pdf" 等怪异文件名
    let trimmed: String = cleaned
        .trim_matches(|c: char| c == ' ' || c == '.' || c == '_')
        .to_string();

    if trimmed.is_empty() {
        "images".to_string()
    } else {
        trimmed
    }
}

/// 维护跨目录同名分组的全局唯一名。
///
/// - 首次出现：直接返回原名；
/// - 重名：在末尾追加 `_<count>`。
fn unique_group_name(name: String, used: &mut HashMap<String, i32>) -> String {
    let count = used.entry(name.clone()).or_insert(0);
    *count += 1;
    if *count == 1 {
        name
    } else {
        format!("{}_{}", name, *count)
    }
}

/// 把不安全字符替换为下划线或剔除（控制字符替换为 NUL 后会被过滤）。
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

    /// 简单的临时目录封装，在 Drop 时自动清理。
    struct TempDir {
        path: PathBuf,
    }

    impl TempDir {
        fn new(name: &str) -> Self {
            // 用进程 ID 与纳秒时间戳拼接，最大限度避免并发测试冲突
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
            // 测试结束后自动清理临时目录
            let _ = fs::remove_dir_all(&self.path);
        }
    }

    /// 验证：9 张混合扩展名的输入会被切成 4 + 4 + 1 三个分组并按字典序排序。
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

    /// 验证：目录名中含 `:`、`?` 等非法字符时会被替换为 `_`，并触发跨目录去重命名。
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

    /// 验证：root 自身的图片和嵌套目录中的图片都会被忽略，仅扫描直接子目录。
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
