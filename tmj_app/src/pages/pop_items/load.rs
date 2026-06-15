use std::{cell::RefCell, rc::Rc};

use ratatui::{
    Frame,
    crossterm::event::KeyCode,
    layout::{Constraint, Direction, Layout, Margin, Rect},
    style::Stylize,
    text::{Line, Span, Text},
    widgets::{Block, Borders, Clear, Paragraph},
};
use tmj_core::{
    command::{CmdBuffer, GameCmd, SaveSlot},
    event::handler::EventDispatcher,
};

use crate::{
    art::theme,
    pages::{
        Draw,
        pop_items::PopItem,
        slot::{SAVE_MANAGER, SlotDrawMode, SlotManager},
    },
    LAYOUT,
};

const SLOT_LIST_MG: usize = 2;

enum EditState {
    Selecting,
    Confirming,
}

fn draw_selecting_shortkey_bar(frame: &mut Frame, area: Rect) {
    let key_style = theme::THEME.key_binding.key;
    let desc_style = theme::THEME.key_binding.description;
    let line = Line::from(vec![
        Span::styled(" ↑/↓ ", key_style),
        Span::styled("移动 ", desc_style),
        Span::styled(" Enter ", key_style),
        Span::styled("加载 ", desc_style),
        Span::styled(" Esc/q ", key_style),
        Span::styled("退出", desc_style),
    ])
    .centered();
    frame.render_widget(line, area);
}

fn draw_confirming_shortkey_bar(frame: &mut Frame, area: Rect) {
    let key_style = theme::THEME.key_binding.key;
    let desc_style = theme::THEME.key_binding.description;
    let line = Line::from(vec![
        Span::styled(" y ", key_style),
        Span::styled("确认加载 ", desc_style),
        Span::styled(" Esc/q ", key_style),
        Span::styled("取消", desc_style),
    ])
    .centered();
    frame.render_widget(line, area);
}

pub struct LoadPopItem {
    slot_list: Rc<RefCell<SlotManager>>,
    edit_state: EditState,
    shown: bool,
    main_menu_mode: bool,
}

impl LoadPopItem {
    pub fn new() -> Self {
        Self::new_with_mode(false)
    }

    pub fn new_for_mainmenu() -> Self {
        Self::new_with_mode(true)
    }

    fn new_with_mode(main_menu_mode: bool) -> Self {
        let slot_list = SAVE_MANAGER.with(|s| s.clone());
        Self {
            slot_list,
            edit_state: EditState::Selecting,
            shown: false,
            main_menu_mode,
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
}

impl PopItem for LoadPopItem {
    fn set_visual(&mut self, visual: bool) {
        self.shown = visual;
        if visual {
            self.slot_list.borrow_mut().set_draw_mode(SlotDrawMode::Load);
            self.edit_state = EditState::Selecting;
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

        let list_h = crate::pages::slot::SLOT_SIZE as u16 + 2 * SLOT_LIST_MG as u16 + 1;
        let chunks = if self.main_menu_mode {
            // For mainmenu popup, keep list centered and shortcut bar at bottom.
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
            .inner(Margin::new(0, SLOT_LIST_MG as u16));
        self.slot_list.borrow_mut().draw(frame, list_rect);

        if !self.main_menu_mode {
            let title = Line::from_iter([Span::from("Load")
                .bold()
                .style(theme::THEME.slot_list.load.title)])
            .centered();
            frame.render_widget(title, title_rect);
        }
        match self.edit_state {
            EditState::Selecting => draw_selecting_shortkey_bar(frame, shortkey_rect),
            EditState::Confirming => draw_confirming_shortkey_bar(frame, shortkey_rect),
        }

        if let EditState::Confirming = self.edit_state {
            let confirm_rect = panel.centered(Constraint::Length(30), Constraint::Length(3));
            let slot_name = self
                .slot_list
                .borrow_mut()
                .get_current_slot()
                .map(|slot| slot.name.clone())
                .unwrap_or_default();
            let confirm_block = Block::bordered()
                .title_top(format!("load {}", slot_name))
                .style(theme::THEME.load_screen.confirm_block);
            let tips = Text::from(
                Line::from(vec![
                    Span::from("<y>: yes ").style(theme::THEME.load_screen.confirm_yes),
                    Span::from("<n>: no ").style(theme::THEME.load_screen.confirm_no),
                ])
                .bold()
                .centered(),
            );
            let p = Paragraph::new(tips).block(confirm_block).centered();
            frame.render_widget(Clear, confirm_rect);
            frame.render_widget(p, confirm_rect);
        }
        Ok(())
    }

    fn is_show(&self) -> bool {
        self.shown
    }
}

impl EventDispatcher for LoadPopItem {
    fn on_key(&mut self, key: &ratatui::crossterm::event::KeyEvent) {
        if self.is_hide() {
            return;
        }
        if key.is_release() {
            return;
        }
        match self.edit_state {
            EditState::Selecting => match key.code {
                KeyCode::Enter if key.is_press() => {
                    if let Some(slot) = self.slot_list.borrow_mut().get_current_slot() {
                        if slot.path.is_some() {
                            self.edit_state = EditState::Confirming;
                        }
                    }
                }
                KeyCode::Char('q') | KeyCode::Esc if key.is_press() => {
                    self.hide();
                }
                _ => {
                    self.slot_list.borrow_mut().on_key(key);
                }
            },
            EditState::Confirming => match key.code {
                KeyCode::Char('y') if key.is_press() => {
                    if let Some(slot) = self.slot_list.borrow_mut().get_current_slot() {
                        CmdBuffer::push(GameCmd::LoadFrom(SaveSlot::Slots(slot.id)));
                    }
                    self.edit_state = EditState::Selecting;
                    self.hide();
                }
                KeyCode::Char('q') | KeyCode::Esc if key.is_press() => {
                    self.edit_state = EditState::Selecting;
                }
                _ => {}
            },
        }
    }
}
