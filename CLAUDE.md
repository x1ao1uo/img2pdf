# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## 项目概述

`img2pdf` 是固定 A4 四宫格图片转 PDF 批处理工具。将根目录下子目录中的图片，每 4 张合并为一个 A4 PDF（2x2 四宫格布局）。

## 常用命令

```bash
# 构建与运行
cargo build --release
cargo run --release -- <输入目录> <输出目录>

# 代码检查
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings

# 测试
cargo test
```

## 使用方式

```bash
# 完整参数
cargo run --release -- ./images ./pdfs

# 省略输入目录（默认当前目录）
cargo run --release -- ./pdfs

# 省略输出目录（默认 备份/YYYYMMDD_pdfs）
cargo run --release -- ./images
```

## 目录结构

```
src/
├── main.rs    # 入口、参数解析、目录遍历
├── images.rs  # 图片扫描与排序
└── pdf.rs     # PDF 生成（printpdf）
```

## 行为规则

- 输入：根目录下的一层子目录中的 jpg/jpeg/png 图片
- 输出：每个子目录每 4 张图片生成一个 PDF
- PDF 命名：目录名.pdf，第二个 PDF 为 目录名2.pdf
- 布局：固定 A4 竖版 2x2 四宫格，不足 4 张时空槽不绘制

## 依赖

- `printpdf` — PDF 生成
- `image` — 图片读取
- `chrono` — 日期处理

## 代码约定

- Rust Edition 2024
- 中文注释
- MIT License
