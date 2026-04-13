mod cmd;

use std::{
    any::Any, time::Duration,
};

pub use cmd::CmdInputItem;
use ratatui::layout::Rect;
use strum_macros::{Display, EnumString};
use tmj_core::event::EventManager;

use crate::pages::Draw;

#[derive(EnumString, Display, Debug)]
pub enum UserItems {
    CmdInput,
}

pub trait PopItem: Any + 'static {

    fn set_visual(&mut self, visual: bool);

    fn draw_impl(&self, _frame: &mut ratatui::Frame, _rect: ratatui::layout::Rect) -> anyhow::Result<()>;

    fn is_show(&self) -> bool;

    fn is_hide(&self) -> bool {
        !self.is_show()
    }

    fn hide(&mut self){
        self.set_visual(false);
        EventManager::cool_down(Duration::from_millis(100));
    }

    fn show(&mut self){
        self.set_visual(true);
        EventManager::cool_down(Duration::from_millis(100));
    }

}

impl<T: PopItem> Draw for T {
    fn draw(&self, frame: &mut ratatui::Frame, area: Rect) {
        if self.is_show() {
            let _ = self.draw_impl(frame, area);
        }
        
    }

}

impl dyn PopItem {
    pub fn as_item<T: PopItem>(&mut self) -> Option<&mut T> {
        let any_self = self as &mut dyn Any;
        any_self.downcast_mut::<T>()
    }
}

