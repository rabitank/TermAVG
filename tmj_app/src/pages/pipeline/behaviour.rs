//! 与 [`crate::pages::pop_items::PopItem`] 相同：`DialogueVeStage: Any`，在 [`impl dyn DialogueVeStage`] 上提供 `as_stage` / `as_stage_mut`。

use std::{any::Any, cell::RefCell, collections::HashMap, rc::Rc, time::Duration};

use ratatui::layout::Rect;
use tmj_core::{
    impl_rust_object,
    script::{ContextRef, ScriptValue, TypeName},
};

use super::visual_element::VisualElement;
use crate::pages::dialogue::DialogueScene;

pub fn get_script_vars(ctx: &ContextRef, vars: &[&str]) -> Vec<anyhow::Result<ScriptValue>> {
    let ct = ctx.borrow();
    vars.iter()
        .map(|s| ct.resolve_path(s).map_err(|e| anyhow::anyhow!(e)))
        .collect()
}
/// 对话管线阶段：脚本状态 → `VisualElement`；由 `DialogueScene` 持有 `Box` 实例并顺序调用。
pub trait Behaviour: Any + 'static {
    fn binding_vars(&self) -> &'static [&'static str];

    fn get_bind_vars(&self, ctx: &ContextRef) -> Vec<anyhow::Result<ScriptValue>> {
        let vars = self.binding_vars();
        get_script_vars(ctx, vars)
    }

    fn build_elements(&self, ctx: &ContextRef) -> anyhow::Result<Vec<VisualElement>>;

    fn tick_update(&mut self, _ctx: ContextRef, _delta_time: Duration){
    }

    fn update_elements(
        &self,
        screen: &DialogueScene,
        ctx: &ContextRef,
        elements: &mut Vec<VisualElement>,
    ) -> anyhow::Result<()>;

    fn stage_clear(
        &self,
        _screen: &DialogueScene,
        _ctx: &ContextRef,
        _elements: &mut Vec<VisualElement>,
        _area: Rect,
    ) -> anyhow::Result<()> {
        Ok(())
    }

    fn on_force_over_animation(&mut self) -> anyhow::Result<()>;

    fn on_end_dialouge(&mut self) -> anyhow::Result<()>;

    fn on_end_session(&mut self, ctx: ContextRef) -> anyhow::Result<()>;

    fn on_scene_active(&mut self, _ctx: ContextRef) -> anyhow::Result<()> {
        Ok(())
    }

    fn is_animating(&self) -> bool {
        false
    }
}

impl dyn Behaviour {
    pub fn as_behaviour<T: Behaviour + TypeName>(&self) -> anyhow::Result<&T> {
        (self as &dyn Any)
            .downcast_ref()
            .ok_or(anyhow::anyhow!("Get Behaviour {} failed", T::TYPE_NAME))
    }

    pub fn as_behaviour_mut<T: Behaviour + TypeName>(&mut self) -> anyhow::Result<&mut T> {
        (self as &mut dyn Any)
            .downcast_mut()
            .ok_or(anyhow::anyhow!("Get Behaviour {} failed", T::TYPE_NAME))
    }
}

#[derive(Clone)]
pub struct BehaviourMap {
    pub behaviours: Rc<RefCell<HashMap<String, Box<dyn Behaviour>>>>,
}

impl BehaviourMap {
    pub fn values_mut(&self) -> std::cell::RefMut<'_, HashMap<String, Box<dyn Behaviour>>> {
        self.behaviours.borrow_mut()
    }
}

impl_rust_object!(BehaviourMap);

pub fn with_behaviour_mut_from_ctx<T, R>(
    ctx: &ContextRef,
    mutator: impl FnOnce(&mut T) -> R,
) -> anyhow::Result<R>
where
    T: Behaviour + TypeName,
{
    use crate::pages::script_def::env::BEHAVIOURS_MAP;
    let behaviours_val = ctx
        .borrow()
        .get_global_val(BEHAVIOURS_MAP)
        .ok_or(anyhow::anyhow!(
            "{BEHAVIOURS_MAP} not found in script globals"
        ))?;
    let behaviour_map = behaviours_val
        .downcast_mut::<BehaviourMap>()
        .ok_or(anyhow::anyhow!(
            "{BEHAVIOURS_MAP} is not BehaviourMap rust object"
        ))?;
    let mut borrowed = behaviour_map.behaviours.borrow_mut();
    let behaviour = borrowed
        .get_mut(T::TYPE_NAME)
        .ok_or(anyhow::anyhow!("behaviour {} not found", T::TYPE_NAME))?;
    let typed = behaviour
        .as_mut()
        .as_behaviour_mut::<T>()
        .map_err(|e| anyhow::anyhow!(e))?;

    Ok(mutator(typed))
}
