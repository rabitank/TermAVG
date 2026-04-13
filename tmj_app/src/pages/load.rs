use crate::pages::Screen;
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

pub struct LoadScreen {
    slot_list: Rc<RefCell<SlotManager>>,
    edit_state: EditState,
}
impl Screen for LoadScreen {}

enum EditState {
    Selecting,
    Confiring, // 确认中
}

impl LoadScreen {
    pub fn spawn(_name_args: HashMap<&str, &str>) -> Self {
        Self {
            slot_list: SAVE_MANAGER.with(|s| s.clone()),
            edit_state: EditState::Selecting,
        }
    }
}

impl EventDispatcher for LoadScreen {
    fn on_quit(&mut self) {
        match self.edit_state {
            EditState::Selecting => {} //todo!(),
            EditState::Confiring => {
                self.edit_state = EditState::Selecting;
            } //todo!(),
        }
    }

    fn on_key(&mut self, key: &ratatui::crossterm::event::KeyEvent) {
        let mut slot_list = self.slot_list.borrow_mut();
        let slot = slot_list.get_current_slot();
        if slot.is_none() {
            tracing::error!("selecting slot is None!");
            return;
        }
        let slot = slot.unwrap();
        match self.edit_state {
            EditState::Selecting => match key.code {
                KeyCode::Enter if key.is_release() => {
                    if slot.path.is_some() {
                        self.edit_state = EditState::Confiring;
                    }
                }
                KeyCode::Char('q') | KeyCode::Esc if key.is_release() => {
                    CmdBuffer::push(GameCmd::GoBack); // todo! 也许先这样?
                }
                _ if !key.is_release() => {
                    slot_list.on_key(key);
                }
                _ => {}
            },
            EditState::Confiring => match key.code {
                KeyCode::Char('y') if key.is_release() => {
                    CmdBuffer::push(GameCmd::LoadFrom(SaveSlot::Slots(slot.id)));
                    self.edit_state = EditState::Selecting;
                }
                KeyCode::Char('n') | KeyCode::Char('q') | KeyCode::Esc if key.is_release() => {
                    self.edit_state = EditState::Selecting;
                }
                _ => {}
            },
        }
    }
}

impl super::Draw for LoadScreen {
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
            Line::from_iter([Span::from("Load").bold(), Span::from("<Enter> to Load")]).centered();
        frame.render_widget(title, title_rect);

        if let EditState::Confiring = self.edit_state {
            let confir_rect = area.centered(Constraint::Length(30), Constraint::Length(3));
            let slot_name = self
                .slot_list
                .borrow_mut()
                .get_current_slot()
                .unwrap()
                .name
                .clone();
            let confir_block = Block::bordered()
                .title_top(format!("load {}", slot_name))
                .light_blue();
            let name = Text::from(
                Line::from(vec![
                    Span::from("<y>: yes ").fg(Color::Green),
                    Span::from("<n>: no ").fg(Color::DarkGray),
                ])
                .bold()
                .centered(),
            );
            let p = Paragraph::new(name).block(confir_block).centered();
            frame.render_widget(Clear, confir_rect);
            frame.render_widget(p, confir_rect);
        }
    }
}
