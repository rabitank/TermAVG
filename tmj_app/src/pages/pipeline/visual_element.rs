use ratatui::{
    buffer::{Buffer, Cell},
    layout::{Alignment, Rect},
    style::{Color, Style},
    text::Text,
    widgets::{Block, Clear, Paragraph, Widget, Wrap},
};
use tmj_core::{img::shape::Pic, pathes};

#[derive(Clone)]
pub enum VisualElementKind {
    Image { source: String },
    Text { content: String },
    Fill,
    Custom {
        draw: fn(&VisualElement, &mut Buffer, Rect) -> anyhow::Result<()>,
    },
}

impl Default for VisualElementKind {
    fn default() -> Self {
        Self::Fill
    }
}

#[derive(Clone, Default)]
pub enum AnimationState {
    #[default]
    NotStarted,
    Running,
    Finished,
}

#[derive(Clone, Default)]
pub struct VisualElementRuntime {
    pub temp_rect: Option<Rect>,
    pub temp_text: Option<String>,
    pub temp_typewriter_source_text: Option<String>,
    pub temp_typewriter_progress: Option<f64>,
    pub temp_alpha: Option<f64>,
    pub animation_state: AnimationState,
}

#[derive(Clone)]
pub struct VisualElement {
    pub name: String,
    pub visible: bool,
    pub is_animated: bool,
    pub z_index: i32,
    pub rect: Rect,
    pub clear_before_draw: bool,
    pub fill_before_draw: bool,
    pub use_typewriter: bool,
    pub typewriter_speed: f64,
    pub text_alignment: Option<Alignment>,
    pub text_wrap: Option<Wrap>,
    pub text_scroll: (u16, u16),
    pub alpha: f64,
    pub alpha_speed: f64,
    pub kind: VisualElementKind,
    pub style: Style,
    pub runtime: VisualElementRuntime,
}

impl Default for VisualElement {
    fn default() -> Self {
        Self {
            name: String::new(),
            visible: true,
            is_animated: false,
            z_index: 0,
            rect: Rect::default(),
            clear_before_draw: false,
            fill_before_draw: false,
            use_typewriter: false,
            typewriter_speed: 0.0,
            text_alignment: None,
            text_wrap: Some(Wrap { trim: false }),
            text_scroll: (0, 0),
            alpha: 1.0,
            alpha_speed: 0.0,
            kind: VisualElementKind::default(),
            style: Style::default(),
            runtime: VisualElementRuntime::default(),
        }
    }
}

impl VisualElement {
    fn blend_rgb(src: Color, dst: Color, alpha: f64) -> Option<Color> {
        let (Color::Rgb(sr, sg, sb), Color::Rgb(dr, dg, db)) = (src, dst) else {
            return None;
        };
        let remain = 1.0 - alpha;
        let r = f64::from(sr).mul_add(alpha, f64::from(dr) * remain);
        let g = f64::from(sg).mul_add(alpha, f64::from(dg) * remain);
        let b = f64::from(sb).mul_add(alpha, f64::from(db) * remain);
        Some(Color::Rgb(r as u8, g as u8, b as u8))
    }

    fn merge_colors(overlay: Color, base: Color, alpha: f64) -> Color {
        // Treat Reset as empty.
        if overlay == Color::Reset {
            return base;
        }
        if base == Color::Reset {
            return overlay;
        }
        Self::blend_rgb(overlay, base, alpha).unwrap_or(overlay)
    }

