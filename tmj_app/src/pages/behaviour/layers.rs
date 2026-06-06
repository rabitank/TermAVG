use std::cell::RefCell;

use tmj_core::img::cover::cover;
use tmj_core::script::{ScriptValue, TableRef, TypeName};

use crate::{
    pages::{
        behaviour::{
            Behaviour,
            animation::{
                VeAniMap, VeTypedAnimationMap, alpha_shift::AniAlpha,
                bytes_stream::EffectBytesStream, error::EffectError, glitch::EffectGlitch, heart_beat::EffectHeartBeat,
            },
            visual_element::{VisualElement, VisualElementKind},
        },
        dialogue::DialogueScene,
        script_def::{env::LAYER_LS, layer},
    },
    utils::script_args::{parse_member, parse_required_member},
};

#[derive(TypeName, Default)]
pub struct LayerBehaviour {
    layer_ves_anim_map: RefCell<VeTypedAnimationMap>,
}

impl LayerBehaviour {
    pub fn export_show(
        &mut self,
        layer: &TableRef,
        duration: std::time::Duration,
    ) -> anyhow::Result<()> {
        let ve_name = LayerBehaviour::get_layer_ve_name(layer)?;
        self.layer_ves_anim_map
            .borrow_mut()
            .insert_ani(&ve_name, AniAlpha::new(0.0, 1.0, duration));
        Ok(())
    }

    pub fn export_hide(
        &mut self,
        layer: &TableRef,
        duration: std::time::Duration,
    ) -> anyhow::Result<()> {
        let ve_name = LayerBehaviour::get_layer_ve_name(layer)?;
        self.layer_ves_anim_map
            .borrow_mut()
            .insert_ani(&ve_name, AniAlpha::new(1.0, 0.0, duration));
        Ok(())
    }

    fn get_layer_rect(layer: &TableRef) -> anyhow::Result<ratatui::layout::Rect> {
        let x = parse_required_member(layer, layer::X, ScriptValue::as_int)?;
        let y = parse_required_member(layer, layer::Y, ScriptValue::as_int)?;
        let w = parse_required_member(layer, layer::W, ScriptValue::as_int)?;
        let h = parse_required_member(layer, layer::H, ScriptValue::as_int)?;
        Ok(ratatui::layout::Rect::new(
            x.try_into()?,
            y.try_into()?,
            w.try_into()?,
            h.try_into()?,
        ))
    }

    pub fn export_reset(&self, layer: &TableRef) -> anyhow::Result<()> {
        let ve_name = Self::get_layer_ve_name(layer)?;
        if let Some(anim_map) = self.layer_ves_anim_map.borrow_mut().get_mut(&ve_name) {
            for (_, ani) in anim_map.iter_mut() {
                ani.reset();
            }
        }
        Ok(())
    }

    fn get_layer_ve_name(layer: &TableRef) -> anyhow::Result<String> {
        let name = parse_required_member(layer, layer::NAME, ScriptValue::as_string)?;
        Ok(format!("layer.{name}"))
    }

    fn apply_layer_base_prop(layer: &TableRef, ve: &mut VisualElement) -> anyhow::Result<()> {
        let z_deep = parse_required_member(layer, layer::Z_DEEP, ScriptValue::as_int)?;
        let visible = parse_required_member(layer, layer::M_VISIBLE, ScriptValue::as_bool)?;
        let area = LayerBehaviour::get_layer_rect(layer)?;

        ve.z_index = z_deep.try_into().unwrap();
        ve.visible = visible;
        ve.rect = area;
        Ok(())
    }

    fn collect_layer_effect_anim(
        &self,
        _layer: &TableRef,
        ve_name: &String,
        effect_type: &String,
    ) -> anyhow::Result<()> {
        match effect_type.to_ascii_lowercase().as_str() {
            "error" => {
                self.layer_ves_anim_map
                    .borrow_mut()
                    .insert_ani(ve_name, EffectError::default());
            }
            "bytestream" | "bytes_stream" => {
                self.layer_ves_anim_map
                    .borrow_mut()
                    .insert_ani(ve_name, EffectBytesStream::default());
            }
            "glitch" => {
                self.layer_ves_anim_map
                    .borrow_mut()
                    .insert_ani(ve_name, EffectGlitch::default());
            }
            "heartbeat" | "heart_beat" => {
                self.layer_ves_anim_map
                    .borrow_mut()
                    .insert_ani(ve_name, EffectHeartBeat::default());
            }
            _ => {
                tracing::error!("unknow effect type {}", effect_type);
            }
        }
        Ok(())
    }

    fn build_ve_from_layer(
        &self,
        layer: &TableRef,
        _ctx: tmj_core::script::ContextRef,
    ) -> anyhow::Result<VisualElement> {
        let data = parse_required_member(layer, layer::DATA, ScriptValue::as_string)?;
        let layer_type = parse_required_member(layer, layer::LAYER_TYPE, ScriptValue::as_string)?;

        let mut ve = VisualElement::default();
        let name = parse_required_member(layer, layer::NAME, ScriptValue::as_string)?;
        ve.name = format!("layer.{name}");

        LayerBehaviour::apply_layer_base_prop(&layer, &mut ve)?;

        if layer_type == "effect" {
            tracing::info!("create new effect layer {}", data);
            self.collect_layer_effect_anim(&layer, &ve.name, &data)?;
        } else {
            tracing::info!("create new image layer {}", data);
            ve.kind = VisualElementKind::Image {
                source: data,
                cover_method: cover,
            }
        };

        Ok(ve)
    }
}

