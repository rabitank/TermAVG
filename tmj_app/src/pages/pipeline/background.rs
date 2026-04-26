use std::{path::PathBuf, time};

use ratatui::{
    layout::{Constraint, Layout},
    widgets::Wrap,
};
use tmj_core::{
    pathes,
    script::{ContextRef, TypeName},
};

use crate::{
    LAYOUT,
    art::theme::THEME,
    pages::{
        dialogue::DialogueScene,
        pipeline::{
            Behaviour,
            animation::{Animation, img_trans::AniImgTrans},
            logical_area,
            visual_element::{VisualElement, VisualElementCustomDrawer, VisualElementKind},
        },
        script_def::env::BG,
    },
};

#[derive(TypeName, Default)]
pub struct BackgroundBehaviour {
    is_edge: bool,
    img_trans_ani: AniImgTrans,
}

impl BackgroundBehaviour {
    fn trans_string_path(s: String) -> Option<PathBuf> {
        if s.is_empty(){
            None
        } else {
            Some(pathes::path(s))
        }
    }
    pub fn export_trans_to(&mut self, new_img_path: String, duration: f64) {
        self.img_trans_ani.old_image = self.img_trans_ani.new_image.clone();
        self.img_trans_ani.new_image = Self::trans_string_path(new_img_path.clone());
        self.img_trans_ani.anim_time = time::Duration::from_secs_f64(duration);
        self.img_trans_ani.run_time = time::Duration::ZERO;
    }
    pub fn export_set(&mut self, new_img_path: String) {
        self.img_trans_ani.old_image = self.img_trans_ani.new_image.clone();
        self.img_trans_ani.new_image = Self::trans_string_path(new_img_path.clone());
        self.img_trans_ani.anim_time = time::Duration::ZERO;
        self.img_trans_ani.run_time = time::Duration::ZERO;
    }

    pub fn export_show_edge(&mut self) {
        self.is_edge = true;
    }

    pub fn export_hide_edge(&mut self) {
        self.is_edge = false;
    }
}

impl Behaviour for BackgroundBehaviour {
    fn is_animating(&self) -> bool {
        self.img_trans_ani.is_animing()
    }
    fn on_scene_active(&mut self, ctx: tmj_core::script::ContextRef) -> anyhow::Result<()> {
        let mut vars = self.get_bind_vars(&ctx);
        self.is_edge = vars.pop().unwrap()?.as_bool().unwrap();
        let img_path = vars.pop().unwrap()?.as_string().unwrap().clone();
        self.img_trans_ani.reset();
        self.img_trans_ani.new_image = Self::trans_string_path(img_path);
        Ok(())
    }

    fn binding_vars(&self) -> &'static [&'static str] {
        &[BG, Self::BG_IMAGE, Self::BG_IS_EDGE]
    }

    fn build_elements(
        &self,
        ctx: &tmj_core::script::ContextRef,
    ) -> anyhow::Result<Vec<VisualElement>> {
        let mut args = self.get_bind_vars(ctx);
        let is_edge_show = args.pop().unwrap()?.as_bool().unwrap();
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
                kind: VisualElementKind::Custom {
                    drawer: VisualElementCustomDrawer::from(|_, _, _| Ok(())),
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

    fn update_elements(
        &self,
        _screen: &DialogueScene,
        _ctx: &tmj_core::script::ContextRef,
        elements: &mut Vec<VisualElement>,
    ) -> anyhow::Result<()> {
        if let Some(bg) = elements.iter_mut().find(|x| x.name == Self::VE_BG) {
            self.img_trans_ani.apply_to_ve(bg);
        }
        if let Some(top) = elements.iter_mut().find(|x| x.name == Self::VE_EDGE_TOP) {
            top.visible = self.is_edge;
        }
        if let Some(bottom) = elements.iter_mut().find(|x| x.name == Self::VE_EDGE_BOTTOM) {
            bottom.visible = self.is_edge;
        }

        Ok(())
    }

    fn tick_update(&mut self, _ctx: ContextRef, delta_time: std::time::Duration) {
        self.img_trans_ani.update(delta_time);
    }

    fn on_force_over_animation(&mut self) -> anyhow::Result<()> {
        self.img_trans_ani.force_over();
        Ok(())
    }

    fn on_end_dialouge(&mut self) -> anyhow::Result<()> {
        self.img_trans_ani.reset();
        Ok(())
    }

    fn on_end_session(&mut self, _ctx: tmj_core::script::ContextRef) -> anyhow::Result<()> {
        self.img_trans_ani.reset();
        Ok(())
    }
}

impl BackgroundBehaviour {
    pub const BG_IMAGE: &'static str =
        constcat::concat!(BG, ".", crate::pages::script_def::var_bg::M_IMAGE);
    pub const BG_IS_EDGE: &'static str =
        constcat::concat!(BG, ".", crate::pages::script_def::var_bg::M_IS_EDGE);
    pub const VE_BG: &'static str = Self::BG_IMAGE;
    pub const VE_EDGE_TOP: &'static str = "bg.edge.top";
    pub const VE_EDGE_BOTTOM: &'static str = "bg.edge.bottom";
}
