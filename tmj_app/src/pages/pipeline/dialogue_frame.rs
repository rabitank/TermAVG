use ratatui::{
    layout::Rect,
    style::Color,
    text::{Line, Span, Text},
    widgets::{Block, Clear, Paragraph, Widget},
};
use tmj_core::script::TypeName;

use crate::{
    SETTING,
    art::theme::{self, THEME},
    pages::{
        pipeline::{PipeStage, typewriter::typewriter_render_text},
        script_def::{env::_TEXT_OBJ, var_frame},
    },
    setting,
};

#[derive(TypeName)]
pub struct DialogueFrameStage;

impl DialogueFrameStage {
    fn render_bottom_bar(area: Rect, buf: &mut ratatui::prelude::Buffer) {
        let keys = [
            #[cfg(debug_assertions)]
            ("Ctr+.", "CmdLine"),
            ("Click/Enter", "Next"),
            ("s", "Save"),
            ("l", "Load"),
            ("h", "HideFrame"),
            ("Q/Esc", "Quit"),
        ];
        let spans: Vec<_> = keys
            .iter()
            .flat_map(|(key, desc)| {
                let key = Span::styled(format!(" {key} "), THEME.key_binding.key);
                let desc = Span::styled(format!(" {desc} "), THEME.key_binding.description);
                [key, desc]
            })
            .collect();
        Line::from(spans)
            .centered()
            .style((Color::Indexed(236), Color::Indexed(232)))
            .render(area, buf);
    }
}
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

        //render frame
        let frame_rect =
            setting::Layout::ltwh2rect(area, &setting::SETTING.layout.frame_content_ltwh);
        {
            let dia_block = Block::new()
                .style(theme::THEME.dialouge.block);
            Clear::default().render(frame_rect, buffer);
            dia_block.render(frame_rect, buffer);
        }

        // render text
        let text = frame
            .borrow()
            .get(var_frame::CONTENT)
            .and_then(|x| x.as_str().map(|s| s.to_string()))
            .unwrap_or_default();
        // tracing::info!("frame target {text}");
        let rendered = typewriter_render_text(&frame, &text, screen.last_tick_secs, true, 40.0);
        // tracing::info!("frame current {rendered}");
        let text_par = Paragraph::new(rendered);
        let text_rect = setting::Layout::ltwh2rect(area, &SETTING.layout.text_ltwh);
        let text_rect = text_rect.clamp(frame_rect);
        text_par.render(text_rect, buffer);

        // name
        let name_rect = setting::Layout::ltwh2rect(area, &setting::SETTING.layout.frame_name_ltwh);
        let speaker = frame
            .borrow()
            .get(var_frame::SPEAKER)
            .and_then(|x| x.as_string().cloned())
            .unwrap_or("".to_string());
        if !speaker.is_empty() {
            let name_text = Span::from(speaker).style(theme::THEME.dialouge.name);
            let name_text = Line::from(name_text).style(theme::THEME.dialouge.name).centered();
            Clear::render(Clear,name_rect, buffer);
            name_text.render(name_rect, buffer);
        }

        // short key
        let short_key_rect =
            setting::Layout::ltwh2rect(area, &setting::SETTING.layout.short_key_ltwh);
        DialogueFrameStage::render_bottom_bar(short_key_rect, buffer);

        Ok(buffer)
    }
}
