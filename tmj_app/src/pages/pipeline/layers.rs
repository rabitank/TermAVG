use ratatui::widgets::Widget;
use tmj_core::{img::shape::Pic, pathes, script::TypeName};

use crate::pages::{
    pipeline::PipeStage,
    script_def::{env::LAYERS, var_layer},
};

#[derive(TypeName)]
pub struct LayersStage;

impl PipeStage for LayersStage {
    fn binding_vars() -> &'static [&'static str] {
        &[LAYERS]
    }

    fn draw<'a>(
        _screen: &crate::pages::dialogue::DialogueScene,
        ctx: &tmj_core::script::ContextRef,
        buffer: &'a mut ratatui::prelude::Buffer,
        area: ratatui::prelude::Rect,
    ) -> anyhow::Result<&'a mut ratatui::prelude::Buffer> {
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

        for (_name, val) in items {
            let layer = match val.as_table() {
                Some(v) => v,
                None => continue,
            };
            let layer_type = layer
                .borrow()
                .get(var_layer::LAYER_TYPE)
                .and_then(|x| x.as_str().map(|s| s.to_string()))
                .unwrap_or_default();
            let visible = layer
                .borrow()
                .get(var_layer::VISIBLE)
                .and_then(|x| x.as_bool())
                .unwrap_or(true);
            if !visible {
                continue;
            }

            if layer_type == "image" {
                let source = layer
                    .borrow()
                    .get(var_layer::SOURCE)
                    .and_then(|x| x.as_str().map(|s| s.to_string()))
                    .unwrap_or_default();
                if source.is_empty() {
                    continue;
                }
                let path = pathes::path(source);
                if !path.exists() {
                    continue;
                }
                let pic = Pic::from(path)?;
                pic.render(area, buffer);
            }
        }

        Ok(buffer)
    }
}
