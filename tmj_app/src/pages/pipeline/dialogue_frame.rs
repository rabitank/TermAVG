use ratatui::{
    layout::{Alignment, Rect},
    text::{Line, Span},
    widgets::{Paragraph, Widget, Wrap},
};
use tmj_core::script::TypeName;

use crate::{
    art::theme::THEME,
    LAYOUT,
    layout::Layout,
    pages::{
        pipeline::{
            logical_area,
            PipeStage,
            ve_utils::clear_animations_by_prefix,
            visual_element::{VisualElement, VisualElementKind},
        },
        script_def::var_frame,
    },
};

#[derive(TypeName)]
pub struct DialogueFrameStage;

fn draw_shortkey_bar(_ve: &VisualElement, buffer: &mut ratatui::buffer::Buffer, rect: Rect) -> anyhow::Result<()> {
    let key = THEME.key_binding.key;
    let desc = THEME.key_binding.description;
    let line = Line::from(vec![
        Span::styled(" Click/Enter ", key),
        Span::styled(" Next  ", desc),
        Span::styled(" s ", key),
        Span::styled(" Save  ", desc),
        Span::styled(" l ", key),
        Span::styled(" Load  ", desc),
        Span::styled(" h ", key),
        Span::styled(" HideFrame  ", desc),
        Span::styled(" Q/Esc ", key),
        Span::styled(" Quit", desc),
    ]);
    Paragraph::new(line).alignment(Alignment::Center).render(rect, buffer);
    Ok(())
}

impl DialogueFrameStage {
    pub const VE_FRAME_BLOCK: &'static str = "frame.block";
    pub const VE_FRAME_TEXT: &'static str = "frame.text";
    pub const VE_FRAME_NAME: &'static str = "frame.name";
    pub const VE_FRAME_SHORTKEY: &'static str = "frame.shortkey";

    pub fn build_elements() -> Vec<VisualElement> {
        let area = logical_area();
        let frame_rect = Layout::ltwh2rect(area, &LAYOUT.frame_content_ltwh);
        let text_rect = Layout::ltwh2rect(area, &LAYOUT.text_ltwh).clamp(frame_rect);
        let name_rect = Layout::ltwh2rect(area, &LAYOUT.frame_name_ltwh);
        let short_key_rect = Layout::ltwh2rect(area, &LAYOUT.short_key_ltwh);
        vec![
            VisualElement {
                name: Self::VE_FRAME_BLOCK.to_string(),
                z_index: 200,
                rect: frame_rect,
                clear_before_draw: true,
                text_wrap: Some(Wrap { trim: false }),
                kind: VisualElementKind::Fill,
                style: THEME.dialouge.block,
                ..Default::default()
            },
            VisualElement {
                name: Self::VE_FRAME_TEXT.to_string(),
                z_index: 210,
                rect: text_rect,
                use_typewriter: true,
                typewriter_speed: 40.0,
                text_wrap: Some(Wrap { trim: true }),
                kind: VisualElementKind::Text {
                    content: String::new(),
                },
                style: THEME.dialouge.inbox,
                ..Default::default()
            },
            VisualElement {
                name: Self::VE_FRAME_NAME.to_string(),
                visible: false,
                z_index: 220,
                rect: name_rect,
                clear_before_draw: true,
                text_wrap: Some(Wrap { trim: false }),
                kind: VisualElementKind::Text {
                    content: String::new(),
                },
                style: THEME.dialouge.name,
                ..Default::default()
            },
            VisualElement {
                name: Self::VE_FRAME_SHORTKEY.to_string(),
                z_index: 220,
                rect: short_key_rect,
                text_wrap: Some(Wrap { trim: false }),
                kind: VisualElementKind::Custom { draw: draw_shortkey_bar },
                style: THEME.content,
                ..Default::default()
            },
        ]
    }

    pub fn update_elements(
        screen: &crate::pages::dialogue::DialogueScene,
        ctx: &tmj_core::script::ContextRef,
        elements: &mut [VisualElement],
    ) -> anyhow::Result<()> {
        let area = logical_area();
        let mut vars = Self::get_script_vars(ctx);
        let frame = vars.pop().unwrap()?.as_table().unwrap();
        let frame_show = frame
            .borrow()
            .get(var_frame::VISIBLE)
            .and_then(|x| x.as_bool())
            .unwrap_or(true);
        let show_all = !screen.hide_dialouge && frame_show;
        let frame_rect = Layout::ltwh2rect(area, &LAYOUT.frame_content_ltwh);
        let text_rect = Layout::ltwh2rect(area, &LAYOUT.text_ltwh).clamp(frame_rect);
        let name_rect = Layout::ltwh2rect(area, &LAYOUT.frame_name_ltwh);
        let short_key_rect = Layout::ltwh2rect(area, &LAYOUT.short_key_ltwh);

        for name in [
            Self::VE_FRAME_BLOCK,
            Self::VE_FRAME_TEXT,
            Self::VE_FRAME_NAME,
            Self::VE_FRAME_SHORTKEY,
        ] {
            if let Some(ve) = elements.iter_mut().find(|x| x.name == name) {
                if !ve.is_animated {
                    ve.visible = show_all;
                }
            }
        }
        if !show_all {
            return Ok(());
        }

        if let Some(ve) = elements.iter_mut().find(|x| x.name == Self::VE_FRAME_BLOCK) {
            if !ve.is_animated {
                ve.rect = frame_rect;
            }
        }
        let content = frame
            .borrow()
            .get(var_frame::CONTENT)
            .and_then(|x| x.as_str().map(|s| s.to_string()))
            .unwrap_or_default();
        let tw_enable = frame
            .borrow()
            .get(var_frame::TYPEWRITER_ENABLE)
            .and_then(|x| x.as_bool())
            .unwrap_or(true);
        let tw_speed = frame
            .borrow()
            .get(var_frame::TYPEWRITER_SPEED)
            .and_then(|x| x.to_number())
            .unwrap_or(40.0);
        if let Some(ve) = elements.iter_mut().find(|x| x.name == Self::VE_FRAME_TEXT) {
            if !ve.is_animated {
                ve.rect = text_rect;
            }
            ve.text_wrap = Some(Wrap { trim: true });
            if let VisualElementKind::Text { content: t } = &mut ve.kind {
                *t = content.clone();
            }
            ve.use_typewriter = tw_enable;
            ve.typewriter_speed = tw_speed;
        }
        let speaker = frame
            .borrow()
            .get(var_frame::SPEAKER)
            .and_then(|x| x.as_string().cloned())
            .unwrap_or_default();
        if let Some(ve) = elements.iter_mut().find(|x| x.name == Self::VE_FRAME_NAME) {
            if !ve.is_animated {
                ve.rect = name_rect;
                ve.visible = show_all && !speaker.is_empty();
                if let VisualElementKind::Text { content: t } = &mut ve.kind {
                    *t = speaker;
                }
            }
        }
        if let Some(ve) = elements
            .iter_mut()
            .find(|x| x.name == Self::VE_FRAME_SHORTKEY)
        {
            if !ve.is_animated {
                ve.rect = short_key_rect;
            }
        }
        Ok(())
    }

    pub fn stage_clear(
        _screen: &crate::pages::dialogue::DialogueScene,
        _ctx: &tmj_core::script::ContextRef,
        elements: &mut [VisualElement],
        _area: Rect,
    ) -> anyhow::Result<()> {
        clear_animations_by_prefix(elements, "frame.");
        Ok(())
    }
}
impl PipeStage for DialogueFrameStage {
    fn binding_vars() -> &'static [&'static str] {
        &[var_frame::FRAME]
    }
    
}
