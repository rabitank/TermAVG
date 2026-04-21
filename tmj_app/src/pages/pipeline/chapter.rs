use constcat;
use ratatui::{layout::Alignment, widgets::Wrap};
use tmj_core::script::TypeName;

use crate::{
    art::theme::THEME,
    pages::{
        pipeline::{
            logical_area,
            PipeStage,
            visual_element::{VisualElement, VisualElementKind},
        },
        script_def::env::CHAPTER,
    },
};

#[derive(TypeName)]
pub struct ChapterStage {}

impl PipeStage for ChapterStage {
    fn binding_vars() -> &'static [&'static str] {
        &[
            Self::CHAPTER_TITLE,
            Self::CHAPTER_SUBTITLE,
            Self::CHAPTER_ALPHA,
            Self::CHAPTER_ALPHA_SPEED,
        ]
    }
}

impl ChapterStage {
    pub const CHAPTER_TITLE: &'static str =
        constcat::concat!(CHAPTER, ".", crate::pages::script_def::var_chapter::TITLE);
    pub const CHAPTER_SUBTITLE: &'static str = constcat::concat!(
        CHAPTER,
        ".",
        crate::pages::script_def::var_chapter::SUBTITLE
    );
    pub const CHAPTER_ALPHA: &'static str =
        constcat::concat!(CHAPTER, ".", crate::pages::script_def::var_chapter::ALPHA);
        pub const CHAPTER_ALPHA_SPEED: &'static str =
        constcat::concat!(CHAPTER, ".", crate::pages::script_def::var_chapter::ALPHA_SPEED);

    pub fn build_elements(ctx: &tmj_core::script::ContextRef) -> anyhow::Result<Vec<VisualElement>> {
        let mut args = Self::get_script_vars(ctx);

        let alpha_speed = args.pop().unwrap()?.as_float().unwrap();
        let alpha = args.pop().unwrap()?.as_float().unwrap();
        let subtitle_content = args.pop().unwrap()?.as_string().unwrap().clone();
        let title_content= args.pop().unwrap()?.as_string().unwrap().clone();

        let area = logical_area();
        let title_rect = crate::layout::Layout::ltwh2rect(area, &crate::LAYOUT.chapter_title_ltwh);
        let subtitle_rect =
            crate::layout::Layout::ltwh2rect(area, &crate::LAYOUT.chapter_subtitle_ltwh);

        Ok(vec![
            VisualElement {
                name: Self::CHAPTER_TITLE.to_string(),
                visible: true,
                alpha,
                alpha_speed,
                z_index: 1,
                rect: title_rect,
                text_alignment: Some(Alignment::Center),
                text_wrap: Some(Wrap { trim: false }),
                kind: VisualElementKind::Text { content: title_content.into() },
                style: THEME.dialouge.charpter_title,
                ..Default::default()
            },
            VisualElement {
                name: Self::CHAPTER_SUBTITLE.to_string(),
                visible: true,
                alpha,
                alpha_speed,
                z_index: 2,
                rect: subtitle_rect,
                text_alignment: Some(Alignment::Center),
                text_wrap: Some(Wrap { trim: false }),
                kind: VisualElementKind::Text { content: subtitle_content.into() },
                style: THEME.dialouge.charpter_subtitle,
                ..Default::default()
            },
        ])
    }

    pub fn update_elements(
        ctx: &tmj_core::script::ContextRef,
        elements: &mut [VisualElement],
    ) -> anyhow::Result<()> {
        let mut vars = Self::get_script_vars(ctx);
        let alpha_speed = vars.pop().unwrap()?.as_float().unwrap();
        let alpha = vars.pop().unwrap()?.as_float().unwrap();
        let subtitle_content = vars.pop().unwrap()?;
        let title_content = vars.pop().unwrap()?;
        if let Some(title) = elements.iter_mut().find(|x| x.name == Self::CHAPTER_TITLE) {
            if !title.is_animated {
                if let VisualElementKind::Text { content } = &mut title.kind {
                    *content = title_content
                        .as_string()
                        .ok_or(anyhow::anyhow!("title content should be str"))?
                        .to_string();
                    title.alpha = alpha;
                    title.alpha_speed = alpha_speed;
                }
            }

        }

        let subtitle_content = subtitle_content
            .as_string()
            .ok_or(anyhow::anyhow!("subtitle content should be str"))?
            .to_string();
        if let Some(subtitle_ve) = elements
            .iter_mut()
            .find(|x| x.name == Self::CHAPTER_SUBTITLE)
        {
            if !subtitle_ve.is_animated {
                if let VisualElementKind::Text { content } = &mut subtitle_ve.kind {
                    *content = subtitle_content;
                    subtitle_ve.alpha = alpha;
                    subtitle_ve.alpha_speed = alpha_speed;
                }
            }

        }
        Ok(())
    }
}
