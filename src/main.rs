//! 程序入口模块，负责启动流程和业务调度。
//!
//! 命令行使用：
//!   img2pdf [输入目录] [输出目录]
//! - 输入目录：必填，存放若干子目录，每个子目录包含待转换图片（默认当前目录）。
//! - 输出目录：可选，未提供时使用 `备份/<日期>_pdfs` 作为默认目录。
//!
//! 业务逻辑概述：
//! 1. 调用 `images::discover_groups` 在输入目录下扫描直接子目录并按 4 张一组分块；
//! 2. 对每个分组调用 `pdf::write_image_grid_pdf` 生成 A4 四宫格 PDF；
//! 3. 汇总成功/失败数量并打印日志。

// 传递依赖版本冲突（hashbrown/wit-bindgen 等）由上游 crate 决定，本项目无法控制
#![allow(clippy::multiple_crate_versions)]
mod images;
mod pdf;

use std::env;
use std::error::Error;
use std::path::{Path, PathBuf};

use chrono::Local;

use images::discover_groups;
use pdf::write_image_grid_pdf;

/// CLI 入口。
///
/// 解析前两个位置参数分别为输入/输出目录，调度图片扫描与 PDF 生成流程。
///
/// # 错误
/// - 输入目录无法规范化为目录或读取子目录时返回 I/O 错误；
/// - PDF 生成失败不会中断整体流程，仅在该分组中打印错误并继续。
fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = env::args().collect();

    // 第一个参数：输入目录，默认当前目录
    let input_dir = if args.len() > 1 {
        Path::new(&args[1]).to_path_buf()
    } else {
        PathBuf::from(".")
    };

    // 第二个参数：输出目录，默认按当日日期生成子目录
    let output_dir = if args.len() > 2 {
        Path::new(&args[2]).to_path_buf()
    } else {
        default_output_dir()
    };

    println!("扫描分组根目录: {}", input_dir.display());
    println!("输出目录: {}", output_dir.display());

    // 在输入目录下发现子目录分组并按 4 张图片切块
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

    // 逐组生成 PDF，单组失败不中断
    let mut success_count = 0;
    for group in &groups {
        let output_path = output_dir.join(format!("{}.pdf", group.name));
        println!("处理: {} ({} 张图片)", group.name, group.files.len());

        match write_image_grid_pdf(&output_path, &group.name, &group.files) {
            Ok(()) => {
                success_count += 1;
                println!("  ✓ 已生成: {}", output_path.display());
            }
            Err(e) => {
                println!("  ✗ 失败: {e}");
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

/// 默认输出目录：本地 `备份/<YYYYMMDD>_pdfs`。
fn default_output_dir() -> PathBuf {
    let now = Local::now();
    PathBuf::from("备份").join(format!("{}_pdfs", now.format("%Y%m%d")))
}
