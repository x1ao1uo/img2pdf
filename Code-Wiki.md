# img2pdf — Code Wiki

> 自动生成于 2026-09-07,基于仓库快照;运行命令均以项目根目录为 cwd。

## 1. 项目概览

- **一句话定位**:一个本地 Rust CLI 工具,把根目录下若干子文件夹中的图片(每子目录一组)按 4 张/页的 A4 2×2 网格批量输出为 PDF 文件。
- **主要功能**:
  - 扫描输入目录的直接子目录,每个子目录作为一个图片分组;
  - 按文件名字典序排序后每 4 张切块,生成若干个 PDF(`IMAGES_PER_PDF = 4`);
  - 在每页 A4(595×842 pt)上等比缩放并居中放置图片,空槽位留白;
  - 默认输出目录为 `./备份/<YYYYMMDD>_pdfs`,也可由命令行第二参数覆盖;
  - 单组失败不影响其它组的生成,运行结束后输出成功/总数。
- **技术栈**:Rust 2024 edition、单 crate 二进制(`src/main.rs` 入口,无 lib target)、`printpdf` 生成 PDF、`image` 读取尺寸、`chrono` 生成日期目录。
- **仓库入口**:`Cargo.toml`、`src/main.rs`;质量门禁脚本 `./quality-gate.sh`(仓库已附);`README.md`、`AGENTS.md`、`CLAUDE.md` 给出项目级 Rust 规则与质量门禁说明。

## 2. 整体架构

- **形态**:单二进制 CLI,无 lib,无异步运行时,无线程模型;按目录树扫描 → 切块 → 同步逐组生成 PDF → 汇总输出。
- **分层**:
  - `main`(程序入口):解析位置参数 → 调用 `images` → 循环调用 `pdf` → 打印结果。
  - `images`(扫描 + 切块):文件系统扫描、扩展名过滤、按名排序、分块命名(含非法字符清洗与跨目录去重)。
  - `pdf`(PDF 生成):A4 网格布局计算、`fit_rect` 等比缩放居中、`printpdf` 嵌入 XObject 并写文件。
- **数据流**(顺序):
  1. `main` 解析 `argv[1]`(输入目录,默认 `.`)和 `argv[2]`(输出目录,默认 `备份/<日期>_pdfs`)。
  2. `images::discover_groups(root)` → `Vec<ImageGroup>`,每个 `ImageGroup { name, files: Vec<PathBuf> }`。
  3. 对每个 `ImageGroup`,`pdf::write_image_grid_pdf(out/<name>.pdf, name, &files)` 生成单页 PDF(最多嵌入前 4 张图片)。
  4. 统计 `success_count / groups.len()` 并打印汇总日志。
- **进程模型**:单进程、同步、阻塞 I/O;PDF 生成期间逐组串行执行,无并发。

## 3. 主要模块

### 3.1 `src/main.rs`(程序入口 / 业务调度)
- **路径**:`/Volumes/LVLIAN_1T/code/img2pdf/src/main.rs`
- **职责**:CLI 入口,解析输入/输出目录,串联 `images` 与 `pdf` 两个模块,并汇总成功/失败结果。
- **关键函数**:
  - `main()`:解析位置参数、调用 `discover_groups`、循环调用 `write_image_grid_pdf`、打印汇总。
  - `default_output_dir()`:基于 `chrono::Local::now()` 生成 `备份/<YYYYMMDD>_pdfs`。
- **依赖关系**:`images::discover_groups` → `pdf::write_image_grid_pdf`;依赖 `chrono::Local`;无测试模块。
- **副作用**:打印 `扫描分组根目录`、`输出目录`、`找到 N 个 PDF 分组`、`处理: ...`、`✓ 已生成: ...` / `✗ 失败: ...`、`已完成! 成功 X / Y`。

### 3.2 `src/images.rs`(目录扫描 + 分块 + 命名)
- **路径**:`/Volumes/LVLIAN_1T/code/img2pdf/src/images.rs`
- **职责**:发现根目录的直接子目录作为分组;在每个分组中按扩展名(`jpg|jpeg|png`,大小写不敏感)筛选图片,按文件名字典序排序,按 `IMAGES_PER_PDF = 4` 切块;生成去重后的 PDF 文件名(`base_name`、跨子目录同名追加 `_2`、`_3`…)并清洗非法字符(`/ \ : * ? " < > |` 与控制字符)。
- **关键类型/函数**:
  - `pub struct ImageGroup { pub name: String, pub files: Vec<PathBuf> }`
  - `pub fn discover_groups(root: &Path) -> Result<Vec<ImageGroup>, Box<dyn Error>>`
  - `fn clean_root_dir` / `discover_group_dirs` / `read_dir_image_files` / `build_groups`
  - `fn group_name` / `pdf_group_name` / `safe_name` / `unique_group_name` / `replace_unsafe_rune` / `is_supported_image`
