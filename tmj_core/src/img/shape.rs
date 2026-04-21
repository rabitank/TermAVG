use std::{
    fs::File,
    io::BufReader,
    path::{Path, PathBuf},
};

use image::{DynamicImage, GenericImageView, ImageReader, Pixel};
use ratatui::{buffer::Buffer, layout::Rect, widgets::Widget};
use ratatui::{
    style::Color,
    symbols::Marker,
    widgets::canvas::{Canvas, Shape},
};

use anyhow::{Context, Result};

pub struct PicFrame {
    pub dimensions: (u32, u32),
    img: DynamicImage,
}

pub struct Pic {
    path: PathBuf,
    pic_frame: PicFrame,
}
// cover 优化图片透明部分的覆盖效果
fn cover(raw_buf: &mut Buffer, new_buf: &mut Buffer, area: Rect) {
    for row in area.rows() {
        for col in row.columns() {
            let cell = &mut raw_buf[(col.x, col.y)];
            let mask_cell = &mut new_buf[(col.x, col.y)];
            if mask_cell.symbol().is_empty() {
                continue;
            }
            if mask_cell.fg != Color::Reset {
                cell.set_fg(mask_cell.style().fg.unwrap());
                cell.set_symbol(mask_cell.symbol());
            }
            if mask_cell.bg != Color::Reset {
                cell.set_bg(mask_cell.style().bg.unwrap());
            }
        }
    }
}

///直接绘制图片的控件,不需要canvas
impl Pic {
    pub fn from(path: impl AsRef<Path>) -> Result<Self> {
        // 这样做可以统一后续处理逻辑，忽略原图可能是灰度、RGB等不同格式
        let path_buf = path.as_ref().to_path_buf();
        let pic_frame = PicFrame::from(&path_buf).unwrap();
        Ok(Self {
            path: path_buf,
            pic_frame,
        })
    }
}

impl Widget for Pic {
    fn render(self, area: ratatui::prelude::Rect, buf: &mut ratatui::prelude::Buffer)
    where
        Self: Sized,
    {
        let mut new_buf = Buffer::empty(area);
        let canva = Canvas::default()
            .x_bounds([0_f64, (self.pic_frame.dimensions.0 + 1).into()])
            .y_bounds([0_f64, (self.pic_frame.dimensions.1 + 1).into()])
            .marker(Marker::HalfBlock)
            .paint(move |ctx| {
                ctx.draw(&self.pic_frame);
            });
        canva.render(area, &mut new_buf);
        cover(buf, &mut new_buf, area);
    }
}

impl PicFrame {
    pub fn from(path: impl AsRef<Path>) -> Result<Self> {
        // 这样做可以统一后续处理逻辑，忽略原图可能是灰度、RGB等不同格式
        let img = Self::path_to_img(path)?;
        let dimensions = img.dimensions();
        // 4. 获取图像的元数据
        // println!("图片尺寸: {} x {}", width, height);
        Ok(Self { dimensions, img })
    }

    fn path_to_img(path: impl AsRef<Path>) -> Result<DynamicImage> {
        let file = File::open(&path).context(format!(
            "open {:?} field!",
            path.as_ref().to_path_buf().as_mut_os_string()
        ))?;
        let reader = BufReader::new(file);
        // 2. 创建 ImageReader 并解码
        // 如果不确定格式，可以使用 with_guessed_format
        // 或者直接通过扩展名自动识别： ImageReader::open("example.png")?
        let mut image_reader = ImageReader::new(reader);
        image_reader.set_format(image::ImageFormat::Png); // 明确指定格式
        let img = image_reader.decode()?; // 解码为 DynamicImage
        Ok(img)
    }
}

impl Shape for PicFrame {
    fn draw(&self, painter: &mut ratatui::widgets::canvas::Painter) {
        if let Some(img) = self.img.as_rgba8() {
            let ([_, max_w], _) = painter.bounds();
            if let Some((max_x, max_y)) = painter.get_point(max_w - 1_f64, 0_f64) {
                for (x, y, pixle) in img.enumerate_pixels() {
                    let a = pixle.alpha();
                    if a <= 1 || x as usize > max_x || y as usize > max_y {
                        continue;
                    }
                    let channels = pixle.channels();
                    let color: Color = Color::Rgb(channels[0], channels[1], channels[2]);
                    painter.paint(x.try_into().unwrap(), y.try_into().unwrap(), color);
                }
            }
        }
    }
}
