//! PDF 模块，负责 PDF 文档生成与页面组织。
//!
//! 主要能力：
//! 1. 通过 `A4GridLayout` 在 A4 页面（默认 595×842 pt）上划分 2×2 四宫格；
//! 2. 使用 `fit_rect` 等比缩放图片到目标槽位内并居中；
//! 3. 通过 `printpdf` 把图片嵌入 XObject 并写出单页 PDF（最多 4 张）。
//!
//! 单位约定：内部计算用 pt（磅），最终转换到 mm 写入 PDF。

use std::error::Error;
use std::fs;
use std::path::Path;

use ::image::GenericImageView;
use printpdf::{Mm, Op, PdfDocument, PdfPage, PdfSaveOptions, Pt, RawImage, XObjectTransform};

/// 单页最大图片数，与 A4GridLayout 的 2×2 网格一一对应。
const IMAGES_PER_PAGE: usize = 4;

/// 二维矩形（pt 单位），用于描述页面内的图片槽位。
#[derive(Debug, Clone, Copy)]
pub struct Rect {
    /// 左下角 x 坐标。
    pub x: f32,
    /// 左下角 y 坐标（PDF 坐标系原点在左下）。
    pub y: f32,
    /// 矩形宽度。
    pub w: f32,
    /// 矩形高度。
    pub h: f32,
}

/// 图片的原始像素尺寸（取自 `image` crate 的 dimensions）。
#[derive(Debug, Clone, Copy)]
pub struct ImageSize {
    /// 图片宽度（像素）。
    pub w: f32,
    /// 图片高度（像素）。
    pub h: f32,
}

/// A4 网格布局参数。
#[derive(Debug, Clone, Copy)]
pub struct LayoutOptions {
    /// 页面宽度（pt）。
    pub page_width: f32,
    /// 页面高度（pt）。
    pub page_height: f32,
    /// 页面四周边距（pt）。
    pub margin: f32,
    /// 网格之间的水平/垂直间隙（pt）。
    pub gap: f32,
}

impl Default for LayoutOptions {
    fn default() -> Self {
        // 595×842 pt 对应 A4；24/12 pt 为常见的留白与间隙
        Self {
            page_width: 595.0,
            page_height: 842.0,
            margin: 24.0,
            gap: 12.0,
        }
    }
}

/// A4 2×2 网格布局：基于 `LayoutOptions` 计算四个图片槽位。
pub struct A4GridLayout {
    options: LayoutOptions,
}

impl A4GridLayout {
    /// 使用给定的布局参数构造实例，参数会被 `normalize_layout_options` 规整化。
    pub fn new(options: LayoutOptions) -> Self {
        Self {
            options: normalize_layout_options(options),
        }
    }

    /// 计算 A4 页面上四个等大的图片槽位。
    ///
    /// 返回数组顺序对应页面：
    /// - `[0]` 左上
    /// - `[1]` 右上
    /// - `[2]` 左下
    /// - `[3]` 右下
    pub fn image_slots(&self) -> [Rect; IMAGES_PER_PAGE] {
        let options = self.options;
        // 每个槽位的宽/高：扣除两侧 margin 后再扣掉 gap，再除以 2
        let slot_width = (2.0f32.mul_add(-options.margin, options.page_width) - options.gap) / 2.0;
        let slot_height =
            (2.0f32.mul_add(-options.margin, options.page_height) - options.gap) / 2.0;

        [
            Rect {
                x: options.margin,
                y: options.page_height - options.margin - slot_height,
                w: slot_width,
                h: slot_height,
            },
            Rect {
                x: options.margin + slot_width + options.gap,
                y: options.page_height - options.margin - slot_height,
                w: slot_width,
                h: slot_height,
            },
            Rect {
                x: options.margin,
                y: 2.0f32.mul_add(-slot_height, options.page_height - options.margin) - options.gap,
                w: slot_width,
                h: slot_height,
            },
            Rect {
                x: options.margin + slot_width + options.gap,
                y: 2.0f32.mul_add(-slot_height, options.page_height - options.margin) - options.gap,
                w: slot_width,
                h: slot_height,
            },
        ]
    }
}

