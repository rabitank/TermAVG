use ratatui::{
    layout::{Constraint, Layout, Margin, Rect},
    widgets::{Block, Clear, Paragraph, Widget},
};
use tmj_core::script::TypeName;

use crate::{
    SETTING, art::theme, pages::{
        pipeline::PipeStage,
        script_def::{env::_TEXT_OBJ, text_obj},
    }
};

#[derive(TypeName)]
pub struct DialogueFrameStage;

impl PipeStage for DialogueFrameStage {
    fn binding_vars() -> &'static [&'static str] {
        &[_TEXT_OBJ]
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

        let binding = Self::get_script_vars(ctx).pop().unwrap()?;
        let text_obj = binding.as_table().unwrap();
        let binding = text_obj.borrow_mut().get(text_obj::CONTENT).unwrap();
        let text_par = Paragraph::new(binding.as_str().unwrap());
        let text_rect = Rect{
            x: SETTING.layout.text_lt.0 as u16 + area.x,
            y: SETTING.layout.text_lt.1 as u16 + area.y,
            width: SETTING.layout.text_size.0 as u16,
            height:SETTING.layout.text_size.1 as u16
        };
        let text_rect = text_rect.clamp(rect);
        text_par.render(text_rect, buffer);
        Ok(buffer)
    }
}
