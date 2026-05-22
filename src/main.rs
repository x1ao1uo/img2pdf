//! 程序入口模块，负责启动流程和业务调度。
//!
//! 本模块中的注释使用中文描述用途、参数和返回值，便于维护。

mod images;
mod pdf;

use std::env;
use std::error::Error;
use std::path::{Path, PathBuf};

use chrono::Local;

use images::discover_groups;
use pdf::write_image_grid_pdf;

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = env::args().collect();

    let input_dir = if args.len() > 1 {
        Path::new(&args[1]).to_path_buf()
    } else {
        PathBuf::from(".")
    };

    let output_dir = if args.len() > 2 {
        Path::new(&args[2]).to_path_buf()
    } else {
        default_output_dir()
    };

    println!("扫描分组根目录: {}", input_dir.display());
    println!("输出目录: {}", output_dir.display());

    let groups = discover_groups(&input_dir)?;

    if groups.is_empty() {
        println!("未在直接子目录中找到可转换的图片。");
        return Ok(());
    }

    let total_images: usize = groups.iter().map(|g| g.files.len()).sum();
    println!(
        "找到 {} 个 PDF 分组，共 {} 张图片，开始转换...",
        groups.len(),
        total_images
    );

    let mut success_count = 0;
    for group in &groups {
        let output_path = output_dir.join(format!("{}.pdf", group.name));
        println!("处理: {} ({} 张图片)", group.name, group.files.len());

        match write_image_grid_pdf(&output_path, &group.name, &group.files) {
            Ok(_) => {
                success_count += 1;
                println!("  ✓ 已生成: {}", output_path.display());
            }
            Err(e) => {
                println!("  ✗ 失败: {}", e);
            }
        }
    }

    println!();
    println!(
        "已完成！成功生成 {} / {} 个 PDF。",
        success_count,
        groups.len()
    );
    println!("输出目录: {}", output_dir.display());

    Ok(())
}

fn default_output_dir() -> PathBuf {
    let now = Local::now();
    PathBuf::from("备份").join(format!("{}_pdfs", now.format("%Y%m%d")))
}
