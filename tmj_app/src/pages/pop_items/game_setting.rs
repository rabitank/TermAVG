use ratatui::{
    Frame,
    crossterm::event::KeyCode,
    layout::{Constraint, Direction, Layout, Margin, Rect},
    style::{Style, Stylize},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, ListState},
};
use tmj_core::{event::handler::EventDispatcher, script::ScriptValue};

use crate::{
    GAME_SETTING, LAYOUT,
    art::theme,
    game_setting::{BGM_VOLUME, EFFECT_VOLUME, ENV_VOLUME},
    pages::pop_items::PopItem,
};

const LIST_MG: usize = 2;
const STEP: f64 = 0.05;

struct SettingEntry {
    key: &'static str,
    label: &'static str,
    value: f64,
}

fn draw_shortkey_bar(frame: &mut Frame, area: Rect) {
    let key_style = theme::THEME.key_binding.key;
    let desc_style = theme::THEME.key_binding.description;
    let line = Line::from(vec![
        Span::styled(" ↑/↓ ", key_style),
        Span::styled("选择 ", desc_style),
        Span::styled(" ←/→ ", key_style),
        Span::styled("增减 ", desc_style),
        Span::styled(" Esc/q ", key_style),
        Span::styled("退出", desc_style),
    ])
    .centered();
    frame.render_widget(line, area);
}

pub struct GameSettingPopItem {
    shown: bool,
    main_menu_mode: bool,
    list_state: ListState,
    entries: Vec<SettingEntry>,
}

impl GameSettingPopItem {
    pub fn new() -> Self {
        Self::new_with_mode(false)
    }

    pub fn new_for_mainmenu() -> Self {
        Self::new_with_mode(true)
    }

    fn new_with_mode(main_menu_mode: bool) -> Self {
        let mut list_state = ListState::default();
        list_state.select(Some(0));
        Self {
            shown: false,
            main_menu_mode,
            list_state,
            entries: vec![
                SettingEntry {
                    key: BGM_VOLUME,
                    label: "BGM Volume",
                    value: 1.0,
                },
                SettingEntry {
                    key: ENV_VOLUME,
                    label: "Env Volume",
                    value: 1.0,
                },
                SettingEntry {
                    key: EFFECT_VOLUME,
                    label: "Effect Volume",
                    value: 1.0,
                },
            ],
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

    fn sync_from_setting(&mut self) {
        GAME_SETTING.with_borrow(|s| {
            for entry in &mut self.entries {
                if let Some(v) = s.get_number(entry.key) {
                    entry.value = v.clamp(0.0, 1.0);
                }
            }
        });
    }

    fn adjust_selected(&mut self, delta: f64) {
        let Some(idx) = self.list_state.selected() else {
            return;
        };
        let Some(entry) = self.entries.get_mut(idx) else {
            return;
        };
        let new_val = (entry.value + delta).clamp(0.0, 1.0);
        entry.value = new_val;

        GAME_SETTING.with_borrow_mut(|s| {
            let _ = s.apply_field(entry.key.to_string(), ScriptValue::Float(new_val));
        });
    }
}

impl PopItem for GameSettingPopItem {
    fn set_visual(&mut self, visual: bool) {
        self.shown = visual;
        if visual {
            self.sync_from_setting();
            if self.list_state.selected().is_none() {
                self.list_state.select(Some(0));
            }
        } else {
            GAME_SETTING.with_borrow(|s| {
                if let Err(e) = s.persist_to_file() {
                    tracing::warn!("persist game setting failed: {:?}", e);
                }
            });
        }
    }

    fn draw(&self, frame: &mut Frame, area: Rect) -> anyhow::Result<()> {
        if !self.shown {
            return Ok(());
        }

        let panel = if self.main_menu_mode {
            Self::resolve_mainmenu_panel(area)
        } else {
            area.centered(Constraint::Percentage(86), Constraint::Percentage(86))
        };
        frame.render_widget(Clear, panel);
        let panel_block = if self.main_menu_mode {
            Block::default().style(theme::THEME.content)
        } else {
            Block::default()
                .borders(Borders::ALL)
                .style(theme::THEME.content)
        };
        frame.render_widget(panel_block, panel);

        let list_h = self.entries.len() as u16 + 2 * LIST_MG as u16 + 1;
        let chunks = if self.main_menu_mode {
            Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Fill(1),
                    Constraint::Length(list_h),
                    Constraint::Fill(1),
                    Constraint::Length(1),
                ])
                .split(panel)
        } else {
            Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(2),
                    Constraint::Min(list_h),
                    Constraint::Length(1),
                ])
                .split(panel)
        };
        let (title_rect, list_rect, shortkey_rect) = if self.main_menu_mode {
            (Rect::default(), chunks[1], chunks[3])
        } else {
            (chunks[0], chunks[1], chunks[2])
        };

        let list_rect = list_rect
            .centered_horizontally(Constraint::Percentage(90))
            .inner(Margin::new(0, LIST_MG as u16));
        let items = self
            .entries
            .iter()
            .map(|entry| {
                let pct = (entry.value * 100.0).round() as i64;
                ListItem::new(Line::from(vec![
                    Span::styled(format!("{:<14}", entry.label), Style::new().fg(theme::WHITE)),
                    Span::styled(
                        format!(" {:>3}%", pct),
                        Style::new().fg(theme::LTY_BLUE).bold(),
                    ),
                ]))
            })
            .collect::<Vec<_>>();
        let list = List::new(items)
            .highlight_symbol(">>")
            .highlight_style(theme::THEME.slot_list.selected_item);
        let mut state = self.list_state.clone();
        frame.render_stateful_widget(list, list_rect, &mut state);

        if !self.main_menu_mode {
            let title = Line::from_iter([Span::from("Game Setting")
                .bold()
                .style(theme::THEME.slot_list.save.title)])
            .centered();
            frame.render_widget(title, title_rect);
        }

        draw_shortkey_bar(frame, shortkey_rect);
        Ok(())
    }

    fn is_show(&self) -> bool {
        self.shown
    }
}

impl EventDispatcher for GameSettingPopItem {
    fn on_key(&mut self, key: &ratatui::crossterm::event::KeyEvent) {
        if self.is_hide() {
            return;
        }
        if key.is_release() {
            return;
        }

        match key.code {
            KeyCode::Char('q') | KeyCode::Esc if key.is_press() => {
                self.hide()
            },
            KeyCode::Down => self.list_state.select_next(),
            KeyCode::Up => self.list_state.select_previous(),
            KeyCode::Left => self.adjust_selected(-STEP),
            KeyCode::Right => self.adjust_selected(STEP),
            _ => {}
        }
    }
}
