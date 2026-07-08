//! PDF 模块，负责 PDF 文档生成与页面组织。
//!
//! 本模块中的注释使用中文描述用途、参数和返回值，便于维护。

use std::error::Error;
use std::fs;
use std::path::Path;

use ::image::GenericImageView;
use printpdf::{Mm, Op, PdfDocument, PdfPage, PdfSaveOptions, Pt, RawImage, XObjectTransform};

const IMAGES_PER_PAGE: usize = 4;

#[derive(Debug, Clone, Copy)]
/// `Rect` 结构体，保存当前模块相关业务数据。
pub struct Rect {
    /// `x` 字段，存储对应业务数据。
    pub x: f32,
    /// `y` 字段，存储对应业务数据。
    pub y: f32,
    /// `w` 字段，存储对应业务数据。
    pub w: f32,
    /// `h` 字段，存储对应业务数据。
    pub h: f32,
}

#[derive(Debug, Clone, Copy)]
/// `ImageSize` 结构体，保存当前模块相关业务数据。
pub struct ImageSize {
    /// `w` 字段，存储对应业务数据。
    pub w: f32,
    /// `h` 字段，存储对应业务数据。
    pub h: f32,
}

#[derive(Debug, Clone, Copy)]
/// `LayoutOptions` 结构体，保存当前模块相关业务数据。
pub struct LayoutOptions {
    /// `page_width` 字段，存储对应业务数据。
    pub page_width: f32,
    /// `page_height` 字段，存储对应业务数据。
    pub page_height: f32,
    /// `margin` 字段，存储对应业务数据。
    pub margin: f32,
    /// `gap` 字段，存储对应业务数据。
    pub gap: f32,
}

impl Default for LayoutOptions {
    fn default() -> Self {
        Self {
            page_width: 595.0,
            page_height: 842.0,
            margin: 24.0,
            gap: 12.0,
        }
    }
}

/// `A4GridLayout` 结构体，保存当前模块相关业务数据。
pub struct A4GridLayout {
    options: LayoutOptions,
}

impl A4GridLayout {
    /// 创建新的实例。
    ///
    /// # 参数
    /// - `options`: 函数签名中定义的业务参数。
    ///
    /// # 返回
    /// 返回 `Self`，错误时按函数签名中的错误类型向上透传。
    pub fn new(options: LayoutOptions) -> Self {
        Self {
            options: normalize_layout_options(options),
        }
    }

    /// 执行 `image_slots` 操作，封装当前模块的业务流程。
    ///
    /// # 返回
    /// 返回 `[Rect`，错误时按函数签名中的错误类型向上透传。
    pub fn image_slots(&self) -> [Rect; IMAGES_PER_PAGE] {
        let options = self.options;
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

/// 执行 `fit_rect` 操作，封装当前模块的业务流程。
///
/// # 参数
/// - `size`: 函数签名中定义的业务参数。
///
/// # 返回
/// 返回 `Rect`，错误时按函数签名中的错误类型向上透传。
pub fn fit_rect(size: ImageSize, r#box: Rect) -> Rect {
    if size.w <= 0.0 || size.h <= 0.0 || r#box.w <= 0.0 || r#box.h <= 0.0 {
        return Rect {
            x: r#box.x,
            y: r#box.y,
            w: 0.0,
            h: 0.0,
        };
    }

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

fn pt_to_mm(pt: f32) -> Mm {
    Mm(pt * 25.4 / 72.0)
}

fn get_image_size(path: &Path) -> Result<ImageSize, Box<dyn Error>> {
    let img = ::image::open(path)?;
    let (width, height) = img.dimensions();
    Ok(ImageSize {
        w: width as f32,
        h: height as f32,
    })
}

fn first_page_images<T>(image_paths: &[T]) -> &[T] {
    &image_paths[..usize::min(image_paths.len(), IMAGES_PER_PAGE)]
}

/// 执行 `write_image_grid_pdf` 操作，封装当前模块的业务流程。
///
/// # 参数
/// - `output_path`: 函数签名中定义的业务参数。
/// - `_title`: 函数签名中定义的业务参数。
/// - `image_paths`: 函数签名中定义的业务参数。
///
/// # 返回
/// 返回 `Result<(), Box<dyn Error>>`，错误时按函数签名中的错误类型向上透传。
pub fn write_image_grid_pdf(
    output_path: &Path,
    _title: &str,
    image_paths: &[impl AsRef<Path>],
) -> Result<(), Box<dyn Error>> {
    if image_paths.is_empty() {
        return Err(format!("no images to write for {_title}").into());
    }

    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent)?;
    }

    let layout = A4GridLayout::new(LayoutOptions::default());
    let page_width_mm = pt_to_mm(layout.options.page_width);
    let page_height_mm = pt_to_mm(layout.options.page_height);
    let slots = layout.image_slots();

    let mut doc = PdfDocument::new("img2pdf");
    let mut page_ops: Vec<Op> = Vec::new();

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

    fn write_test_png(path: &Path, color: [u8; 3]) {
        let image = ImageBuffer::from_pixel(8, 8, Rgb(color));
        image.save(path).expect("test PNG should be written");
    }

    #[test]
    fn a4_grid_layout_always_has_four_slots() {
        let layout = A4GridLayout::new(LayoutOptions::default());

        assert_eq!(layout.image_slots().len(), 4);
    }

    #[test]
    fn first_page_images_uses_at_most_four_images() {
        let images = [1, 2, 3, 4, 5];

        assert_eq!(first_page_images(&images), &[1, 2, 3, 4]);
    }

    #[test]
    fn first_page_images_keeps_short_input_for_empty_slots() {
        let images = [1, 2];

        assert_eq!(first_page_images(&images), &[1, 2]);
    }

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