/// 等比缩放 `size` 到 `box` 内部并居中，返回实际放置的矩形。
///
/// 当任一维度非正时返回零尺寸矩形，方便上层跳过绘制。
pub fn fit_rect(size: ImageSize, r#box: Rect) -> Rect {
    if size.w <= 0.0 || size.h <= 0.0 || r#box.w <= 0.0 || r#box.h <= 0.0 {
        return Rect {
            x: r#box.x,
            y: r#box.y,
            w: 0.0,
            h: 0.0,
        };
    }

    // 取最小缩放比，保证整张图片完整放入槽位
    let scale = f32::min(r#box.w / size.w, r#box.h / size.h);
    let width = size.w * scale;
    let height = size.h * scale;

    Rect {
        x: r#box.x + (r#box.w - width) / 2.0,
        y: r#box.y + (r#box.h - height) / 2.0,
        w: width,
        h: height,
    }
}

/// 规整化布局参数：非正值用默认值代替，避免下游除零或负尺寸。
fn normalize_layout_options(options: LayoutOptions) -> LayoutOptions {
    let defaults = LayoutOptions::default();
    LayoutOptions {
        page_width: if options.page_width <= 0.0 {
            defaults.page_width
        } else {
            options.page_width
        },
        page_height: if options.page_height <= 0.0 {
            defaults.page_height
        } else {
            options.page_height
        },
        margin: if options.margin < 0.0 {
            defaults.margin
        } else {
            options.margin
        },
        gap: if options.gap < 0.0 {
            defaults.gap
        } else {
            options.gap
        },
    }
}

/// pt -> mm 转换（1 in = 72 pt = 25.4 mm）。
fn pt_to_mm(pt: f32) -> Mm {
    Mm(pt * 25.4 / 72.0)
}

/// 读取图片真实像素尺寸（不解码全图，只读取头部）。
fn get_image_size(path: &Path) -> Result<ImageSize, Box<dyn Error>> {
    let img = ::image::open(path)?;
    let (width, height) = img.dimensions();
    Ok(ImageSize {
        w: width as f32,
        h: height as f32,
    })
}

/// 取第一页能容纳的图片切片（最多 `IMAGES_PER_PAGE` 张），剩余图片忽略。
fn first_page_images<T>(image_paths: &[T]) -> &[T] {
    &image_paths[..usize::min(image_paths.len(), IMAGES_PER_PAGE)]
}

