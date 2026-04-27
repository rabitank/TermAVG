mod cmd;
mod history;
mod history_ls;
pub use history_ls::{HISTORY_LS, DialogueRecord};
pub use history::DialogueHistoryLs;

use std::{
    any::Any, time::Duration,
};

pub use cmd::CmdInputItem;
use tmj_core::event::EventManager;


pub trait PopItem: Any + 'static {

    fn set_visual(&mut self, visual: bool);

    fn draw(&self, _frame: &mut ratatui::Frame, _rect: ratatui::layout::Rect) -> anyhow::Result<()>;

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

impl dyn PopItem {
    pub fn as_item<T: PopItem>(&mut self) -> Option<&mut T> {
        let any_self = self as &mut dyn Any;
        any_self.downcast_mut::<T>()
    }
}

