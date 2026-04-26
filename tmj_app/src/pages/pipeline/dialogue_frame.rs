use ratatui::{
    layout::{Alignment, Rect},
    text::{Line, Span},
    widgets::{Paragraph, Widget, Wrap},
};
use tmj_core::script::{ContextRef, TypeName};

use crate::{
    LAYOUT,
    art::theme::THEME,
    layout::Layout,
    pages::{
        dialogue::DialogueScene,
        pipeline::{
            Behaviour,
            animation::{self, Animation},
            logical_area,
            visual_element::{VisualElement, VisualElementCustomDrawer, VisualElementKind},
        },
        script_def::var_frame,
    },
};

#[derive(TypeName)]
pub struct FrameBehaviour {
    face_img: String,
    text: String,
    speaker: String,
    typewriter: animation::typewriter::AniTypeWriter,
}

impl Default for FrameBehaviour {
    fn default() -> Self {
        Self {
            face_img: String::default(),
            text: String::default(),
            speaker: String::default(),
            typewriter: animation::typewriter::AniTypeWriter {
                speed: 30.0,
                ..Default::default()
            },
        }
    }
}

impl FrameBehaviour {
    pub const VE_FRAME_BLOCK: &'static str = "frame.block";
    pub const VE_FRAME_TEXT: &'static str = "frame.text";
    pub const VE_FRAME_NAME: &'static str = "frame.name";
    pub const VE_FRAME_SHORTKEY: &'static str = "frame.shortkey";
    pub const VE_FACE: &'static str = "frame.face";

    pub fn export_text(&mut self, text: String) {
        self.face_img = "".into();
        self.text = text.clone();
        self.speaker = "".into();
        self.typewriter.reset();
        self.typewriter.target_text = text;
        self.typewriter.start_text = "".into();
    }

    pub fn export_say(&mut self, speaker: String, face_img: String, text: String) {
        self.face_img = face_img;
        self.text = text.clone();
        self.speaker = speaker;
        self.typewriter.reset();
        self.typewriter.target_text = text;
        self.typewriter.start_text = "".into();
    }
}

impl Behaviour for FrameBehaviour {
    fn is_animating(&self) -> bool {
        self.typewriter.is_animing()
    }

    fn tick_update(&mut self, _ctx: ContextRef, delta_time: std::time::Duration) {
        self.typewriter.update(delta_time);
    }

    fn on_force_over_animation(&mut self) -> anyhow::Result<()> {
        self.typewriter.force_over();
        Ok(())
    }

    fn on_end_dialouge(&mut self) -> anyhow::Result<()> {
        self.face_img = "".into();
        self.text = "".into();
        self.typewriter.reset();
        Ok(())
    }

    fn on_end_session(&mut self, _ctx: tmj_core::script::ContextRef) -> anyhow::Result<()> {
        self.face_img = "".into();
        self.text = "".into();
        self.typewriter.reset();
        Ok(())
    }

    fn binding_vars(&self) -> &'static [&'static str] {
        &[var_frame::FRAME]
    }

    fn build_elements(
        &self,
        _ctx: &tmj_core::script::ContextRef,
    ) -> anyhow::Result<Vec<VisualElement>> {
        let area = logical_area();
        let frame_rect = Layout::ltwh2rect(area, &LAYOUT.frame_content_ltwh);
        let text_rect = Layout::ltwh2rect(area, &LAYOUT.text_ltwh).clamp(frame_rect);
        let name_rect = Layout::ltwh2rect(area, &LAYOUT.frame_name_ltwh);
        let short_key_rect = Layout::ltwh2rect(area, &LAYOUT.short_key_ltwh);
        let ves = vec![
            VisualElement {
                name: Self::VE_FACE.to_string(),
                z_index: 230,
                rect: Layout::ltwh2rect(area, &LAYOUT.frame_face_ltwh),
                kind: VisualElementKind::Image {
                    source: String::new(),
                },
                ..Default::default()
            },
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
                kind: VisualElementKind::Custom { drawer: VisualElementCustomDrawer::from(draw_shortkey_bar)},
                style: THEME.content,
                ..Default::default()
            },
        ];

        Ok(ves)
    }

    fn update_elements(
        &self,
        screen: &DialogueScene,
        ctx: &tmj_core::script::ContextRef,
        elements: &mut Vec<VisualElement>,
    ) -> anyhow::Result<()> {
        let mut vars = self.get_bind_vars(ctx);
        let frame = vars.pop().unwrap()?.as_table_or_resolve(ctx).unwrap();
        let frame_show = frame
            .borrow()
            .get(var_frame::VISIBLE, None)
            .and_then(|x| x.as_bool())
            .unwrap_or(true);
        let show_all = !screen.hide_dialouge && frame_show;

        for name in [
            Self::VE_FRAME_BLOCK,
            Self::VE_FRAME_TEXT,
            Self::VE_FRAME_NAME,
            Self::VE_FRAME_SHORTKEY,
            Self::VE_FACE,
        ] {
            if let Some(ve) = elements.iter_mut().find(|x| x.name == name) {
                ve.visible = show_all;
            }
        }

        if !show_all {
            return Ok(());
        }

        if let Some(ve) = elements.iter_mut().find(|x| x.name == Self::VE_FRAME_TEXT) {
            self.typewriter.apply_to_ve(ve);
        }

        if let Some(ve) = elements.iter_mut().find(|x| x.name == Self::VE_FACE) {
            ve.visible = !self.face_img.is_empty();
            if !self.face_img.is_empty() && let VisualElementKind::Image { source } = &mut ve.kind {
                *source = self.face_img.clone();
            }
        }

        if let Some(ve) = elements.iter_mut().find(|x| x.name == Self::VE_FRAME_NAME) {
            if self.speaker.is_empty() {
                ve.visible = false;
            }

            if let VisualElementKind::Text { content } = &mut ve.kind {
                *content = self.speaker.clone();
            }
        }

        Ok(())
    }
}

fn draw_shortkey_bar(
    _ve: &VisualElement,
    buffer: &mut ratatui::buffer::Buffer,
    rect: Rect,
) -> anyhow::Result<()> {
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
    Paragraph::new(line)
        .alignment(Alignment::Center)
        .render(rect, buffer);
    Ok(())
}
