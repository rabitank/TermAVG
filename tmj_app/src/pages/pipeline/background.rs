use ratatui::{
    layout::{Constraint, Layout},
    widgets::Wrap,
};
use tmj_core::script::TypeName;

use crate::{
    LAYOUT,
    art::theme::THEME,
    pages::{
        pipeline::{
            logical_area,
            PipeStage,
            ve_utils::clear_animations_by_prefix,
            visual_element::{VisualElement, VisualElementKind},
        },
        script_def::env::BG ,
    },
};

#[derive(TypeName)]
pub struct BackgroundStage;

impl PipeStage for BackgroundStage {
    fn binding_vars() -> &'static [&'static str] {
        &[BG, Self::BG_IMAGE, Self::BG_IS_EDGE]
    }

}

impl BackgroundStage {
    pub const BG_IMAGE: &'static str = constcat::concat!(BG, ".", crate::pages::script_def::var_bg::IMAGE);
    pub const BG_IS_EDGE: &'static str = constcat::concat!(BG, ".", crate::pages::script_def::var_bg::IS_EDGE);
    pub const VE_BG: &'static str = Self::BG_IMAGE;
    pub const VE_EDGE_TOP: &'static str = "bg.edge.top";
    pub const VE_EDGE_BOTTOM: &'static str = "bg.edge.bottom";

    pub fn build_elements(ctx: &tmj_core::script::ContextRef) -> anyhow::Result<Vec<VisualElement>> {
        let mut args = Self::get_script_vars(ctx);
        let is_edge_show = args.pop().unwrap()?.as_bool().unwrap();
        let bg_image=  args.pop().unwrap()?.as_string().unwrap().clone();
        let area = logical_area();
        
        let [up, _, down] = area.layout(&Layout::vertical([
            Constraint::Length(LAYOUT.vertical_dark_edge),
            Constraint::Fill(1),
            Constraint::Length(LAYOUT.vertical_dark_edge),
        ]));

        Ok(vec![
            VisualElement {
                name: Self::VE_BG.to_string(),
                z_index: 0,
                rect: area,
                text_wrap: Some(Wrap { trim: false }),
                kind: VisualElementKind::Image {
                    source: bg_image.to_string(),
                },
                style: THEME.dialouge.background,
                ..Default::default()
            },
            VisualElement {
                name: Self::VE_EDGE_TOP.to_string(),
                visible: is_edge_show,
                z_index: 5,
                rect: up,
                clear_before_draw: true,
                text_wrap: Some(Wrap { trim: false }),
                kind: VisualElementKind::Fill,
                style: THEME.dialouge.black_edge,
                ..Default::default()
            },
            VisualElement {
                name: Self::VE_EDGE_BOTTOM.to_string(),
                visible: is_edge_show,
                z_index: 5,
                rect: down,
                clear_before_draw: true,
                text_wrap: Some(Wrap { trim: false }),
                kind: VisualElementKind::Fill,
                style: THEME.dialouge.black_edge,
                ..Default::default()
            },
        ])
    }

    pub fn update_elements(
        ctx: &tmj_core::script::ContextRef,
        elements: &mut [VisualElement],
    ) -> anyhow::Result<()> {
        let area = logical_area();
        let mut vars = Self::get_script_vars(ctx);
        let use_v_edge = vars.pop().unwrap()?;
        let bgimg_path = vars.pop().unwrap()?;
        if let Some(bg) = elements.iter_mut().find(|x| x.name == Self::VE_BG) {
            if !bg.is_animated {
                bg.rect = area;
                if let VisualElementKind::Image { source } = &mut bg.kind {
                    *source = bgimg_path.as_string().cloned().unwrap_or_default();
                }
            }
        if bgimg_path.is_nil() || bgimg_path.as_str().unwrap_or("").is_empty(){
            bg.fill_before_draw = true;
        } else {
            bg.fill_before_draw = false;
        }
        }
        
        let edge_visible = use_v_edge.as_bool().unwrap_or(true);
        if let Some(top) = elements.iter_mut().find(|x| x.name == Self::VE_EDGE_TOP) {
            if !top.is_animated {
                //@todo 
            }
            top.visible = edge_visible;
        }
        if let Some(bottom) = elements.iter_mut().find(|x| x.name == Self::VE_EDGE_BOTTOM) {
            if !bottom.is_animated {
                //@todo
            }
            bottom.visible = edge_visible;
        }
        Ok(())
    }

    pub fn stage_clear(
        _ctx: &tmj_core::script::ContextRef,
        elements: &mut [VisualElement],
        _area: ratatui::prelude::Rect,
    ) -> anyhow::Result<()> {
        clear_animations_by_prefix(elements, "bg.");
        Ok(())
    }
}
