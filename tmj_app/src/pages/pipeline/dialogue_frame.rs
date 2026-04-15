use ratatui::{
    layout::{Constraint, Layout, Margin, Rect},
    widgets::{Block, Clear, Paragraph, Widget},
};
use tmj_core::script::TypeName;

use crate::{
    SETTING, art::theme, pages::{
        pipeline::{PipeStage, typewriter::typewriter_render_text},
        script_def::{env::_TEXT_OBJ, text_obj, var_frame},
    }
};

#[derive(TypeName)]
pub struct DialogueFrameStage;

impl PipeStage for DialogueFrameStage {
    fn binding_vars() -> &'static [&'static str] {
        &[_TEXT_OBJ, var_frame::FRAME]
    }

    fn draw<'a>(
        screen: &crate::pages::dialogue::DialogueScene,
        ctx: &tmj_core::script::ContextRef,
        buffer: &'a mut ratatui::prelude::Buffer,
        area: ratatui::prelude::Rect,
    ) -> anyhow::Result<&'a mut ratatui::prelude::Buffer> {
        if screen.hide_dialouge {
            return Ok(buffer);
        };

        let rect = Layout::vertical([Constraint::Fill(1), Constraint::Length(16)]).split(area)[1];
        let rect = rect.inner(Margin::new(32, 0));

        {
            let dia_block = Block::new()
                .title("Dialogue")
                .style(theme::THEME.dialouge.block);
            Clear::default().render(rect, buffer);
            dia_block.render(rect, buffer);
        }

        let mut vars = Self::get_script_vars(ctx);
        let frame = vars.pop().unwrap()?.as_table().unwrap();
        let text_obj = vars.pop().unwrap()?.as_table().unwrap();

        let frame_show = frame
            .borrow()
            .get(var_frame::VISIBLE)
            .and_then(|x| x.as_bool())
            .unwrap_or(true);
        if !frame_show {
            return Ok(buffer);
        }

        let text = text_obj
            .borrow()
            .get(text_obj::CONTENT)
            .and_then(|x| x.as_str().map(|s| s.to_string()))
            .unwrap_or_default();
        let rendered = typewriter_render_text(&frame, &text, screen.last_tick_secs, true, 40.0);
        let text_par = Paragraph::new(rendered);
        let text_rect = Rect {
            x: SETTING.layout.text_lt.0 + area.x,
            y: SETTING.layout.text_lt.1 + area.y,
            width: SETTING.layout.text_size.0,
            height: SETTING.layout.text_size.1,
        };
        let text_rect = text_rect.clamp(rect);
        text_par.render(text_rect, buffer);
        Ok(buffer)
    }
}
