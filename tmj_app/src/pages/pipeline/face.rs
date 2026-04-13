use ratatui::{
    layout::{Constraint, Layout, Offset},
    widgets::Widget,
};
use tmj_core::{img::shape::Pic, script::TypeName};

use crate::{
    SETTING,
    pages::{pipeline::PipeStage, script_def::env::FACE_PATH},
};

#[derive(TypeName)]
pub struct FaceStage;

impl PipeStage for FaceStage {
    fn binding_vars() -> &'static [&'static str] {
        &[FACE_PATH]
    }

    fn draw<'a>(
        screen: &crate::pages::dialogue::DialogueScene,
        ctx: &tmj_core::script::ContextRef,
        buffer: &'a mut ratatui::prelude::Buffer,
        area: ratatui::prelude::Rect,
    ) -> anyhow::Result<&'a mut ratatui::prelude::Buffer> {
        if screen.hide_dialouge {
            return Ok(buffer);
        }

        let binding = Self::get_script_vars(ctx).pop().unwrap()?;
        let img_path = binding
            .as_str()
            .ok_or(anyhow::anyhow!("{FACE_PATH} should be str"))?;
        if img_path.is_empty() {
            return Ok(buffer);
        }
        let face = Pic::from(img_path)?;
        let face_rect = Layout::vertical([
            Constraint::Length(SETTING.layout.df_size.1 as u16),
            Constraint::Fill(1),
        ])
        .split(area)[0];
        let face_rect = Layout::horizontal([
            Constraint::Length(SETTING.layout.df_size.0),
            Constraint::Fill(1),
        ])
        .split(face_rect)[0];

        let face_rect = face_rect.offset(Offset::new(
            SETTING.layout.df_lt.0 as i32,
            SETTING.layout.df_lt.1 as i32,
        ));
        let face_rect = face_rect.clamp(area);
        face.render(face_rect, buffer);
        Ok(buffer)
    }
}