- **依赖关系**:仅依赖 `std::collections::HashMap`、`std::fs`、`std::path`;输出供 `main` 与 `pdf` 之间传递路径列表。
- **测试模块**:本文件内嵌 `#[cfg(test)] mod tests`,覆盖切块顺序、目录名非法字符的去重、root 与嵌套目录图片被忽略三条场景。

### 3.3 `src/pdf.rs`(A4 网格 + PDF 写入)
- **路径**:`/Volumes/LVLIAN_1T/code/img2pdf/src/pdf.rs`
- **职责**:计算 A4 2×2 槽位、`fit_rect` 等比缩放并居中、用 `printpdf` 把图片作为 `XObject` 嵌入单页 PDF;必要时自动创建输出父目录。
- **关键类型/函数**:
  - `pub struct Rect { x, y, w, h }` (pt 单位,PDF 坐标原点左下)
  - `pub struct ImageSize { w, h }`(像素)
  - `pub struct LayoutOptions { page_width, page_height, margin, gap }` + `Default`(595×842 pt、margin 24、gap 12)
  - `pub struct A4GridLayout`:`new(options)`、`image_slots() -> [Rect; 4]`(顺序:左上、右上、左下、右下)
  - `pub fn fit_rect(size, box) -> Rect`(取最小缩放比并居中,空尺寸返回零矩形)
  - `pub fn write_image_grid_pdf(output_path, _title, image_paths) -> Result<(), Box<dyn Error>>`(主入口)
  - 私有 `pt_to_mm`、`get_image_size`(走 `image::open` 读取像素尺寸)、`first_page_images`(最多取 4 张)、`normalize_layout_options`(非正值回退默认值)
- **依赖关系**:`printpdf::{Mm, Op, PdfDocument, PdfPage, PdfSaveOptions, Pt, RawImage, XObjectTransform}`、`::image::GenericImageView`;开发时 `lopdf::Document` 用于测试断言页数与图像数。
- **测试模块**:本文件内嵌 `#[cfg(test)] mod tests`,覆盖默认布局恒返回 4 槽、`first_page_images` 截断、单页多图与单页单图的 PDF 页数 / XObject 数。
- **已知行为**:坐标转换用 `0.75 = 72/96` 把像素按 96 DPI 映射回 pt;`_title` 当前未写入 PDF(预留参数)。

## 4. 关键类与函数

- **入口函数**:`main()`(位置参数 → `discover_groups` → 循环 `write_image_grid_pdf`)。
- **协议 / 数据契约**:
  - `images::ImageGroup { name, files }` 是模块间传递图片批次的数据契约。
  - `pdf::LayoutOptions` 是布局可调参数的入口(默认 A4,可通过 `A4GridLayout::new` 替换,但当前 `main` 使用 `Default`)。
- **状态机**:无显式状态机;逻辑为单次线性流程。
- **常量与枚举**(皆来自源码,不再硬编码):
  - `images::IMAGES_PER_PDF = 4`(每个 PDF 最多图片数)
  - `pdf::IMAGES_PER_PAGE = 4`(每页槽位数,与 2×2 网格对应)
  - `pdf::LayoutOptions::default() = { page_width: 595.0, page_height: 842.0, margin: 24.0, gap: 12.0 }`(A4 + 默认留白)
  - 支持图片扩展名集合:`jpg | jpeg | png`(大小写不敏感)
  - 分块命名规则:首个分块 = 目录名;后续分块追加 `2`、`3`…;跨子目录重名追加 `_2`、`_3`…

## 5. 依赖关系

### 5.1 直接依赖(`Cargo.toml`)
- `printpdf = "0.12.6"`,`default-features = false`,`features = ["jpeg", "png"]` — PDF 文档生成与图片 XObject。
- `image = "0.25.10"`,`default-features = false`,`features = ["jpeg", "png"]` — 读取图片像素尺寸。
- `chrono = "0.4.45"` — `Local::now().format("%Y%m%d")` 生成默认输出目录日期后缀。

### 5.2 开发依赖
- `lopdf = "=0.44.0"`,`default-features = false` — 测试中解析生成的 PDF,断言页数与每页图像数(版本严格锁定避免破坏 `get_pages` / `get_page_images` 行为)。

### 5.3 系统 / 平台依赖
- 仅使用 Rust `std`(`fs`,`path`,`env`,`time` 等),无平台原生库绑定。
- 工具链约束(由仓库 `quality-gate.sh` 强校验):只接受 stable Rust;质量门禁脚本会读取 `rustc -vV` 并要求形如 `<major>.<minor>.<patch>` 的 release 字符串。

