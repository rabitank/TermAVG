use crate::pages::{Screen, UserScreen};
use crate::pages::slot::{SAVE_MANAGER, SlotManager};
use ratatui::Frame;
use ratatui::crossterm::event::KeyCode;
use ratatui::layout::Margin;
use ratatui::style::Color;
use ratatui::text::Text;
use ratatui::widgets::{Block, Clear, Paragraph};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::Stylize,
    text::{Line, Span},
};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use tmj_core::command::{CmdBuffer, GameCmd, SaveSlot};
use tmj_core::event::handler::EventDispatcher;

const SLOT_LIST_MG: usize = 2;

pub struct SaveScreen {
    slot_list: Rc<RefCell<SlotManager>>,
    edit_state: EditState,
    renaming: String,
}

impl Screen for SaveScreen {}

enum EditState {
    Selecting,
    Creating,
    Confiring, // 确认中
}

impl SaveScreen {
    pub fn spawn(_name_args: HashMap<&str, &str>) -> Self {
        Self {
            slot_list: SAVE_MANAGER.with(|s| s.clone()),
            edit_state: EditState::Selecting,
            renaming: "".into(),
        }
    }
}

impl EventDispatcher for SaveScreen {
    fn on_quit(&mut self) {
        match self.edit_state {
            EditState::Creating => {
                let _slot = self.slot_list.borrow_mut().get_current_slot();
                // todo!
            }
            EditState::Selecting => {} //todo!(),
            EditState::Confiring => {} //todo!(),
        }
    }

    fn on_key(&mut self, key: &ratatui::crossterm::event::KeyEvent) {
        let mut slot_list = self.slot_list.borrow_mut();
        let slot = slot_list.get_current_slot();
        match self.edit_state {
            EditState::Selecting => match key.code {
                KeyCode::Enter if key.is_release() => {
                    if slot.is_some() {
                        let slot = slot.unwrap();
                        if slot.path.is_some() {
                            let now = if let Ok(_now) = time::OffsetDateTime::now_local() {
                                _now
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
                KeyCode::Char('q') | KeyCode::Esc => {
                    CmdBuffer::push(GameCmd::GoScene(UserScreen::Dialogue.to_string())); // todo! 也许先这样?
                }
                _ if !key.is_release() => {
                    slot_list.on_key(key);
                }
                _ => {}
            },
            EditState::Creating => match key.code {
                KeyCode::Backspace if !key.is_release() => {
                    self.renaming.pop();
                }
                KeyCode::Char(c) if !key.is_release() => {
                    self.renaming.push(c);
                }
                KeyCode::Enter | KeyCode::Esc if key.is_release() => {
                    if slot.is_some() {
                        let slot = slot.unwrap();
                        if self.renaming.is_empty() {
                            self.renaming = "unnamed".into();
                        }
                        slot.name = self.renaming.clone().into();
                        self.renaming = "".into();
                        CmdBuffer::push(GameCmd::SaveTo(SaveSlot::Slots(slot.id)));
                    }
                    self.edit_state = EditState::Selecting;
                }
                _ => {}
            },
            EditState::Confiring => todo!(),
        }
    }
}

impl super::Draw for SaveScreen {
    fn draw(&self, frame: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(2),
                Constraint::Min(super::slot::SLOT_SIZE as u16 + 2 * SLOT_LIST_MG as u16),
            ])
            .split(area);

        let list_rect = chunks[1]
            .centered_horizontally(Constraint::Percentage(80))
            .inner(Margin::new(0, SLOT_LIST_MG as u16));
        self.slot_list.borrow_mut().draw(frame, list_rect);

        let title_rect = chunks[0];

        let title =
            Line::from_iter([Span::from("Save").bold(), Span::from("<Enter> to save")]).centered();
        frame.render_widget(title, title_rect);

        if let EditState::Creating = self.edit_state {
            let rename_rect = area.centered(Constraint::Length(30), Constraint::Length(3));
            let rename_block = Block::bordered().title_top("slot name").light_blue();
            let name = Text::from(
                Line::from(self.renaming.clone())
                    .bold()
                    .fg(Color::White)
                    .left_aligned(),
            );
            let p = Paragraph::new(name).block(rename_block).centered();
            frame.render_widget(Clear, rename_rect);
            frame.render_widget(p, rename_rect);
        }
    }
}
