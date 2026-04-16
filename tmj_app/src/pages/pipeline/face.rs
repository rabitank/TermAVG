use ratatui::{
    widgets::Widget,
};
use tmj_core::{img::shape::Pic, script::TypeName};

use crate::{
    pages::{pipeline::PipeStage, script_def::{env::FACE_PATH, var_frame}},
    setting,
};

#[derive(TypeName)]
pub struct FaceStage;

impl PipeStage for FaceStage {
    fn binding_vars() -> &'static [&'static str] {
        &[FACE_PATH, var_frame::FRAME]
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
        let mut vars = Self::get_script_vars(ctx);
        let frame = vars.pop().unwrap()?.as_table().unwrap();

        let frame_show = frame
            .borrow()
            .get(var_frame::VISIBLE)
            .and_then(|x| x.as_bool())
            .unwrap_or(true);
        if !frame_show {
            return Ok(buffer);
        }

        let binding = vars.pop().unwrap()?;
        let img_path = binding
            .as_str()
            .ok_or(anyhow::anyhow!("{FACE_PATH} should be str"))?;
        if img_path.is_empty() {
            return Ok(buffer);
        }
        let face = Pic::from(img_path)?;
        let face_rect = setting::Layout::ltwh2rect(area, &setting::SETTING.layout.frame_face_ltwh);
        face.render(face_rect, buffer);
        Ok(buffer)
    }
}
