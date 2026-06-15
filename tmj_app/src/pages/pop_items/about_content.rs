use std::fs;

use ratatui::{
    Frame,
    crossterm::event::KeyCode,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    text::{Line, Span},
    widgets::{Block, Clear, Paragraph},
};
use tmj_core::{event::handler::EventDispatcher, pathes};

use crate::{LAYOUT, SETTING, art::theme, pages::pop_items::PopItem};

fn draw_shortkey_bar(frame: &mut Frame, area: Rect) {
    let key_style = theme::THEME.key_binding.key;
    let desc_style = theme::THEME.key_binding.description;
    let line = Line::from(vec![
        Span::styled(" q/Esc ", key_style),
        Span::styled("退出", desc_style),
    ])
    .centered();
    frame.render_widget(line, area);
}

pub struct AboutContentPopItem {
    shown: bool,
    lines: Vec<Line<'static>>,
}

impl AboutContentPopItem {
    pub fn new() -> Self {
        Self {
            shown: false,
            lines: Vec::new(),
        }
    }

    fn resolve_mainmenu_panel(area: Rect) -> Rect {
        let max_x = area.x.saturating_add(area.width.saturating_sub(1));
        let pop_x = area.x.saturating_add(LAYOUT.mainmenu_popitem_lw.0).min(max_x);
        let pop_avail_w = area.width.saturating_sub(pop_x.saturating_sub(area.x));
        let configured_pop_w = if LAYOUT.mainmenu_popitem_lw.1 == 0 {
            pop_avail_w
        } else {
            LAYOUT.mainmenu_popitem_lw.1
        };
        let pop_w = configured_pop_w.min(pop_avail_w).max(1);
        Rect::new(pop_x, area.y, pop_w, area.height)
    }

    fn reload_content(&mut self) {
        let content = if let Some(rel) = &SETTING.about_file {
            let path = pathes::path(rel);
            fs::read_to_string(path).unwrap_or_else(|e| format!("about file read failed: {e}"))
        } else {
            "about_file not configured".to_string()
        };
        self.lines = content
            .lines()
            .map(|line| Line::from(line.to_string()).centered())
            .collect();
        if self.lines.is_empty() {
            self.lines.push(Line::from(""));
        }
    }
}

impl PopItem for AboutContentPopItem {
    fn set_visual(&mut self, visual: bool) {
        self.shown = visual;
        if visual {
            self.reload_content();
        }
    }

    fn draw(&self, frame: &mut Frame, area: Rect) -> anyhow::Result<()> {
        if !self.shown {
            return Ok(());
        }

        let panel = Self::resolve_mainmenu_panel(area);
        frame.render_widget(Clear, panel);
        frame.render_widget(Block::default().style(theme::THEME.content), panel);

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Fill(1), Constraint::Length(1)])
            .split(panel);

        let content_h = (self.lines.len() as u16).min(chunks[0].height.saturating_sub(1));
        let content_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Fill(1),
                Constraint::Length(content_h),
                Constraint::Fill(1),
            ])
            .split(chunks[0]);

        let para = Paragraph::new(self.lines.clone()).alignment(Alignment::Center);
        frame.render_widget(para, content_chunks[1]);
        draw_shortkey_bar(frame, chunks[1]);
        Ok(())
    }

    fn is_show(&self) -> bool {
        self.shown
    }
}

impl EventDispatcher for AboutContentPopItem {
    fn on_key(&mut self, key: &ratatui::crossterm::event::KeyEvent) {
        if self.is_hide() {
            return;
        }
        if !key.is_press() {
            return;
        }
        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => {
                self.hide()
            },
            _ => {}
        }
    }
}
