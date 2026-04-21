use ratatui::{buffer::Buffer, layout::Rect};
use tmj_core::script::TypeName;

use crate::pages::pipeline::visual_element::VisualElement;

#[derive(TypeName)]
pub struct RenderVeStage;

impl RenderVeStage {
    pub fn draw<'a>(
        elements: &mut [VisualElement],
        buffer: &'a mut Buffer,
        delta_secs: f64,
        _area: Rect,
    ) -> anyhow::Result<&'a mut Buffer> {
        elements.sort_by_key(|e| e.z_index);
        for ve in elements.iter_mut() {
            ve.update_animation(delta_secs);
            ve.render(buffer, _area)?;
        }
        Ok(buffer)
    }
}
