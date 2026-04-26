pub mod typewriter;
pub mod alpha_shift;
pub mod img_trans;

use std::any::Any;
use crate::pages::pipeline::visual_element::VisualElement;


pub trait Animation: {
    fn update(&mut self, tick_delta: std::time::Duration);
    fn apply_to_ve(&self, ve: &mut VisualElement) -> anyhow::Result<()>;
    fn force_over(&mut self);
    fn reset(&mut self);
    fn is_animing(&self) -> bool;
    
}

pub trait AnyAnimation:  Any + Animation{
    
}
