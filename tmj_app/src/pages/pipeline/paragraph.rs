use ratatui::{
    layout::{Constraint, Layout, Margin},
    widgets::Wrap,
};
use tmj_core::script::TypeName;

use crate::{
    art::theme::THEME,
    pages::{
        pipeline::{
            logical_area,
            PipeStage,
            ve_utils::clear_animations_by_name,
            visual_element::{VisualElement, VisualElementKind},
        },
        script_def::{var_frame, var_paragraph},
    },
};

#[derive(TypeName)]
pub struct ParagraphStage;

impl PipeStage for ParagraphStage {
    fn binding_vars() -> &'static [&'static str] {
        &[var_frame::FRAME, var_paragraph::PARAGRAPH]
    }


}

impl ParagraphStage {
    pub const VE_PARAGRAPH_TEXT: &'static str = "paragraph.text";

    pub fn build_elements() -> Vec<VisualElement> {
        let area = logical_area();
        let rect = Layout::vertical([Constraint::Fill(1), Constraint::Length(16)]).split(area)[1];
        let rect = rect.inner(Margin::new(32, 0));
        vec![VisualElement {
            name: Self::VE_PARAGRAPH_TEXT.to_string(),
            visible: false,
            z_index: 300,
            rect,
            clear_before_draw: true,
            use_typewriter: true,
            typewriter_speed: 60.0,
            text_wrap: Some(Wrap { trim: true }),
            kind: VisualElementKind::Text {
                content: String::new(),
            },
            style: THEME.content,
            ..Default::default()
        }]
    }

    pub fn update_elements(
        screen: &crate::pages::dialogue::DialogueScene,
        ctx: &tmj_core::script::ContextRef,
        elements: &mut [VisualElement],
    ) -> anyhow::Result<()> {
        let area = logical_area();
        let mut vars = Self::get_script_vars(ctx);
        let paragraph = vars.pop().unwrap()?.as_table().unwrap();
        let frame = vars.pop().unwrap()?.as_table().unwrap();
        let frame_show = frame
            .borrow()
            .get(var_frame::VISIBLE)
            .and_then(|x| x.as_bool())
            .unwrap_or(true);
        let paragraph_show = paragraph
            .borrow()
            .get(var_paragraph::VISIBLE)
            .and_then(|x| x.as_bool())
            .unwrap_or(false);
        let visible = !screen.hide_dialouge && frame_show && paragraph_show;
        let rect = Layout::vertical([Constraint::Fill(1), Constraint::Length(16)]).split(area)[1];
        let rect = rect.inner(Margin::new(32, 0));
        if let Some(ve) = elements
            .iter_mut()
            .find(|x| x.name == Self::VE_PARAGRAPH_TEXT)
        {
            if !ve.is_animated {
                ve.visible = visible;
                ve.rect = rect;
            }
            ve.text_wrap = Some(Wrap { trim: true });
            let content = paragraph
                .borrow()
                .get(var_paragraph::CONTENT)
                .and_then(|x| x.as_str().map(|s| s.to_string()))
                .unwrap_or_default();
            let tw_enable = paragraph
                .borrow()
                .get(var_paragraph::TYPEWRITER_ENABLE)
                .and_then(|x| x.as_bool())
                .unwrap_or(true);
            let tw_speed = paragraph
                .borrow()
                .get(var_paragraph::TYPEWRITER_SPEED)
                .and_then(|x| x.to_number())
                .unwrap_or(60.0);
            if let VisualElementKind::Text { content: t } = &mut ve.kind {
                *t = content.clone();
            }
            ve.use_typewriter = tw_enable;
            ve.typewriter_speed = tw_speed;
        }
        Ok(())
    }

    pub fn stage_clear(
        _screen: &crate::pages::dialogue::DialogueScene,
        _ctx: &tmj_core::script::ContextRef,
        elements: &mut [VisualElement],
        _area: ratatui::prelude::Rect,
    ) -> anyhow::Result<()> {
        clear_animations_by_name(elements, Self::VE_PARAGRAPH_TEXT);
        Ok(())
    }
}
