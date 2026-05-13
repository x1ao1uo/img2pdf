use std::error::Error;
use std::fs;
use std::path::Path;

use image::GenericImageView;
use printpdf::*;

const IMAGES_PER_PAGE: usize = 4;

#[derive(Debug, Clone, Copy)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

#[derive(Debug, Clone, Copy)]
pub struct ImageSize {
    pub w: f32,
    pub h: f32,
}

#[derive(Debug, Clone, Copy)]
pub struct LayoutOptions {
    pub page_width: f32,
    pub page_height: f32,
    pub margin: f32,
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

pub struct A4GridLayout {
    options: LayoutOptions,
}

impl A4GridLayout {
    pub fn new(options: LayoutOptions) -> Self {
        Self {
            options: normalize_layout_options(options),
        }
    }

    pub fn image_slots(&self) -> [Rect; IMAGES_PER_PAGE] {
        let options = self.options;
        let slot_width = (options.page_width - 2.0 * options.margin - options.gap) / 2.0;
        let slot_height = (options.page_height - 2.0 * options.margin - options.gap) / 2.0;

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
                y: options.page_height - options.margin - 2.0 * slot_height - options.gap,
                w: slot_width,
                h: slot_height,
            },
            Rect {
                x: options.margin + slot_width + options.gap,
                y: options.page_height - options.margin - 2.0 * slot_height - options.gap,
                w: slot_width,
                h: slot_height,
            },
        ]
    }
}

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
    let img = image::open(path)?;
    let (width, height) = img.dimensions();
    Ok(ImageSize {
        w: width as f32,
        h: height as f32,
    })
}

pub fn write_image_grid_pdf(
    output_path: &Path,
    _title: &str,
    image_paths: &[impl AsRef<Path>],
) -> Result<(), Box<dyn Error>> {
    if image_paths.is_empty() {
        return Err(format!("no images to write for {}", _title).into());
    }

    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent)?;
    }

    let layout = A4GridLayout::new(LayoutOptions::default());
    let page_width_mm = pt_to_mm(layout.options.page_width);
    let page_height_mm = pt_to_mm(layout.options.page_height);
    let slots = layout.image_slots();

    let mut doc = PdfDocument::new("img2pdf");
    let mut pages = Vec::new();

    for page_images in image_paths.chunks(IMAGES_PER_PAGE) {
        let mut page_ops: Vec<Op> = Vec::new();

        for (index, image_path) in page_images.iter().enumerate() {
            if index >= slots.len() {
                break;
            }
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

        pages.push(PdfPage::new(page_width_mm, page_height_mm, page_ops));
    }

    doc.with_pages(pages);

    let mut warnings = Vec::new();
    let pdf_bytes = doc.save(&PdfSaveOptions::default(), &mut warnings);

    if !warnings.is_empty() {
        eprintln!("PDF 生成警告: {:?}", warnings);
    }

    fs::write(output_path, &pdf_bytes)?;

    Ok(())
}
