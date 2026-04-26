use std::time::{self, Duration};

use tmj_core::script::TypeName;

use crate::pages::pipeline::animation::Animation;

#[derive(TypeName, Default)]
pub struct AniAlpha {
    pub anim_time: time::Duration,
    pub start_alpha: f64,
    pub target_alpha: f64,
    pub run_time: time::Duration,
}

impl AniAlpha {}

impl Animation for AniAlpha {
    fn apply_to_ve(
        &self,
        ve: &mut crate::pages::pipeline::visual_element::VisualElement,
    ) -> anyhow::Result<()> {
        let elapsed_secs = self.run_time.as_secs_f64().max(0.0);
        let total_secs = self.anim_time.as_secs_f64().max(0.0);
        let mut evalued_alpha = self.start_alpha + (self.target_alpha - self.start_alpha) * (elapsed_secs / total_secs);
        evalued_alpha = evalued_alpha.clamp(0.0, self.target_alpha);
        ve.alpha = evalued_alpha;
        Ok(())
    }

    fn update(&mut self, tick_delta: std::time::Duration) {
        self.run_time += tick_delta;
        self.run_time.clamp(Duration::ZERO, self.anim_time);
    }

    fn force_over(&mut self) {
        self.run_time = self.anim_time;
    }

    fn reset(&mut self) {
        self.run_time = time::Duration::from_secs_f64(0.0);
        self.start_alpha = self.target_alpha;
    }

    fn is_animing(&self) -> bool {
        self.run_time < self.anim_time
    }
}
