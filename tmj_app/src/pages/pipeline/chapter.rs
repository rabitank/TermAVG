use constcat;
use ratatui::{layout::Alignment, widgets::Wrap};
use tmj_core::script::{ContextRef, TypeName};

use crate::{
    art::theme::THEME,
    pages::{
        dialogue::DialogueScene,
        pipeline::{
            Behaviour,
            animation::{Animation, alpha_shift::AniAlpha},
            logical_area,
            visual_element::{VisualElement, VisualElementKind},
        },
        script_def::env::CHAPTER,
    },
};

#[derive(TypeName)]
pub struct ChapterBehaviour {
    title: String,
    subtitle: String,
    title_alpha_ani: AniAlpha,
    subtitle_alpha_ani: AniAlpha,
}

impl Default for ChapterBehaviour {
    fn default() -> Self {
        Self {
            title: Default::default(),
            subtitle: Default::default(),
            title_alpha_ani: Default::default(),
            subtitle_alpha_ani: Default::default(),
        }
    }
}

impl ChapterBehaviour {
    fn export_show_title(&mut self, show_time: std::time::Duration, title: String) {
        self.title = title;
        self.title_alpha_ani.reset();
        self.title_alpha_ani.start_alpha = 0.0;
        self.title_alpha_ani.target_alpha = 1.0;
        self.title_alpha_ani.anim_time = show_time;
    }

    fn export_show_sub_title(&mut self, show_time: std::time::Duration, title: String) {
        self.subtitle = title;
        self.subtitle_alpha_ani.reset();
        self.subtitle_alpha_ani.start_alpha = 0.0;
        self.subtitle_alpha_ani.target_alpha = 1.0;
        self.subtitle_alpha_ani.anim_time = show_time;
    }
}

impl Behaviour for ChapterBehaviour {
    fn binding_vars(&self) -> &'static [&'static str] {
        &[]
    }

    fn is_animating(&self) -> bool {
        self.title_alpha_ani.is_animing() || self.subtitle_alpha_ani.is_animing()
    }

    fn build_elements(
        &self,
        _ctx: &tmj_core::script::ContextRef,
    ) -> anyhow::Result<Vec<VisualElement>> {
        let area = logical_area();
        let title_rect = crate::layout::Layout::ltwh2rect(area, &crate::LAYOUT.chapter_title_ltwh);
        let subtitle_rect =
            crate::layout::Layout::ltwh2rect(area, &crate::LAYOUT.chapter_subtitle_ltwh);

        Ok(vec![
            VisualElement {
                name: Self::CHAPTER_TITLE.to_string(),
                visible: true,
                alpha: 0.0,
                z_index: 1,
                rect: title_rect,
                text_alignment: Some(Alignment::Center),
                text_wrap: Some(Wrap { trim: false }),
                kind: VisualElementKind::Text { content: "".into() },
                style: THEME.dialouge.charpter_title,
                ..Default::default()
            },
            VisualElement {
                name: Self::CHAPTER_SUBTITLE.to_string(),
                visible: true,
                alpha: 0.0,
                z_index: 2,
                rect: subtitle_rect,
                text_alignment: Some(Alignment::Center),
                text_wrap: Some(Wrap { trim: false }),
                kind: VisualElementKind::Text { content: "".into() },
                style: THEME.dialouge.charpter_subtitle,
                ..Default::default()
            },
        ])
    }

    fn update_elements(
        &self,
        _screen: &DialogueScene,
        ctx: &tmj_core::script::ContextRef,
        elements: &mut Vec<VisualElement>,
    ) -> anyhow::Result<()> {

        if let Some(title) = elements.iter_mut().find(|x| x.name == Self::CHAPTER_TITLE) {
            self.title_alpha_ani.apply_to_ve(title);

            if self.title.is_empty() {
                title.visible = false;
            } else {
                if let VisualElementKind::Text { content } = &mut title.kind {
                    *content = self.title.clone();
                }
            }
        }

        if let Some(subtitle) = elements
            .iter_mut()
            .find(|x| x.name == Self::CHAPTER_SUBTITLE)
        {
            self.subtitle_alpha_ani.apply_to_ve(subtitle);
            if self.subtitle.is_empty() {
                subtitle.visible = false;
            } else {
                if let VisualElementKind::Text { content } = &mut subtitle.kind {
                    *content = self.subtitle.clone();
                }
            }
        }
        Ok(())
    }

    fn tick_update(&mut self, _ctx: ContextRef, delta_time: std::time::Duration) {
        self.title_alpha_ani.update(delta_time);
        self.subtitle_alpha_ani.update(delta_time);
    }

    fn on_force_over_animation(&mut self) -> anyhow::Result<()> {
        self.title_alpha_ani.force_over();
        self.subtitle_alpha_ani.force_over();
        Ok(())
    }

    fn on_end_dialouge(&mut self) -> anyhow::Result<()> {
        self.title = "".into();
        self.subtitle = "".into();
        self.title_alpha_ani.reset();
        self.subtitle_alpha_ani.reset();
        Ok(())
    }

    fn on_end_session(&mut self, _ctx: tmj_core::script::ContextRef) -> anyhow::Result<()> {
        self.title = "".into();
        self.subtitle = "".into();
        self.title_alpha_ani.target_alpha = 0.0;
        self.subtitle_alpha_ani.target_alpha = 0.0;
        self.title_alpha_ani.reset();
        self.subtitle_alpha_ani.reset();
        Ok(())
    }
}

impl ChapterBehaviour {
    pub const CHAPTER_TITLE: &'static str =
        constcat::concat!(CHAPTER, ".", crate::pages::script_def::var_chapter::TITLE);
    pub const CHAPTER_SUBTITLE: &'static str = constcat::concat!(
        CHAPTER,
        ".",
        crate::pages::script_def::var_chapter::SUBTITLE
    );
    pub const CHAPTER_ALPHA: &'static str =
        constcat::concat!(CHAPTER, ".", crate::pages::script_def::var_chapter::ALPHA);
    pub const CHAPTER_ALPHA_SPEED: &'static str = constcat::concat!(
        CHAPTER,
        ".",
        crate::pages::script_def::var_chapter::ALPHA_SPEED
    );
}