    fn merge_cell(overlay: &Cell, base: &Cell, alpha: f64) -> Cell {
        let mut out = base.clone();

        let overlay_symbol_empty = overlay.symbol().trim().is_empty();
        let overlay_fg = overlay.fg;
        let overlay_bg = overlay.bg;
        let base_fg = base.fg;
        let base_bg = base.bg;
        let base_symbol_empty = base.symbol().trim().is_empty();

        let overlay_is_color_block = overlay_symbol_empty && overlay_bg != Color::Reset;
        let base_is_color_block = base_symbol_empty && base_bg != Color::Reset;

        let overlay_effective_fg = if overlay_is_color_block { overlay_bg } else { overlay_fg };
        let overlay_effective_bg = if overlay_is_color_block || overlay_bg != Color::Reset {
            overlay_bg
        } else {
            base_bg
        };
        let base_effective_fg = if base_is_color_block { base_bg } else { base_fg };
        let base_effective_bg = base_bg;

        // Intent 1: empty symbol + valid bg => this cell means "paint a color block".
        if overlay_is_color_block {
            let paint = overlay_bg;
            out.set_fg(Self::merge_colors(paint, base_effective_fg, alpha));
            out.set_bg(Self::merge_colors(paint, base_effective_bg, alpha));
            return out;
        }

        // Intent 2: normal glyph drawing.
        out.set_symbol(overlay.symbol());
        out.set_fg(Self::merge_colors(overlay_effective_fg, base_effective_fg, alpha));
        out.set_bg(Self::merge_colors(overlay_effective_bg, base_effective_bg, alpha));
        out
    }

    fn draw_kind(&self, buffer: &mut Buffer, rect: Rect) -> anyhow::Result<()> {
        match &self.kind {
            VisualElementKind::Image { source } => {
                if !source.trim().is_empty() {
                    let pic = Pic::from(pathes::path(source))?;
                    pic.render(rect, buffer);
                }
            }
            VisualElementKind::Text { content } => {
                let rendered_text = self.runtime.temp_text.clone().unwrap_or_else(|| content.clone());
                let text = Text::from(rendered_text);
                let mut paragraph = Paragraph::new(text).style(self.style).scroll(self.text_scroll);
                if let Some(alignment) = self.text_alignment {
                    paragraph = paragraph.alignment(alignment);
                }
                if let Some(wrap) = self.text_wrap {
                    paragraph = paragraph.wrap(wrap);
                }
                paragraph.render(rect, buffer);
            }
            VisualElementKind::Fill => {
                Block::default().style(self.style).render(rect, buffer);
            }
            VisualElementKind::Custom { draw } => {
                draw(self, buffer, rect)?;
            }
        }
        Ok(())
    }

    fn resolve_render_rect(&self, area: Rect) -> Rect {
        let relative_rect = self.runtime.temp_rect.unwrap_or(self.rect);
        let offset_rect = relative_rect.offset(ratatui::layout::Offset::new(
            area.x as i32,
            area.y as i32,
        ));
        offset_rect.intersection(area)
    }

    pub fn apply_props(&mut self) {
        // Force-settle animation to target state immediately instead of resetting to defaults.
        self.runtime.temp_rect = None;
        self.runtime.temp_alpha = Some(self.alpha.clamp(0.0, 1.0));
        match &self.kind {
            VisualElementKind::Text { content } => {
                if self.use_typewriter {
                    let total_chars = content.chars().count() as f64;
                    self.runtime.temp_typewriter_source_text = Some(content.clone());
                    self.runtime.temp_typewriter_progress = Some(total_chars);
                    self.runtime.temp_text = Some(content.clone());
                } else {
                    self.runtime.temp_text = Some(content.clone());
                    self.runtime.temp_typewriter_source_text = None;
                    self.runtime.temp_typewriter_progress = None;
                }
            }
            _ => {
                self.runtime.temp_text = None;
                self.runtime.temp_typewriter_source_text = None;
                self.runtime.temp_typewriter_progress = None;
            }
        }
        self.runtime.animation_state = AnimationState::NotStarted;
        self.is_animated = false;
    }

    pub fn clear_animation_runtime(&mut self) {
        self.apply_props();
    }

