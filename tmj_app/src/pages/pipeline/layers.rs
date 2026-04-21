use ratatui::widgets::Wrap;
use tmj_core::script::TypeName;

use crate::pages::{
    pipeline::{
        logical_area,
        PipeStage,
        ve_utils::clear_animations_by_prefix,
        visual_element::{VisualElement, VisualElementKind},
    },
    script_def::{env::LAYERS, var_layer},
};

#[derive(TypeName)]
pub struct LayersStage;

impl PipeStage for LayersStage {
    fn binding_vars() -> &'static [&'static str] {
        &[LAYERS]
    }

}

impl LayersStage {
    pub fn build_elements(ctx: &tmj_core::script::ContextRef) -> anyhow::Result<Vec<VisualElement>> {
        let layers = Self::get_script_vars(ctx)
            .pop()
            .unwrap()?
            .as_table()
            .ok_or(anyhow::anyhow!("{LAYERS} should be table"))?;
        let items: Vec<_> = layers
            .borrow()
            .iter()
            .map(|(name, val)| (name.clone(), val.clone()))
            .collect();
        let mut out = Vec::new();
        for (name, val) in items {
            let layer = match val.as_table() {
                Some(v) => v,
                None => continue,
            };
            let layer_type = layer
                .borrow()
                .get(var_layer::LAYER_TYPE)
                .and_then(|x| x.as_str().map(|s| s.to_string()))
                .unwrap_or_default();
            if layer_type != "image" {
                continue;
            }
            out.push(VisualElement {
                name: format!("layer.{name}"),
                z_index: 400,
                rect: ratatui::layout::Rect::new(0, 0, 0, 0),
                kind: VisualElementKind::Image {
                    source: String::new(),
                },
                ..Default::default()
            });
        }
        Ok(out)
    }

    pub fn update_elements(
        ctx: &tmj_core::script::ContextRef,
        elements: &mut [VisualElement],
    ) -> anyhow::Result<()> {
        let area = logical_area();
        let layers = Self::get_script_vars(ctx)
            .pop()
            .unwrap()?
            .as_table()
            .ok_or(anyhow::anyhow!("{LAYERS} should be table"))?;
        let items: Vec<_> = layers
            .borrow()
            .iter()
            .map(|(name, val)| (name.clone(), val.clone()))
            .collect();

        for (name, val) in items {
            let layer = match val.as_table() {
                Some(v) => v,
                None => continue,
            };
            let ve_name = format!("layer.{name}");
            let visible = layer
                .borrow()
                .get(var_layer::VISIBLE)
                .and_then(|x| x.as_bool())
                .unwrap_or(true);
            let layer_type = layer
                .borrow()
                .get(var_layer::LAYER_TYPE)
                .and_then(|x| x.as_str().map(|s| s.to_string()))
                .unwrap_or_default();
            if let Some(ve) = elements.iter_mut().find(|x| x.name == ve_name) {
                if layer_type != "image" {
                    ve.visible = false;
                    continue;
                }
                if !ve.is_animated {
                    ve.visible = visible;
                    ve.rect = area;
                    if let VisualElementKind::Image { source } = &mut ve.kind {
                        *source = layer
                            .borrow()
                            .get(var_layer::SOURCE)
                            .and_then(|x| x.as_str().map(|s| s.to_string()))
                            .unwrap_or_default();
                    }
                }
            }
        }
        Ok(())
    }

    pub fn stage_clear(
        _ctx: &tmj_core::script::ContextRef,
        elements: &mut [VisualElement],
        _area: ratatui::prelude::Rect,
    ) -> anyhow::Result<()> {
        clear_animations_by_prefix(elements, "layer.");
        Ok(())
    }
}
