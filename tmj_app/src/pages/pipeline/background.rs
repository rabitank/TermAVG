use ratatui::{
    layout::{Constraint, Layout},
    widgets::{Block, Clear, Widget},
};
use tmj_core::{img::shape::Pic, pathes, script::TypeName};

use crate::{
    SETTING,
    art::theme::THEME,
    pages::{
        pipeline::PipeStage,
        script_def::env::{_BLACK_V_EDGE, BGIMG_PATH},
    },
};

#[derive(TypeName)]
pub struct BackgroundStage;

impl PipeStage for BackgroundStage {
    fn binding_vars() -> &'static [&'static str] {
        &[BGIMG_PATH, _BLACK_V_EDGE]
    }

    fn draw<'a>(
        _screen: &crate::pages::dialogue::DialogueScene,
        ctx: &tmj_core::script::ContextRef,
        buffer: &'a mut ratatui::prelude::Buffer,
        area: ratatui::prelude::Rect,
    ) -> anyhow::Result<&'a mut ratatui::prelude::Buffer> {
        let mut vars = Self::get_script_vars(ctx);
        let use_v_edge = vars.pop().unwrap()?;
        let bgimg_path = vars.pop().unwrap()?;

        // render bg
        if !bgimg_path.is_nil() && !bgimg_path.as_str().unwrap().is_empty() {
            let bgimg_path = bgimg_path.as_string().unwrap();
            let bgimg_path = pathes::path(bgimg_path);
            tracing::info!("rendering bg {:?}", bgimg_path);
            let bg_img = Pic::from(bgimg_path)?;
            bg_img.render(area, buffer);
        }

        // render v black edge
        if use_v_edge.as_bool().unwrap_or(true) {
            let [up, _, down] = area.layout(&Layout::vertical([
                Constraint::Length(SETTING.layout.vertical_dark_edge),
                Constraint::Fill(1),
                Constraint::Length(SETTING.layout.vertical_dark_edge),
            ]));
            for r in vec![up, down] {
                let f = Block::default().style(THEME.dialouge.black_edge);
                Clear::render(Clear, r, buffer);
                f.render(r, buffer);
            }
        }
        Ok(buffer)
    }
}
