use std::path::PathBuf;

use ratatui::{
    Frame,
    crossterm::event::KeyCode,
    layout::{Constraint, Direction, Layout, Margin, Rect},
    style::Style,
    text::{Line, Span},
    widgets::{Block, Clear, List, ListItem, ListState},
};
use tmj_core::{event::handler::EventDispatcher, img::shape::Pic, pathes};

use crate::{LAYOUT, SETTING, art::theme, pages::pop_items::PopItem};

const GALLERY_LIST_MG: usize = 2;

fn draw_shortkey_bar(frame: &mut Frame, area: Rect) {
    let key_style = theme::THEME.key_binding.key;
    let desc_style = theme::THEME.key_binding.description;
    let line = Line::from(vec![
        Span::styled(" ↑/↓ ", key_style),
        Span::styled("移动 ", desc_style),
        Span::styled(" Enter ", key_style),
        Span::styled("查看 ", desc_style),
        Span::styled(" Esc/q ", key_style),
        Span::styled("退出", desc_style),
    ])
    .centered();
    frame.render_widget(line, area);
}

fn draw_focus_shortkey_bar(frame: &mut Frame, area: Rect) {
    let key_style = theme::THEME.key_binding.key;
    let desc_style = theme::THEME.key_binding.description;
    let line = Line::from(vec![
        Span::styled(" ←/→ ", key_style),
        Span::styled("切换 ", desc_style),
        Span::styled(" Esc/q ", key_style),
        Span::styled("返回列表", desc_style),
    ])
    .centered();
    frame.render_widget(line, area);
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum GalleryMode {
    List,
    Focused,
}

pub struct GalleryLsPopItem {
    shown: bool,
    images: Vec<PathBuf>,
    list_state: ListState,
    mode: GalleryMode,
}

impl GalleryLsPopItem {
    pub fn new_for_mainmenu() -> Self {
        Self {
            shown: false,
            images: Vec::new(),
            list_state: ListState::default(),
            mode: GalleryMode::List,
        }
    }

    fn refresh_images(&mut self) {
        let dir = pathes::path(&SETTING.gallery_dir);
        let mut images = Vec::new();
        let allow_ext = ["png", "jpg"];
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if !path.is_file() {
                    continue;
                }
                let ext = path
                    .extension()
                    .and_then(|e| e.to_str())
                    .map(|e| e.to_ascii_lowercase());
                if let Some(ext) = ext
                    && allow_ext.contains(&ext.as_str())
                {
                    images.push(path);
                }
            }
        }
        images.sort();
        self.images = images;
        if self.images.is_empty() {
            self.list_state.select(None);
        } else {
            self.list_state.select(Some(0));
        }
    }

    pub fn is_focused_mode(&self) -> bool {
        self.mode == GalleryMode::Focused
    }

    pub fn is_list_mode(&self) -> bool {
        self.mode == GalleryMode::List
    }

    pub fn selected_image_path(&self) -> Option<&PathBuf> {
        let selected = self.list_state.selected()?;
        self.images.get(selected)
    }

    fn focus_next(&mut self) {
        if self.images.is_empty() {
            return;
        }
        let idx = self.list_state.selected().unwrap_or(0);
        let next = (idx + 1) % self.images.len();
        self.list_state.select(Some(next));
    }

    fn focus_prev(&mut self) {
        if self.images.is_empty() {
            return;
        }
        let idx = self.list_state.selected().unwrap_or(0);
        let prev = if idx == 0 {
            self.images.len() - 1
        } else {
            idx - 1
        };
        self.list_state.select(Some(prev));
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
}

impl PopItem for GalleryLsPopItem {
    fn set_visual(&mut self, visual: bool) {
        self.shown = visual;
        if visual {
            self.refresh_images();
            self.mode = GalleryMode::List;
        }
    }

    fn draw(&self, frame: &mut Frame, area: Rect) -> anyhow::Result<()> {
        if !self.shown {
            return Ok(());
        }

        let panel = if self.is_focused_mode() {
            area
        } else {
            Self::resolve_mainmenu_panel(area)
        };
        frame.render_widget(Clear, panel);
        frame.render_widget(Block::default().style(theme::THEME.content), panel);

        if self.is_focused_mode() {
            if let Some(path) = self.selected_image_path()
                && let Ok(pic) = Pic::from(path)
            {
                frame.render_widget(pic, panel);
            }
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Fill(1), Constraint::Length(1)])
                .split(panel);
            draw_focus_shortkey_bar(frame, chunks[1]);
            return Ok(());
        }

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Fill(1),
                Constraint::Length(crate::pages::slot::SLOT_SIZE as u16 + 2 * GALLERY_LIST_MG as u16 + 1),
                Constraint::Fill(1),
                Constraint::Length(1),
            ])
            .split(panel);

        let list_rect = chunks[1]
            .centered_horizontally(Constraint::Percentage(90))
            .inner(Margin::new(0, GALLERY_LIST_MG as u16));

        let mut items: Vec<ListItem> = Vec::new();
        if self.images.is_empty() {
            items.push(ListItem::new(Line::from(Span::styled(
                "No images found in gallery_dir",
                Style::new().fg(theme::LIGHT_GRAY),
            ))));
        } else {
            for (idx, path) in self.images.iter().enumerate() {
                let name = path
                    .file_name()
                    .and_then(|s| s.to_str())
                    .unwrap_or("<unknown>");
                let line = Line::from(vec![
                    Span::styled(format!("{:>2}. ", idx + 1), Style::new().fg(theme::WHITE).bold()),
                    Span::styled(name.to_string(), Style::new().fg(theme::WHITE)),
                ]);
                items.push(ListItem::new(line));
            }
        }

        let list = List::new(items)
            .highlight_symbol(">>")
            .highlight_style(theme::THEME.slot_list.selected_item);

        let mut state = self.list_state.clone();
        frame.render_stateful_widget(list, list_rect, &mut state);
        draw_shortkey_bar(frame, chunks[3]);
        Ok(())
    }

    fn is_show(&self) -> bool {
        self.shown
    }
}

impl EventDispatcher for GalleryLsPopItem {
    fn on_key(&mut self, key: &ratatui::crossterm::event::KeyEvent) {
        if self.is_hide() {
            return;
        }
        if key.is_release() {
            return;
        }

        if self.is_list_mode() {
            match key.code {
                KeyCode::Char('q') | KeyCode::Esc if key.is_press() => {
                    self.hide();
                }
                KeyCode::Enter if key.is_press() => {
                    if self.selected_image_path().is_some() {
                        self.mode = GalleryMode::Focused;
                    }
                }
                KeyCode::Down => {
                    self.list_state.select_next();
                }
                KeyCode::Up => {
                    self.list_state.select_previous();
                }
                KeyCode::Home => {
                    self.list_state.select_first();
                }
                KeyCode::End => {
                    self.list_state.select_last();
                }
                _ => {}
            }
            return;
        }

        match key.code {
            KeyCode::Char('q') | KeyCode::Esc if key.is_press() => {
                self.mode = GalleryMode::List;
            }
            KeyCode::Left => {
                self.focus_prev();
            }
            KeyCode::Right => {
                self.focus_next();
            }
            _ => {}
        }
    }
}
