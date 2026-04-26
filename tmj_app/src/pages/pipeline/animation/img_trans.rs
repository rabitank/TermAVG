use std::{
    path::PathBuf,
    time::{self, Duration},
};

use ratatui::{buffer::Buffer, layout::Rect};
use ratatui::{buffer::Cell, style::Color, widgets::Widget};
use tmj_core::{img::shape::Pic, script::TypeName};

use crate::{
    art::{
        halfblock::mix_into_cell,
        theme::{self, Theme},
    },
    pages::pipeline::{animation::Animation, visual_element::VisualElementKind},
};

#[derive(TypeName, Default)]
pub struct AniImgTrans {
    pub anim_time: time::Duration,
    pub old_image: Option<PathBuf>,
    pub new_image: Option<PathBuf>,
    pub run_time: time::Duration,
}

impl AniImgTrans {
    fn draw_image(
        image: Option<PathBuf>,
        rect: Rect,
        default_bg: Color,
        buffer: &mut Buffer,
    ) -> anyhow::Result<()> {
        if !image.is_none() {
            Pic::from(image.unwrap())?.render(rect, buffer);
        } else {
            let mut fill_cell = Cell::new(" ");
            fill_cell.set_bg(default_bg);
            *buffer = Buffer::filled(rect, fill_cell);
        };
        Ok(())
    }
}

impl Animation for AniImgTrans {
    fn apply_to_ve(
        &self,
        ve: &mut crate::pages::pipeline::visual_element::VisualElement,
    ) -> anyhow::Result<()> {
        let elapsed_secs = self.run_time.as_secs_f64().max(0.0);
        let total_secs = self.anim_time.as_secs_f64().max(0.0);
        let mut evalued_alpha = if total_secs == 0.0 {
            1_f64
        } else {
            elapsed_secs / total_secs
        };

        evalued_alpha = evalued_alpha.clamp(0.0, 1.0);

        if let VisualElementKind::Custom { drawer } = &mut ve.kind {
            let cur_img_path = self.new_image.clone();

            let old_image_path = self.old_image.clone();

            drawer.draw = Box::new(move |ve, buffer, rect| {
                let default_bg = ve.style.bg.unwrap_or(theme::BLACK);
                if evalued_alpha == 1.0 {
                    AniImgTrans::draw_image(cur_img_path.clone(), rect, default_bg, buffer)?;
                    return Ok(());
                }

                if evalued_alpha == 0.0 {
                    AniImgTrans::draw_image(old_image_path.clone(), rect, default_bg, buffer)?;
                    return Ok(());
                }

                let mut new_buf = Buffer::empty(rect);
                AniImgTrans::draw_image(cur_img_path.clone(), rect, default_bg, &mut new_buf)?;

                let mut old_buf = Buffer::empty(rect);
                AniImgTrans::draw_image(old_image_path.clone(), rect, default_bg, &mut old_buf)?;

                for row in rect.rows() {
                    for col in row.columns() {
                        let old_cell = &old_buf[(col.x, col.y)];
                        let new_cell = &new_buf[(col.x, col.y)];
                        let blend_cell = &mut buffer[(col.x, col.y)];
                        mix_into_cell(new_cell, old_cell, evalued_alpha as f32, blend_cell);
                    }
                }
                Ok(())
            });
        }
        Ok(())
    }

    fn update(&mut self, tick_delta: std::time::Duration) {
        self.run_time += tick_delta;
        self.run_time = self.run_time.clamp(Duration::ZERO, self.anim_time);
    }

    fn force_over(&mut self) {
        self.run_time = self.anim_time;
    }

    fn reset(&mut self) {
        self.run_time = time::Duration::from_secs_f64(0.0);
        self.anim_time = time::Duration::from_secs_f64(0.0);
    }

    fn is_animing(&self) -> bool {
        self.run_time < self.anim_time
    }
}
