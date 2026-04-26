use ratatui::{
    buffer::{Buffer, Cell},
    layout::{Alignment, Margin, Rect},
    style::{Color, Style},
    text::Text,
    widgets::{Block, Borders, Clear, Paragraph, Widget, Wrap},
};
use tmj_core::{img::shape::Pic, pathes};

// 使用假的Clone来满足需求罢了
fn _default_no_draw(_: &VisualElement, _: &mut Buffer, _: Rect) -> anyhow::Result<()> {
    Ok(())
}
pub struct VisualElementCustomDrawer {
    pub draw: Box<dyn Fn(&VisualElement, &mut Buffer, Rect) -> anyhow::Result<()>>,
}
impl Clone for VisualElementCustomDrawer {
    fn clone(&self) -> Self {
        Self {
            draw: Box::new(_default_no_draw),
        }
    }
}
impl VisualElementCustomDrawer {
    pub fn from<F>(draw: F) -> Self
    where
        F: Fn(&VisualElement, &mut Buffer, Rect) -> anyhow::Result<()> + 'static,
    {
        VisualElementCustomDrawer {
            draw: Box::new(draw),
        }
    }
}

#[derive(Clone)]
pub enum VisualElementKind {
    Image { source: String },
    Text { content: String },
    Fill,
    Custom { drawer: VisualElementCustomDrawer },
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

#[derive(Clone)]
pub struct VisualElement {
    pub name: String,
    pub visible: bool,
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
    pub border: bool,
    pub border_style: Style,
    pub border_type: ratatui::widgets::BorderType,
    pub alpha_speed: f64,
    pub kind: VisualElementKind,
    pub style: Style,
}

impl Default for VisualElement {
    fn default() -> Self {
        Self {
            name: String::new(),
            visible: true,
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
            border: false,
            border_style: Style::default(),
            border_type: ratatui::widgets::BorderType::Rounded,
            kind: VisualElementKind::default(),
            style: Style::default(),
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

        let overlay_effective_fg = if overlay_is_color_block {
            overlay_bg
        } else {
            overlay_fg
        };
        let overlay_effective_bg = if overlay_is_color_block || overlay_bg != Color::Reset {
            overlay_bg
        } else {
            base_bg
        };
        let base_effective_fg = if base_is_color_block {
            base_bg
        } else {
            base_fg
        };
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
        out.set_fg(Self::merge_colors(
            overlay_effective_fg,
            base_effective_fg,
            alpha,
        ));
        out.set_bg(Self::merge_colors(
            overlay_effective_bg,
            base_effective_bg,
            alpha,
        ));
        out
    }

    fn draw_content(&self, buffer: &mut Buffer, rect: Rect) -> anyhow::Result<()> {

        if self.fill_before_draw {
            Block::default().style(self.style).render(rect, buffer);
        }
        match &self.kind {
            VisualElementKind::Image { source } => {
                if !source.trim().is_empty() {
                    let pic = Pic::from(pathes::path(source))?;
                    pic.render(rect, buffer);
                }
            }
            VisualElementKind::Text { content } => {
                let text = Text::from(content.clone());
                let mut paragraph = Paragraph::new(text)
                    .style(self.style)
                    .scroll(self.text_scroll);
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
            VisualElementKind::Custom { drawer } => {
                drawer.draw.as_ref()(self, buffer, rect)?;
            }
        }
        Ok(())
    }

    fn resolve_render_rect(&self, area: Rect) -> Rect {
        let offset_rect = self
            .rect
            .offset(ratatui::layout::Offset::new(area.x as i32, area.y as i32));
        offset_rect.intersection(area)
    }

    pub fn render(&self, buffer: &mut Buffer, area: Rect) -> anyhow::Result<()> {
        let alpha = self.alpha.clamp(0.0, 1.0);
        if alpha == 0.0 {
            return Ok(());
        }
        if !self.visible {
            return Ok(());
        }
        let mut rect = self.resolve_render_rect(area);
        if rect.width == 0 || rect.height == 0 {
            return Ok(());
        }
        if self.clear_before_draw {
            Clear::render(Clear, rect, buffer);
        }
        if self.border {
            Block::default()
                .borders(Borders::all())
                .border_type(self.border_type)
                .border_style(self.border_style)
                .render(rect, buffer);
            rect = rect.inner(Margin::new(1, 1));
        }
        if alpha >= 1.0 {
            self.draw_content(buffer, rect)?;
        } else {
            let mut src = Buffer::empty(rect);
            self.draw_content(&mut src, rect)?;
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
}
