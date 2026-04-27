use std::time::Duration;

use ratatui::layout::Rect;
use tmj_core::script::TypeName;

use crate::pages::pipeline::{
    animation::Animation,
    visual_element::VisualElement,
};

/// 在 `anim_time` 内将 `VisualElement.rect` 从 `start_rect` 线性插值到 `target_rect`。
#[derive(TypeName)]
pub struct AniRectTrans {
    pub anim_time: Duration,
    pub start_rect: Rect,
    pub target_rect: Rect,
    pub run_time: Duration,
}

impl Default for AniRectTrans {
    fn default() -> Self {
        Self {
            anim_time: Duration::ZERO,
            start_rect: Rect::default(),
            target_rect: Rect::default(),
            run_time: Duration::ZERO,
        }
    }
}

impl AniRectTrans {
    /// 开始一次矩形过渡（会重置已流逝时间）。
    pub fn export_rect_trans(&mut self, start: Rect, target: Rect, duration_secs: f64) {
        self.start_rect = start;
        self.target_rect = target;
        self.anim_time = Duration::from_secs_f64(duration_secs.max(0.0));
        self.run_time = Duration::ZERO;
    }
}

impl Animation for AniRectTrans {
    fn apply_to_ve(&self, ve: &mut VisualElement) -> anyhow::Result<()> {
        let t = if self.anim_time.is_zero() {
            1.0
        } else {
            (self.run_time.as_secs_f64() / self.anim_time.as_secs_f64()).clamp(0.0, 1.0)
        };

        let lerp_u16 = |a: u16, b: u16| -> u16 {
            let af = a as f64;
            let bf = b as f64;
            (af + (bf - af) * t).round().clamp(0.0, u16::MAX as f64) as u16
        };

        let lerp_dim = |a: u16, b: u16| -> u16 {
            let af = a as f64;
            let bf = b as f64;
            (af + (bf - af) * t).round().max(0.0).clamp(0.0, u16::MAX as f64) as u16
        };

        ve.rect = Rect::new(
            lerp_u16(self.start_rect.x, self.target_rect.x),
            lerp_u16(self.start_rect.y, self.target_rect.y),
            lerp_dim(self.start_rect.width, self.target_rect.width),
            lerp_dim(self.start_rect.height, self.target_rect.height),
        );
        Ok(())
    }

    fn update(&mut self, tick_delta: Duration) {
        self.run_time += tick_delta;
        self.run_time = self.run_time.min(self.anim_time);
    }

    fn force_over(&mut self) {
        self.run_time = self.anim_time;
    }

    fn reset(&mut self) {
        self.run_time = Duration::ZERO;
    }

    fn is_animing(&self) -> bool {
        self.run_time < self.anim_time
    }
}
