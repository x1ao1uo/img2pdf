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


# CLAUDE.md

Behavioral guidelines to reduce common LLM coding mistakes. Merge with project-specific instructions as needed.

**Tradeoff:** These guidelines bias toward caution over speed. For trivial tasks, use judgment.

## 1. Think Before Coding

**Don't assume. Don't hide confusion. Surface tradeoffs.**

Before implementing:
- State your assumptions explicitly. If uncertain, ask.
- If multiple interpretations exist, present them - don't pick silently.
- If a simpler approach exists, say so. Push back when warranted.
- If something is unclear, stop. Name what's confusing. Ask.

## 2. Simplicity First

**Minimum code that solves the problem. Nothing speculative.**

- No features beyond what was asked.
- No abstractions for single-use code.
- No "flexibility" or "configurability" that wasn't requested.
- No error handling for impossible scenarios.
- If you write 200 lines and it could be 50, rewrite it.

Ask yourself: "Would a senior engineer say this is overcomplicated?" If yes, simplify.

## 3. Surgical Changes

**Touch only what you must. Clean up only your own mess.**

When editing existing code:
- Don't "improve" adjacent code, comments, or formatting.
- Don't refactor things that aren't broken.
- Match existing style, even if you'd do it differently.
- If you notice unrelated dead code, mention it - don't delete it.

When your changes create orphans:
- Remove imports/variables/functions that YOUR changes made unused.
- Don't remove pre-existing dead code unless asked.

The test: Every changed line should trace directly to the user's request.

## 4. Goal-Driven Execution

**Define success criteria. Loop until verified.**

Transform tasks into verifiable goals:
- "Add validation" → "Write tests for invalid inputs, then make them pass"
- "Fix the bug" → "Write a test that reproduces it, then make it pass"
- "Refactor X" → "Ensure tests pass before and after"

For multi-step tasks, state a brief plan:
```
1. [Step] → verify: [check]
2. [Step] → verify: [check]
3. [Step] → verify: [check]
```

Strong success criteria let you loop independently. Weak criteria ("make it work") require constant clarification.

---

**These guidelines are working if:** fewer unnecessary changes in diffs, fewer rewrites due to overcomplication, and clarifying questions come before implementation rather than after mistakes.