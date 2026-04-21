use ratatui::widgets::Wrap;
use tmj_core::script::TypeName;

use crate::{
    LAYOUT,
    layout::Layout,
    pages::{
        pipeline::{
            logical_area,
            PipeStage,
            ve_utils::clear_animations_by_name,
            visual_element::{VisualElement, VisualElementKind},
        },
        script_def::{env::FACE_PATH, var_frame},
    },
};

#[derive(TypeName)]
pub struct FaceStage;

impl PipeStage for FaceStage {
    fn binding_vars() -> &'static [&'static str] {
        &[FACE_PATH, var_frame::FRAME]
    }
}

impl FaceStage {
    pub const VE_FACE: &'static str = "frame.face";

    pub fn build_elements() -> Vec<VisualElement> {
        let area = logical_area();
        vec![VisualElement {
            name: Self::VE_FACE.to_string(),
            z_index: 230,
            rect: Layout::ltwh2rect(area, &LAYOUT.frame_face_ltwh),
            kind: VisualElementKind::Image {
                source: String::new(),
            },
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
        let frame = vars.pop().unwrap()?.as_table().unwrap();
        let frame_show = frame
            .borrow()
            .get(var_frame::VISIBLE)
            .and_then(|x| x.as_bool())
            .unwrap_or(true);
        let binding = vars.pop().unwrap()?;
        let img_path = binding
            .as_str()
            .ok_or(anyhow::anyhow!("{FACE_PATH} should be str"))?;
        if let Some(ve) = elements.iter_mut().find(|x| x.name == Self::VE_FACE) {
            if !ve.is_animated {
                ve.rect = Layout::ltwh2rect(area, &LAYOUT.frame_face_ltwh);
                ve.visible = !screen.hide_dialouge && frame_show && !img_path.is_empty();
                if let VisualElementKind::Image { source } = &mut ve.kind {
                    *source = img_path.to_string();
                }
            }
        }
        Ok(())
    }

    pub fn stage_clear(
        _screen: &crate::pages::dialogue::DialogueScene,
        _ctx: &tmj_core::script::ContextRef,
        elements: &mut [VisualElement],
        _area: ratatui::prelude::Rect,
    ) -> anyhow::Result<()> {
        clear_animations_by_name(elements, Self::VE_FACE);
        Ok(())
    }
}