### 5.4 构建 / 审计工具
- `quality-gate.sh`(仓库内置,`rust-quality-gate:v4` 母版):要求 `cargo-audit`、`cargo-llvm-cov`、`cargo-nextest`、`cargo-machete`、`cargo-outdated`、`cargo-deny`;当 `UPGRADE=1` 或 `ENABLE_UPGRADE=1` 时再启用 `cargo upgrade` 与 `cargo update`。
- `deny.toml`:`unmaintained = "workspace"`(只对工作区直接依赖报警),放行常见宽松许可证(`MIT` / `Apache-2.0` / `BSD-*` / `ISC` / `MPL-2.0` / `Zlib` 等),仅允许 `crates.io` 已知注册源,`multiple-versions = "warn"`,明确豁免 `RUSTSEC-2023-0071`(rsa Marvin Attack,无安全升级路径)。
- `.github/dependabot.yml`:每周一 04:00(Asia/Shanghai)扫描 `cargo` 生态,minor/patch 合并为一个 PR,带 `deps/cargo`、`auto-update` 标签。

## 6. 项目运行方式

### 6.1 构建命令
```bash
cargo build --release
# 仓库推荐:严格、可复现,通过仓库内置门禁脚本完成
./quality-gate.sh
```

### 6.2 运行命令
```bash
# 用法:img2pdf [输入目录] [输出目录]
# 输入目录:必填,扫描其直接子目录;默认为当前目录。
# 输出目录:可选;默认 ./备份/<YYYYMMDD>_pdfs。
cargo run --release -- [输入目录] [输出目录]
```

### 6.3 测试命令
```bash
# 单元测试(项目内嵌在 src/images.rs / src/pdf.rs 的 #[cfg(test)] mod tests)
cargo test --locked
# 仓库推荐的严格流水线:fmt + clippy + test + build + audit + deny
./quality-gate.sh
# 升级全部直接 / 传递依赖到当时最新稳定版(可选,需 ENABLE_UPGRADE=1)
ENABLE_UPGRADE=1 ./quality-gate.sh
```

### 6.4 关键配置
- `Cargo.toml`:`edition = "2024"`,包名 `img2pdf`,版本 `0.1.0`,`MIT` 许可;`printpdf` / `image` 仅启用 `jpeg`、`png` feature。
- `deny.toml`:见 §5.4;明确豁免 `RUSTSEC-2023-0071`(复查日期 2026-11-01)。
- `quality-gate.sh`:质量门禁总入口;默认 `FMT_FIX=1`(本地先 `cargo fmt` 再 `--check`),`COV_MIN` 留空(不卡覆盖率阈值)。
- `.gitignore`:`/target`、`.claude/settings.local.json`。
- 依赖升级触发:Dependabot 周一轮询 + 仓库 `ENABLE_UPGRADE=1 ./quality-gate.sh`。

## 7. 约定与备注

- **命名规范**:Rust 风格 snake_case;模块公开 API 用 `pub fn` / `pub struct`,内部辅助函数模块私有;测试模块统一 `#[cfg(test)] mod tests` 并复用 `TempDir`(Drop 时清理 `/tmp/img2pdf-<name>-<pid>-<nanos>`)。
- **重要约束**:
  - 仅扫描输入根目录的直接子目录,root 自身的图片与嵌套目录里的图片会被忽略(由 `discover_groups` 测试明确约束)。
  - 受支持扩展名只有 `jpg|jpeg|png`(大小写不敏感),其它扩展名会被 `is_supported_image` 过滤掉。
  - PDF 单页最多嵌入 4 张图片,`first_page_images` 截断;多余图片被丢弃(由 `write_image_grid_pdf_uses_one_page_for_more_than_four_images` 测试覆盖)。
  - 布局参数走 `normalize_layout_options`,非正值回退默认;`fit_rect` 对空尺寸返回零矩形以避免除零。
  - 仓库 `CLAUDE.md` / `AGENTS.md` 强制要求 Rust 项目遵守 stable 工具链,提交前必须 `./quality-gate.sh` 通过;不通过不得声称完成。
- **已知风险 / 遗留事项**:
  - `pdf::write_image_grid_pdf` 的 `_title` 参数当前未写入 PDF(预留,命名 `name` 仅用于日志与输出文件名)。
  - 像素 → pt 使用硬编码 `0.75`(96 DPI 假设);若图片 EXIF DPI 与 96 偏差较大,显示尺寸可能与期望不符。
  - 多组生成串行执行,没有并行 / 进度条 / 取消信号;大目录下 `cargo run --release` 仍可能较慢。
  - `Cargo.lock` 已提交,质量门禁强制 `--locked`,确保可复现。
  - `deny.toml` 显式豁免 `RUSTSEC-2023-0071`(rsa Marvin Attack)复查日期 2026-11-01;届时需复核 RustCrypto 是否发布修复版并取消豁免。
  - 单一二进制、无 lib target,故质量门禁跳过 `cargo test --doc` 与 `rustdoc -D warnings` 中的 lib 相关步骤(脚本自动判断)。