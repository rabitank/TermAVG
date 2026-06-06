use std::time::Duration;

use ratatui::{
    layout::{Alignment, Rect},
    text::{Line, Span},
    widgets::{Paragraph, Widget, Wrap},
};
use tmj_core::script::{ContextRef, ScriptValue, TypeName};

use crate::{
    LAYOUT,
    art::theme::{THEME, Theme},
    layout::Layout,
    pages::{
        behaviour::{
            Behaviour,
            animation::{self, Animation},
            logical_area,
            ve_z_index::{
                Z_FRAME_BLOCK, Z_FRAME_FACE, Z_FRAME_NAME, Z_FRAME_SHORTKEY, Z_FRAME_TEXT,
            },
            visual_element::{VisualElement, VisualElementCustomDrawer, VisualElementKind},
        },
        dialogue::DialogueScene,
        script_def::var_frame,
    },
    utils::script_args::{parse_member, parse_required_member},
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

    pub fn export_text(&mut self, text: String, speed: f64, speaker: String) {
        self.face_img = "".into();
        self.text = text.clone();
        self.speaker = speaker;
        self.typewriter.reset();
        self.typewriter.speed = speed;
        self.typewriter.target_text = text;
        self.typewriter.start_text = "".into();
    }

    pub fn export_tadd(&mut self, text: String, speed: f64, speaker: String) {
        self.face_img = "".into();
        self.text = text.clone();
        self.speaker = speaker;
        self.typewriter.start_text = self.typewriter.target_text.clone();
        self.typewriter.target_text = format!("{}{text}", self.typewriter.target_text);
        self.typewriter.run_time = Duration::ZERO;
        self.typewriter.speed = speed;
    }

    pub fn export_say(&mut self, speaker: String, face_img: String, text: String, speed: f64) {
        self.face_img = face_img;
        self.text = text.clone();
        self.speaker = speaker;
        self.typewriter.start_text = "".to_string();
        self.typewriter.target_text = text;
        self.typewriter.run_time = Duration::ZERO;
        self.typewriter.speed = speed;
    }

    pub fn export_sadd(&mut self, speaker: String, face_img: String, text: String, speed: f64) {
        self.face_img = face_img;
        self.text = text.clone();
        self.speaker = speaker;
        self.typewriter.start_text = self.typewriter.target_text.clone();
        self.typewriter.target_text = format!("{}{text}", self.typewriter.target_text);
        self.typewriter.run_time = Duration::ZERO;
        self.typewriter.speed = speed;
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
        self.speaker = "".into();
        self.text = "".into();
        self.typewriter.reset();
        Ok(())
    }

    fn on_end_session(&mut self, _ctx: tmj_core::script::ContextRef) -> anyhow::Result<()> {
        self.face_img = "".into();
        self.text = "".into();
        self.typewriter.reset();
        self.speaker = "".into();
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
                z_index: Z_FRAME_FACE,
                rect: Layout::ltwh2rect(area, &LAYOUT.frame_face_ltwh),
                kind: VisualElementKind::Image {
                    source: String::new(),
                    cover_method: tmj_core::img::cover::cover,
                },
                ..Default::default()
            },
            VisualElement {
                name: Self::VE_FRAME_BLOCK.to_string(),
                z_index: Z_FRAME_BLOCK,
                rect: frame_rect,
                clear_before_draw: true,
                text_wrap: Some(Wrap { trim: false }),
                kind: VisualElementKind::Fill,
                style: THEME.dialouge.block,
                ..Default::default()
            },
            VisualElement {
                name: Self::VE_FRAME_TEXT.to_string(),
                z_index: Z_FRAME_TEXT,
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
                z_index: Z_FRAME_NAME,
                rect: name_rect,
                clear_before_draw: true,
                text_wrap: Some(Wrap { trim: false }),
                kind: VisualElementKind::Text {
                    content: String::new(),
                },
                style: THEME.dialouge.name,
                text_alignment: Some(Alignment::Center),
                ..Default::default()
            },
            VisualElement {
                name: Self::VE_FRAME_SHORTKEY.to_string(),
                z_index: Z_FRAME_SHORTKEY,
                rect: short_key_rect,
                text_wrap: Some(Wrap { trim: false }),
                kind: VisualElementKind::Custom {
                    drawer: VisualElementCustomDrawer::from(draw_shortkey_bar),
                },
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
        let rev_style =
            parse_required_member(&frame, var_frame::M_REV_STYLE, ScriptValue::as_bool)?;

        let frame_show = frame
            .borrow()
            .get(var_frame::VISIBLE, None)
            .and_then(|x| x.as_bool())
            .unwrap_or(true);
        let show_all = !screen.hide_dialouge && frame_show;

        for ve in elements.iter_mut() {
            match ve.name.as_str() {
                Self::VE_FRAME_BLOCK
                | Self::VE_FRAME_TEXT
                | Self::VE_FRAME_NAME
                | Self::VE_FRAME_SHORTKEY
                | Self::VE_FACE => ve.visible = show_all,
                _ => continue,
            }
        }

        if !show_all {
            return Ok(());
        }

        for ve in elements.iter_mut() {
            match ve.name.as_str() {
                Self::VE_FRAME_BLOCK => {
                    ve.style = if rev_style {
                        THEME.dialouge.rev_inbox
                    } else {
                        THEME.dialouge.block
                    };
                }
                Self::VE_FRAME_TEXT => {
                    self.typewriter.apply_to_ve(ve)?;
                    ve.style = if rev_style {
                        THEME.dialouge.rev_inbox
                    } else {
                        THEME.dialouge.inbox
                    };
                }
                Self::VE_FACE => {
                    ve.visible = !self.face_img.is_empty();
                    if !self.face_img.is_empty()
                        && let VisualElementKind::Image { source, .. } = &mut ve.kind
                    {
                        *source = self.face_img.clone();
                    }
                }
                Self::VE_FRAME_NAME => {
                    if self.speaker.is_empty() {
                        ve.visible = false;
                    }
                    if let VisualElementKind::Text { content } = &mut ve.kind {
                        *content = self.speaker.clone();
                    }
                }
                _ => {}
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
        Span::styled(" Enter/点击 ", key),
        Span::styled("继续", desc),
        Span::styled(" s ", key),
        Span::styled("保存", desc),
        Span::styled(" l ", key),
        Span::styled("读取", desc),
        Span::styled(" c ", key),
        Span::styled("设置", desc),
        Span::styled(" Esc/q ", key),
        Span::styled("返回菜单", desc),
        Span::styled(" ↑ ", key),
        Span::styled("历史", desc),
    ]);
    Paragraph::new(line)
        .alignment(Alignment::Center)
        .render(rect, buffer);
    Ok(())
}