/// 把一个图片分组按 A4 四宫格写入单页 PDF。
///
/// # 参数
/// - `output_path`: 输出 PDF 路径，必要时自动创建父目录。
/// - `_title`: 预留的标题参数（当前未写入 PDF）。
/// - `image_paths`: 待写入的图片路径列表，最多取前 4 张。
///
/// # 行为
/// - 列表为空时返回错误；
/// - 每张图片通过 `RawImage::decode_from_bytes` 解码后作为 XObject 嵌入；
/// - 坐标系使用 PDF 的 pt 单位，并通过 `0.75` 系数把像素 96 DPI 映射回 pt。
///
/// # 返回
/// 成功时返回 `Ok(())`，错误向上透传为 `Box<dyn Error>`。
pub fn write_image_grid_pdf(
    output_path: &Path,
    _title: &str,
    image_paths: &[impl AsRef<Path>],
) -> Result<(), Box<dyn Error>> {
    if image_paths.is_empty() {
        return Err(format!("no images to write for {_title}").into());
    }

    // 确保输出目录存在
    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent)?;
    }

    let layout = A4GridLayout::new(LayoutOptions::default());
    let page_width_mm = pt_to_mm(layout.options.page_width);
    let page_height_mm = pt_to_mm(layout.options.page_height);
    let slots = layout.image_slots();

    let mut doc = PdfDocument::new("img2pdf");
    let mut page_ops: Vec<Op> = Vec::new();

    // 遍历第一页图片，按槽位布局插入
    for (index, image_path) in first_page_images(image_paths).iter().enumerate() {
        let image_path = image_path.as_ref();
        let slot = slots[index];

        let img_size = get_image_size(image_path)?;
        let fit = fit_rect(img_size, slot);

        let image_bytes = fs::read(image_path)?;
        let mut warnings = Vec::new();
        let image = RawImage::decode_from_bytes(&image_bytes, &mut warnings)?;

        let image_xobject_id = doc.add_image(&image);

        page_ops.push(Op::UseXobject {
            id: image_xobject_id,
            transform: XObjectTransform {
                // 0.75 = 72/96，把像素按 96 DPI 转回 PDF 的 pt 单位
                translate_x: Some(Pt(fit.x * 0.75)),
                translate_y: Some(Pt(fit.y * 0.75)),
                scale_x: Some(fit.w / img_size.w),
                scale_y: Some(fit.h / img_size.h),
                ..XObjectTransform::default()
            },
        });
    }

    doc.with_pages(vec![PdfPage::new(page_width_mm, page_height_mm, page_ops)]);

    let mut warnings = Vec::new();
    let pdf_bytes = doc.save(&PdfSaveOptions::default(), &mut warnings);

    // 输出 PDF 库的告警（一般是字体/编码相关，不致命）
    if !warnings.is_empty() {
        eprintln!("PDF 生成警告: {warnings:?}");
    }

    fs::write(output_path, &pdf_bytes)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use ::image::{ImageBuffer, Rgb};
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    /// 简单的临时目录封装，在 Drop 时自动清理。
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

    /// 写入一个固定颜色的 8x8 PNG 作为测试图片。
    fn write_test_png(path: &Path, color: [u8; 3]) {
        let image = ImageBuffer::from_pixel(8, 8, Rgb(color));
        image.save(path).expect("test PNG should be written");
    }

    /// 验证：A4GridLayout 默认参数下始终返回 4 个槽位。
    #[test]
    fn a4_grid_layout_always_has_four_slots() {
        let layout = A4GridLayout::new(LayoutOptions::default());

        assert_eq!(layout.image_slots().len(), 4);
    }

    /// 验证：`first_page_images` 对超过 4 张的输入只取前 4 张。
    #[test]
    fn first_page_images_uses_at_most_four_images() {
        let images = [1, 2, 3, 4, 5];

        assert_eq!(first_page_images(&images), &[1, 2, 3, 4]);
    }

    /// 验证：`first_page_images` 对不足 4 张的输入按原样返回。
    #[test]
    fn first_page_images_keeps_short_input_for_empty_slots() {
        let images = [1, 2];

        assert_eq!(first_page_images(&images), &[1, 2]);
    }

    /// 验证：超过 4 张图片时也只生成一页，且嵌入 4 个 XObject。
    #[test]
    fn write_image_grid_pdf_uses_one_page_for_more_than_four_images() {
        let temp = TempDir::new("one-page");
        let mut images = Vec::new();
        for index in 0..5 {
            let path = temp.path().join(format!("{index}.png"));
            write_test_png(&path, [index * 20, 0, 0]);
            images.push(path);
        }
        let output = temp.path().join("out.pdf");

        write_image_grid_pdf(&output, "test", &images).expect("PDF should be written");

        let pdf = lopdf::Document::load(&output).expect("PDF should be readable");
        assert_eq!(pdf.get_pages().len(), 1);
        let page_id = *pdf.get_pages().get(&1).expect("first page should exist");
        assert_eq!(
            pdf.get_page_images(page_id)
                .expect("page images should be readable")
                .len(),
            4
        );
    }

    /// 验证：仅 1 张图片时仍正常生成单页 PDF，剩余槽位留空。
    #[test]
    fn write_image_grid_pdf_keeps_empty_slots_when_only_one_image_exists() {
        let temp = TempDir::new("one-image");
        let image = temp.path().join("0.png");
        write_test_png(&image, [255, 0, 0]);
        let output = temp.path().join("out.pdf");

        write_image_grid_pdf(&output, "test", &[image]).expect("PDF should be written");

        let pdf = lopdf::Document::load(&output).expect("PDF should be readable");
        assert_eq!(pdf.get_pages().len(), 1);
        let page_id = *pdf.get_pages().get(&1).expect("first page should exist");
        assert_eq!(
            pdf.get_page_images(page_id)
                .expect("page images should be readable")
                .len(),
            1
        );
    }
}
