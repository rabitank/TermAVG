use std::cell::RefCell;

use anyhow::{Ok, Result};
use ratatui::Frame;
use ratatui::crossterm::event::KeyCode;
use ratatui::widgets::ListState;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Stylize},
    text::{Line, Span},
    widgets::{List, ListItem},
};
use strum_macros::{Display, EnumString};
use tmj_core::command::{CmdBuffer, GameCmd};
use tmj_core::event::handler::EventDispatcher;

use crate::art::{self, theme};
use crate::pages::{SAVE_MANAGER, Screen, UserScreen};

#[warn(dead_code)]
#[derive(Display, EnumString, Debug, PartialEq)]
enum MainSelections {
    Continue,
    Load,
    NewGame,
    Gallery,
    Setting,
    About,
    Exit,
}

const SELECTION_LEN: usize = 7;

pub struct MainScreen {
    selections: [MainSelections; SELECTION_LEN],
    select_state: RefCell<ListState>,

    frame_count: usize,
}
impl Screen for MainScreen {
    fn active(&mut self, _named_args: &crate::gameflow::NamedArgs) -> anyhow::Result<super::ScreenActRespond> {
        self.frame_count = 0;
        Ok(super::ScreenActRespond::default())
    }
}

impl super::Draw for MainScreen {
    fn draw(&self, frame: &mut Frame, area: Rect) {
        let [title_rect , _, list_rect]= Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Percentage(20), Constraint::Percentage(10), Constraint::Percentage(70)])
            .areas(area);

        art::effect::text(self.frame_count, title_rect, frame.buffer_mut(), theme::LTY_BLUE);

        let list_rect = list_rect.centered(Constraint::Length(50), Constraint::Percentage(100));

        let mut menu_items: Vec<ListItem> = Vec::with_capacity(SELECTION_LEN);
        for (_pos, selection) in self.selections.iter().enumerate() {
            let item = ListItem::new(Line::from(Span::from(format!(
                "{:<25}",
                selection.to_string()
            ))));

            let item = match selection {
                MainSelections::Load => {
                    if SAVE_MANAGER.with(|m| !m.borrow().check_any_save_slot()) {
                        item.fg(Color::DarkGray)
                    } else {
                        item.fg(Color::White)
                    }
                }
                _ => {
                    item.fg(Color::White)
                }
            };
            menu_items.push(item);
        }
        
        let menu_ls = List::new(menu_items)
            .highlight_style(Color::Yellow)
            .highlight_symbol(">> ");
        
        frame.render_stateful_widget(menu_ls, list_rect, &mut *self.select_state.borrow_mut());
    }
}

impl MainScreen {
    pub fn spawn(_name_args: std::collections::HashMap<&str, &str>) -> Self {
        let mut select_state = ListState::default();
        select_state.select(Some(2));
        let select_state = RefCell::new(select_state);
        MainScreen {
            selections: [
                MainSelections::Continue,
                MainSelections::Load,
                MainSelections::NewGame,
                MainSelections::Gallery,
                MainSelections::Setting,
                MainSelections::About,
                MainSelections::Exit,
            ],
            select_state,
            frame_count: 0,
        }
    }
}

impl MainScreen {
    pub fn execute_selection(&mut self) -> Result<()> {
        let cur_selection = &self.selections[self.select_state.borrow().selected().unwrap()];
        match cur_selection {
            MainSelections::NewGame => {
                CmdBuffer::push(GameCmd::GoScene(UserScreen::Dialogue.to_string()));
            }
            MainSelections::Load => {
                CmdBuffer::push(GameCmd::GoScene(UserScreen::Load.to_string()));
            }
            MainSelections::Gallery => {
                CmdBuffer::push(GameCmd::GoScene(UserScreen::Gallery.to_string()));
            }
            MainSelections::Setting => {
                CmdBuffer::push(GameCmd::GoScene(UserScreen::Setting.to_string()));
            }
            MainSelections::Exit => {
                CmdBuffer::push(GameCmd::QuitGame);
            }
            MainSelections::About => {
                CmdBuffer::push(GameCmd::GoScene(UserScreen::About.to_string()));
            }
            MainSelections::Continue => {
                CmdBuffer::push(GameCmd::ContinueGame);
            }
        }
        Ok(())
    }
}

impl EventDispatcher for MainScreen {
    fn handle_tick(&mut self, _tick: std::time::Duration) {
        self.frame_count += 1;
    }

    fn on_key(&mut self, key: &ratatui::crossterm::event::KeyEvent) {
        if !key.is_release() {
            return;
        }
        match key.code {
            KeyCode::Down => {
                self.select_state.borrow_mut().select_next();
            }
            KeyCode::Up => {
                self.select_state.borrow_mut().select_previous();
            }
            KeyCode::Enter => {
                let _ = self.execute_selection();
            }
            _ => {}
        }
    }
}
