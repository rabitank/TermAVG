use std::{cell::RefCell, rc::Rc};

use ratatui::{
    crossterm::event::{KeyCode, KeyModifiers},
    text::{Line, Span},
    widgets::{self, Paragraph},
};
use tmj_core::{event::handler::EventDispatcher, script::Interpreter};

use crate::{
    art::theme,
    pages::{Draw, pop_items::PopItem},
};

pub struct CmdInputItem {
    interpreter_ref: Rc<RefCell<Interpreter>>,
    shown: bool,
    current_input: String,
}

impl CmdInputItem {
    pub fn new(interpreter_ref: Rc<RefCell<Interpreter>>) -> Self {
        CmdInputItem {
            interpreter_ref,
            shown: true,
            current_input: String::new(),
        }
    }

    fn execute_cmd(&mut self) -> anyhow::Result<()> {
        match self
            .interpreter_ref
            .borrow_mut()
            .eval_new_session(self.current_input.clone())
        {
            Ok(_) => {
                self.current_input.clear();
            }
            Err(e) => {
                tracing::error!("execute cmd failed!: {}", e);
            }
        };
        Ok(())
    }
}

impl PopItem for CmdInputItem {
    fn set_visual(&mut self, visual: bool) {
        self.shown = visual;
    }

    fn draw_impl(
        &self,
        frame: &mut ratatui::Frame,
        rect: ratatui::layout::Rect,
    ) -> anyhow::Result<()> {
        let line = Line::from_iter([
            Span::from(">").style(theme::THEME.root),
            Span::from(self.current_input.clone()).style(theme::THEME.content),
        ]);
        let block = widgets::Block::bordered().style(theme::THEME.root);
        let p = Paragraph::new(line).block(block).style(theme::THEME.content);
        frame.render_widget(widgets::Clear, rect);
        frame.render_widget(p, rect);
        Ok(())
    }

    fn is_show(&self) -> bool {
        self.shown
    }
}

impl EventDispatcher for CmdInputItem {
    fn on_key(&mut self, key: &ratatui::crossterm::event::KeyEvent) {
        if self.is_hide() {
            return;
        }
        if key.is_release() {
            return;
        }
        match key.code {
            KeyCode::Char(c) => {
                self.current_input.push(c);
            }
            KeyCode::Backspace => {
                self.current_input.pop();
            }
            KeyCode::Enter => {
                if self.current_input == "exit" {
                    self.hide();
                    self.current_input.clear();
                } else {
                    let _ = self.execute_cmd();
                }
            }
            _ => {}
        }
    }
}
