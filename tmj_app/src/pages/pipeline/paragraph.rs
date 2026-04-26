use ratatui::{
    layout::Margin,
    widgets::{BorderType, Wrap},
};
use tmj_core::script::TypeName;

use crate::{
    art::theme::THEME,
    pages::{
        dialogue::DialogueScene,
        pipeline::{
            Behaviour,
            animation::{Animation, typewriter::AniTypeWriter},
            logical_area,
            visual_element::{VisualElement, VisualElementKind},
        },
        script_def::{env::PARAGRAPH, var_paragraph::{M_CONTENT, M_VISIBLE}},
    },
};

#[derive(TypeName, Default)]
pub struct ParagraphBehaviour {
    typewriter_ani: AniTypeWriter,
}

impl ParagraphBehaviour {
    pub fn export_clear(&mut self) {
        self.typewriter_ani.reset();
    }

    pub fn export_print(&mut self, append_string: &String) {
        self.typewriter_ani.start_text = self.typewriter_ani.target_text.clone();
        self.typewriter_ani.target_text = self.typewriter_ani.start_text.clone() + append_string;
        self.typewriter_ani.speed = 40.0;
        self.typewriter_ani.run_time = std::time::Duration::ZERO;
    }

    pub fn export_new(&mut self, new_string: &String) {
        self.typewriter_ani.start_text = "".into();
        self.typewriter_ani.target_text = new_string.clone();
        self.typewriter_ani.run_time = std::time::Duration::ZERO;
        self.typewriter_ani.speed = 40.0;
    }
}

impl Behaviour for ParagraphBehaviour {
    fn tick_update(&mut self, _ctx: tmj_core::script::ContextRef, delta_time: std::time::Duration) {
        self.typewriter_ani.update(delta_time);
    }

    fn is_animating(&self) -> bool {
        self.typewriter_ani.is_animing()
    }
    fn on_scene_active(&mut self, _ctx: tmj_core::script::ContextRef) -> anyhow::Result<()> {
        let content = _ctx.borrow().get_val(Self::PARAGRAPH_CONTENT).ok_or(anyhow::anyhow!("paragraph content get failed"))?;
        let content = content.as_str().unwrap();
        self.typewriter_ani.target_text = content.to_string();
        self.typewriter_ani.start_text = content.to_string();
        self.typewriter_ani.run_time = std::time::Duration::ZERO;
        Ok(())
    }

    fn binding_vars(&self) -> &'static [&'static str] {
        &[Self::PARAGRAPH_VISIBLE]
    }

    fn build_elements(
        &self,
        _ctx: &tmj_core::script::ContextRef,
    ) -> anyhow::Result<Vec<VisualElement>> {
        let area = logical_area();
        let rect = crate::layout::Layout::ltwh2rect(area, &crate::LAYOUT.paragraph_ltwh);
        let rect = rect.inner(Margin::new(2, 2));
        let border_style = THEME.borders;
        let border_type= BorderType::Rounded;
        Ok(vec![VisualElement {
            name: Self::VE_PARAGRAPH_TEXT.to_string(),
            visible: false,
            z_index: 300,
            rect,
            clear_before_draw: false,
            alpha: 0.85,
            border: true,
            border_type,
            border_style,
            use_typewriter: true,
            typewriter_speed: 60.0,
            text_wrap: Some(Wrap { trim: false}),
            kind: VisualElementKind::Text {
                content: String::new(),
            },
            style: THEME.content,
            ..Default::default()
        }])
    }

    fn update_elements(
        &self,
        screen: &DialogueScene,
        ctx: &tmj_core::script::ContextRef,
        elements: &mut Vec<VisualElement>,
    ) -> anyhow::Result<()> {
        let mut vars = self.get_bind_vars(ctx);
        let paragraph_show = vars.pop().unwrap()?.as_bool().unwrap();
        let visible = !screen.hide_dialouge && paragraph_show;
        if let Some(ve) = elements
            .iter_mut()
            .find(|x| x.name == Self::VE_PARAGRAPH_TEXT)
        {
            ve.visible = visible;
            self.typewriter_ani.apply_to_ve(ve)?;
        }
        Ok(())
    }

    fn on_force_over_animation(&mut self) -> anyhow::Result<()> {
        self.typewriter_ani.force_over();
        Ok(())
    }

    fn on_end_dialouge(&mut self) -> anyhow::Result<()> {
        self.typewriter_ani.start_text = "".into();
        self.typewriter_ani.target_text = "".into();
        Ok(())
    }

    fn on_end_session(&mut self, _ctx: tmj_core::script::ContextRef) -> anyhow::Result<()> {
        self.typewriter_ani.start_text = self.typewriter_ani.target_text.clone();
        Ok(())
    }
}

impl ParagraphBehaviour {
    pub const VE_PARAGRAPH_TEXT: &'static str = "paragraph.text";
    pub const PARAGRAPH_VISIBLE: &'static str = constcat::concat!(PARAGRAPH, ".", M_VISIBLE);
    pub const PARAGRAPH_CONTENT: &'static str = constcat::concat!(PARAGRAPH, ".", M_CONTENT);
}
