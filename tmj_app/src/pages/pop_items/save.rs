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
};

const SLOT_LIST_MG: usize = 2;

enum EditState {
    Selecting,
    Creating,
}

fn draw_selecting_shortkey_bar(frame: &mut Frame, area: Rect) {
    let key_style = theme::THEME.key_binding.key;
    let desc_style = theme::THEME.key_binding.description;
    let line = Line::from(vec![
        Span::styled(" ↑/↓ ", key_style),
        Span::styled("移动 ", desc_style),
        Span::styled(" Enter ", key_style),
        Span::styled("保存/新建 ", desc_style),
        Span::styled(" Esc/q ", key_style),
        Span::styled("退出", desc_style),
    ])
    .centered();
    frame.render_widget(line, area);
}

fn draw_creating_shortkey_bar(frame: &mut Frame, area: Rect) {
    let key_style = theme::THEME.key_binding.key;
    let desc_style = theme::THEME.key_binding.description;
    let line = Line::from(vec![
        Span::styled(" 字符 ", key_style),
        Span::styled("输入名称 ", desc_style),
        Span::styled(" Backspace ", key_style),
        Span::styled("删除 ", desc_style),
        Span::styled(" Enter ", key_style),
        Span::styled("确认 ", desc_style),
        Span::styled(" Esc ", key_style),
        Span::styled("取消", desc_style),
    ])
    .centered();
    frame.render_widget(line, area);
}

pub struct SavePopItem {
    slot_list: Rc<RefCell<SlotManager>>,
    edit_state: EditState,
    renaming: String,
    shown: bool,
}

impl SavePopItem {
    pub fn new() -> Self {
        let slot_list = SAVE_MANAGER.with(|s| s.clone());
        Self {
            slot_list,
            edit_state: EditState::Selecting,
            renaming: String::new(),
            shown: false,
        }
    }
}

impl PopItem for SavePopItem {
    fn set_visual(&mut self, visual: bool) {
        self.shown = visual;
        if visual {
            self.slot_list.borrow_mut().set_draw_mode(SlotDrawMode::Save);
            self.edit_state = EditState::Selecting;
            self.renaming.clear();
        }
    }

    fn draw(&self, frame: &mut Frame, area: Rect) -> anyhow::Result<()> {
        if !self.shown {
            return Ok(());
        }

        let panel = area.centered(Constraint::Percentage(86), Constraint::Percentage(86));
        frame.render_widget(Clear, panel);
        frame.render_widget(
            Block::default()
                .borders(Borders::ALL)
                .style(theme::THEME.content),
            panel,
        );

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(2),
                Constraint::Min(crate::pages::slot::SLOT_SIZE as u16 + 2 * SLOT_LIST_MG as u16 + 1),
                Constraint::Length(1),
            ])
            .split(panel);

        let list_rect = chunks[1]
            .centered_horizontally(Constraint::Percentage(90))
            .inner(Margin::new(0, SLOT_LIST_MG as u16));
        self.slot_list.borrow_mut().draw(frame, list_rect);

        let title = Line::from_iter([Span::from("Save")
            .bold()
            .style(theme::THEME.slot_list.save.title)])
        .centered();
        frame.render_widget(title, chunks[0]);
        match self.edit_state {
            EditState::Selecting => draw_selecting_shortkey_bar(frame, chunks[2]),
            EditState::Creating => draw_creating_shortkey_bar(frame, chunks[2]),
        }

        if let EditState::Creating = self.edit_state {
            let rename_rect = panel.centered(Constraint::Length(30), Constraint::Length(3));
            let rename_block = Block::bordered()
                .title_top("slot name")
                .style(theme::THEME.save_screen.rename_block);
            let name = Text::from(
                Line::from(self.renaming.clone())
                    .bold()
                    .style(theme::THEME.save_screen.rename_text)
                    .left_aligned(),
            );
            let p = Paragraph::new(name).block(rename_block).centered();
            frame.render_widget(Clear, rename_rect);
            frame.render_widget(p, rename_rect);
        }
        Ok(())
    }

    fn is_show(&self) -> bool {
        self.shown
    }
}

fn is_safe_filename_char(c: char) -> bool {
    matches!(c, 'a'..='z' | 'A'..='Z' | '0'..='9' | ' ' | '-' | '_' | '.')
}

impl EventDispatcher for SavePopItem {
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
                            let now = if let Ok(local) = time::OffsetDateTime::now_local() {
                                local
                            } else {
                                time::OffsetDateTime::now_utc()
                            };
                            slot.time = now;
                            CmdBuffer::push(GameCmd::SaveTo(SaveSlot::Slots(slot.id)));
                        } else {
                            self.edit_state = EditState::Creating;
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
            EditState::Creating => match key.code {
                KeyCode::Backspace => {
                    self.renaming.pop();
                }
                KeyCode::Char(c) if is_safe_filename_char(c) => {
                    self.renaming.push(c);
                }
                KeyCode::Enter if key.is_press() => {
                    if let Some(slot) = self.slot_list.borrow_mut().get_current_slot() {
                        if self.renaming.is_empty() {
                            self.renaming = "unnamed".into();
                        }
                        slot.name = self.renaming.clone();
                        self.renaming.clear();
                        CmdBuffer::push(GameCmd::SaveTo(SaveSlot::Slots(slot.id)));
                    }
                    self.edit_state = EditState::Selecting;
                }
                KeyCode::Esc if key.is_press() => {
                    self.renaming.clear();
                    self.edit_state = EditState::Selecting;
                }
                _ => {}
            },
        }
    }
}
