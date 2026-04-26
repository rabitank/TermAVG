use std::time::{self, Duration};

use tmj_core::script::TypeName;

use crate::pages::pipeline::{animation::Animation, visual_element::VisualElementKind};

#[derive(TypeName, Default)]
pub struct AniTypeWriter {
    pub anim_time: f64,
    pub start_text: String,
    pub target_text: String,
    pub speed: f64,
    pub run_time: time::Duration,
}

impl AniTypeWriter {
    fn get_diff_chars_len(&self) -> i32 {
        self.target_text.chars().count() as i32 - self.start_text.chars().count() as i32
    }

    fn anim_time(&self) -> Duration {
        if self.speed <= 0.0 {
            return Duration::ZERO
        }
        let diff = self.get_diff_chars_len();
        let anim_time = diff as f64 / self.speed;
        Duration::from_secs_f64(anim_time)
    }
}

impl Animation for AniTypeWriter {
    fn apply_to_ve(
        &self,
        ve: &mut crate::pages::pipeline::visual_element::VisualElement,
    ) -> anyhow::Result<()> {
        if let VisualElementKind::Text { content } = &mut ve.kind {
            let elapsed_secs = self.run_time.as_secs_f64().max(0.0);
            let target_total_chars = self.target_text.chars().count();
            let start_chars = self.start_text.chars().count();
            let shown_chars = start_chars as f64 + elapsed_secs * self.speed;
            let shown_chars = shown_chars.floor().clamp(0.0, target_total_chars as f64) as usize;
            *content = self.target_text.chars().take(shown_chars).collect::<String>();
        }
        Ok(())
    }

    fn update(&mut self, tick_delta: std::time::Duration) {
        self.run_time += tick_delta;
        self.run_time = self.run_time.clamp(Duration::ZERO, self.anim_time());
    }

    fn force_over(&mut self) {
        if self.speed <= 0.0 {
            return
        }
        let diff = self.get_diff_chars_len();
        if diff > 0 {
            self.run_time = time::Duration::from_secs_f64((diff as f64) / self.speed);
        }
    }

    fn reset(&mut self) {
        self.run_time = time::Duration::from_secs_f64(0.0);
        self.start_text = "".into();
        self.target_text = "".into();
    }

    fn is_animing(&self) -> bool {
        if self.target_text == self.start_text {
            return false;
        }
        let anim = (self.anim_time().as_secs_f64() - self.run_time.as_secs_f64()).abs() > 0.001_f64;
        return anim
        
    }
}
