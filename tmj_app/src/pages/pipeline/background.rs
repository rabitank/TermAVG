use ratatui::widgets::Widget;
use tmj_core::{img::shape::Pic, pathes, script::TypeName};

use crate::pages::{pipeline::PipeStage, script_def::env::BGIMG_PATH};

#[derive(TypeName)]
pub struct BackgrondStage;

impl PipeStage for BackgrondStage {
    fn binding_vars() -> &'static [&'static str] {
        &[BGIMG_PATH]
    }

    fn draw<'a>(
        _screen: &crate::pages::dialogue::DialogueScene,
        ctx: &tmj_core::script::ContextRef,
        buffer: &'a mut ratatui::prelude::Buffer,
        area: ratatui::prelude::Rect,
    ) -> anyhow::Result<&'a mut ratatui::prelude::Buffer> {
        let binding = Self::get_script_vars(ctx).pop().unwrap()?;
        let bgimg_path = binding.as_str().unwrap();
        if bgimg_path.is_empty() {
            return Ok(buffer);
        }
        let bgimg_path = pathes::path(bgimg_path);
        let bg_img = Pic::from(bgimg_path)?;
        bg_img.render(area, buffer);
        Ok(buffer)
    }
}