impl Behaviour for LayerBehaviour {
    fn binding_vars(&self) -> &'static [&'static str] {
        &[LAYER_LS]
    }

    fn build_elements(
        &self,
        ctx: &tmj_core::script::ContextRef,
    ) -> anyhow::Result<Vec<VisualElement>> {
        let layers = self
            .get_bind_vars(ctx)
            .pop()
            .unwrap()?
            .as_table_or_resolve(ctx)
            .ok_or(anyhow::anyhow!("{LAYER_LS} should be table"))?;
        let items: Vec<_> = layers
            .borrow()
            .iter()
            .map(|(name, val)| (name.clone(), val.clone()))
            .collect();

        let mut out = Vec::new();
        for (name, val) in items {
            let layer = match val.as_table_or_resolve(ctx) {
                Some(v) => v,
                None => continue,
            };
            match self.build_ve_from_layer(&layer, ctx.clone()) {
                Ok(ve) => out.push(ve),
                Err(e) => {
                    let e = e.context(format!("layer {} build ve failded", name));
                    tracing::error!("{e:?}");
                    continue;
                }
            }
        }
        Ok(out)
    }

    fn sync_from_ctx(&mut self, ctx: tmj_core::script::ContextRef) -> anyhow::Result<()> {
        let layers = self
            .get_bind_vars(&ctx)
            .pop()
            .unwrap()?
            .as_table_or_resolve(&ctx)
            .ok_or(anyhow::anyhow!("{LAYER_LS} should be table"))?;

        for (_, layer) in layers.borrow().iter() {
            let layer = layer.as_table_or_resolve(&ctx).ok_or(anyhow::anyhow!(
                "sync_from_ctx faild for parse layer as table faild"
            ))?;
            let layer_type = parse_member(
                &layer,
                layer::LAYER_TYPE,
                "image".to_string(),
                ScriptValue::as_string,
            );
            if layer_type == "effect" {
                let name = parse_member(
                    &layer,
                    layer::NAME,
                    "unknow".to_string(),
                    ScriptValue::as_string,
                );
                let effect_type = parse_member(
                    &layer,
                    layer::DATA,
                    "unknow".to_string(),
                    ScriptValue::as_string,
                );
                self.collect_layer_effect_anim(&layer, &format!("layer.{name}"), &effect_type);
            }
        }
        Ok(())
    }

    fn update_elements(
        &self,
        _screen: &DialogueScene,
        ctx: &tmj_core::script::ContextRef,
        elements: &mut Vec<VisualElement>,
    ) -> anyhow::Result<()> {
        let layers = self
            .get_bind_vars(ctx)
            .pop()
            .unwrap()?
            .as_table_or_resolve(ctx)
            .ok_or(anyhow::anyhow!("{LAYER_LS} should be table"))?;
        let items: Vec<_> = layers
            .borrow()
            .iter()
            .map(|(name, val)| (name.clone(), val.clone()))
            .collect();

        for (name, val) in items {
            let layer = match val.as_table_or_resolve(ctx) {
                Some(v) => v,
                None => continue,
            };
            let ve_name = format!("layer.{name}");
            if let Some(ve) = elements.iter_mut().find(|x| x.name == ve_name) {
                LayerBehaviour::apply_layer_base_prop(&layer, ve);

                // 更新图片源
                if let VisualElementKind::Image { source, .. } = &mut ve.kind {
                    let data = parse_required_member(&layer, layer::DATA, ScriptValue::as_string)?;
                    *source = data;
                }
                // 更新动画
                if let Some(anims) = self.layer_ves_anim_map.borrow().get(&ve.name) {
                    for (_, anim) in anims.iter() {
                        anim.apply_to_ve(ve)?
                    }
                }
            } else {
                elements.push(self.build_ve_from_layer(&layer, ctx.clone())?);
            }
        }
        Ok(())
    }

    fn on_force_over_animation(&mut self) -> anyhow::Result<()> {
        Ok(())
    }

    fn is_animating(&self) -> bool {
        false
    }

    fn on_end_dialouge(&mut self) -> anyhow::Result<()> {
        self.layer_ves_anim_map.borrow_mut().clear();
        Ok(())
    }

    fn on_end_session(&mut self, _ctx: tmj_core::script::ContextRef) -> anyhow::Result<()> {
        // layer 的普通anim也不会强行停止,一定执行完
        for (_, anims) in self.layer_ves_anim_map.borrow_mut().iter_mut() {
            anims.retain(|_, anim| anim.is_indeterminate() || anim.is_animing());
        }
        Ok(())
    }

    fn tick_update(&mut self, _ctx: tmj_core::script::ContextRef, delta_time: std::time::Duration) {
        for (_, anims) in self.layer_ves_anim_map.borrow_mut().iter_mut() {
            for (_, anim) in anims.iter_mut() {
                anim.update(delta_time);
            }
        }
    }
}