    pub fn render(&self, buffer: &mut Buffer, area: Rect) -> anyhow::Result<()> {
        let alpha = self
            .runtime
            .temp_alpha
            .unwrap_or(self.alpha)
            .clamp(0.0, 1.0);
        if alpha == 0.0 {
            return Ok(());
        }
        if !self.visible {
            return Ok(());
        }
        let rect = self.resolve_render_rect(area);
        if rect.width == 0 || rect.height == 0 {
            return Ok(());
        }
        if self.clear_before_draw {
            Clear::render(Clear, rect, buffer);
        }
        if self.fill_before_draw {
            Block::default().style(self.style).render(rect, buffer);
        }

        if alpha >= 1.0 {
            self.draw_kind(buffer, rect)?;
        } else {
            let mut src = Buffer::empty(rect);
            self.draw_kind(&mut src, rect)?;
            for row in rect.rows() {
                for col in row.columns() {
                    let src_cell = src[(col.x, col.y)].clone();
                    let dst_cell = buffer[(col.x, col.y)].clone();
                    let merged = Self::merge_cell(&src_cell, &dst_cell, alpha);
                    buffer[(col.x, col.y)] = merged;
                }
            }
        }
        Ok(())
    }

    pub fn update_animation(&mut self, delta_secs: f64) {
        if !self.visible {
            self.is_animated = false;
            return;
        }
        let mut text_animated = false;
        match &self.kind {
            VisualElementKind::Text { content } if self.use_typewriter => {
                let final_text = content.as_str();
                let speed = self.typewriter_speed.max(0.0);
                let total_chars = final_text.chars().count() as f64;
                if matches!(self.runtime.animation_state, AnimationState::Finished) {
                    self.runtime.temp_text = Some(final_text.to_string());
                    self.runtime.temp_typewriter_source_text = Some(final_text.to_string());
                    self.runtime.temp_typewriter_progress = Some(total_chars);
                    text_animated = false;
                } else if matches!(self.runtime.animation_state, AnimationState::NotStarted) {
                    let last_source = self
                        .runtime
                        .temp_typewriter_source_text
                        .clone()
                        .unwrap_or_default();
                    if last_source == final_text {
                        self.runtime.temp_text = Some(final_text.to_string());
                        self.runtime.temp_typewriter_progress = Some(total_chars);
                        text_animated = false;
                    } else {
                        self.runtime.temp_typewriter_source_text = Some(final_text.to_string());
                        self.runtime.temp_typewriter_progress = Some(0.0);
                        self.runtime.animation_state = AnimationState::Running;
                    }
                }

                if !matches!(self.runtime.animation_state, AnimationState::Finished) {
                    let mut progress = self.runtime.temp_typewriter_progress.unwrap_or(0.0);
                    progress = (progress + speed * delta_secs.max(0.0)).min(total_chars);
                    self.runtime.temp_typewriter_progress = Some(progress);
                    self.runtime.temp_text =
                        Some(final_text.chars().take(progress.floor() as usize).collect());
                    text_animated = progress < total_chars;
                    if text_animated {
                        self.runtime.animation_state = AnimationState::Running;
                    } else {
                        self.runtime.animation_state = AnimationState::Finished;
                    }
                }
            }
            VisualElementKind::Text { content } => {
                self.runtime.temp_text = Some(content.clone());
                self.runtime.temp_typewriter_source_text = Some(content.clone());
                self.runtime.temp_typewriter_progress = Some(content.chars().count() as f64);
                self.runtime.animation_state = AnimationState::Finished;
                text_animated = false;
            }
            _ => {
                self.runtime.animation_state = AnimationState::Finished;
                text_animated = false;
            }
        }

        let target_alpha = self.alpha.clamp(0.0, 1.0);
        if self.alpha_speed <= 0.0 {
            self.runtime.temp_alpha = Some(target_alpha);
            self.is_animated = text_animated;
            return;
        }
        let current_alpha = self.runtime.temp_alpha.unwrap_or(target_alpha);
        let alpha_step = delta_secs.max(0.0) * self.alpha_speed.max(0.0);
        let next_alpha = if (current_alpha - target_alpha).abs() <= alpha_step {
            target_alpha
        } else if current_alpha < target_alpha {
            current_alpha + alpha_step
        } else {
            current_alpha - alpha_step
        };
        self.runtime.temp_alpha = Some(next_alpha.clamp(0.0, 1.0));
        let alpha_animated = (next_alpha - target_alpha).abs() > f64::EPSILON;

        self.is_animated = text_animated || alpha_animated;
    }
}
