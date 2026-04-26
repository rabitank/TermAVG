use std::{any::Any, collections::HashMap};

use tmj_core::script::TypeName;

use crate::pages::{
    dialogue::DialogueScene,
    pipeline::{
        Behaviour,
        animation::{Animation, AnyAnimation},
        logical_area,
        visual_element::{VisualElement, VisualElementKind},
    },
    script_def::{env::LAYERS, var_layer},
};

trait AniCast {
    fn get_ani<T>(&self, name: &String) -> anyhow::Result<&T>
    where
        T: Animation + Any;
    fn get_ani_mut<T>(&mut self, name: &String) -> anyhow::Result<&mut T>
    where
        T: Animation + Any;
}

pub type AnyAnimationMap = HashMap<String, Box<dyn AnyAnimation>>;

impl AniCast for HashMap<String, Box<dyn AnyAnimation>> {
    fn get_ani<T>(&self, name: &String) -> anyhow::Result<&T>
    where
        T: Animation + Any,
    {
        let res = self
            .get(name)
            .ok_or(anyhow::anyhow!("{name} not in Animation Map"))?;
        let r = res.as_ref() as &dyn Any;
        r.downcast_ref::<T>().ok_or(anyhow::anyhow!(
            "{name} Animation is not a {} instance",
            std::any::type_name::<T>()
        ))
    }

    fn get_ani_mut<T>(&mut self, name: &String) -> anyhow::Result<&mut T>
    where
        T: Animation + Any,
    {
        let res = self
            .get_mut(name)
            .ok_or(anyhow::anyhow!("{name} not in Animation Map"))?;
        let r = res.as_mut() as &mut dyn Any;
        r.downcast_mut::<T>().ok_or(anyhow::anyhow!(
            "{name} Animation is not a {} instance",
            std::any::type_name::<T>()
        ))
    }
}

#[derive(TypeName, Default)]
pub struct LayerBehaviour {
    anim_effect_map: HashMap<String, Box<dyn Animation>>,
}

impl Behaviour for LayerBehaviour {
    fn binding_vars(&self) -> &'static [&'static str] {
        &[LAYERS]
    }

    fn build_elements(
        &self,
        ctx: &tmj_core::script::ContextRef,
    ) -> anyhow::Result<Vec<VisualElement>> {
        let layers = self
            .get_bind_vars(ctx)
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

    fn update_elements(
        &self,
        _screen: &DialogueScene,
        ctx: &tmj_core::script::ContextRef,
        elements: &mut Vec<VisualElement>,
    ) -> anyhow::Result<()> {
        let area = logical_area();
        let layers = self
            .get_bind_vars(ctx)
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
        Ok(())
    }

    fn on_force_over_animation(&mut self) -> anyhow::Result<()> {
        Ok(())
    }

    fn on_end_dialouge(&mut self) -> anyhow::Result<()> {
        Ok(())
    }

    fn on_end_session(&mut self, _ctx: tmj_core::script::ContextRef) -> anyhow::Result<()> {
        Ok(())
    }
}
