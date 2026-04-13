use anyhow::{Ok, Result};
use ratatui::crossterm::event::{KeyEvent, MouseEvent};

use crate::event::GameEvent;


pub trait EventDispatcher {

    fn on_key(&mut self, key: &KeyEvent){

    }

    fn on_quit(&mut self) {

    }

    fn on_mouse(&mut self, mouse: &MouseEvent) {

    }

    fn on_resize(&mut self, w: u16, h: u16)
    {

    }

    fn handle_event(&mut self, event: &GameEvent)  -> Result<bool>{
        match event {
            GameEvent::CtKeyEvent(key) => self.on_key(key),
            GameEvent::CtMouseEvent(mouse) => self.on_mouse(mouse),
            GameEvent::ResizeTerm(w, h) => self.on_resize(*w, *h),
            GameEvent::QuitGame => self.on_quit(),
            _ => (),
        }
        Ok(true)
    }

    fn handle_tick(&mut self, tick: std::time::Duration) {}

}


