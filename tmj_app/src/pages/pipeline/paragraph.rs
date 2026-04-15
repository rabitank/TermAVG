use ratatui::{
    layout::{Constraint, Layout, Margin},
    widgets::{Clear, Paragraph, Widget},
};
use tmj_core::script::TypeName;

use crate::pages::{
    pipeline::{PipeStage, typewriter::typewriter_render_text},
    script_def::{var_frame, var_paragraph},
};

#[derive(TypeName)]
pub struct ParagraphStage;

impl PipeStage for ParagraphStage {
    fn binding_vars() -> &'static [&'static str] {
        &[var_frame::FRAME, var_paragraph::PARAGRAPH]
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
        let paragraph = vars.pop().unwrap()?.as_table().unwrap();
        let frame = vars.pop().unwrap()?.as_table().unwrap();

        let frame_show = frame
            .borrow()
            .get(var_frame::VISIBLE)
            .and_then(|x| x.as_bool())
            .unwrap_or(true);
        if !frame_show {
            return Ok(buffer);
        }

        let paragraph_show = paragraph
            .borrow()
            .get(var_paragraph::VISIBLE)
            .and_then(|x| x.as_bool())
            .unwrap_or(false);
        if !paragraph_show {
            return Ok(buffer);
        }

        let rect = Layout::vertical([Constraint::Fill(1), Constraint::Length(16)]).split(area)[1];
        let rect = rect.inner(Margin::new(32, 0));
        let content = paragraph
            .borrow()
            .get(var_paragraph::CONTENT)
            .and_then(|x| x.as_str().map(|s| s.to_string()))
            .unwrap_or_default();
        let rendered = typewriter_render_text(&paragraph, &content, screen.last_tick_secs, true, 60.0);

        Clear::default().render(rect, buffer);
        Paragraph::new(rendered).render(rect, buffer);
        Ok(buffer)
    }
}
