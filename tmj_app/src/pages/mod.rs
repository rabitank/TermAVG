use ratatui::layout::Rect;
use std::{
    any::Any,
    collections::HashMap,
};
use strum_macros::{Display, EnumString};
use tmj_core::event::handler::EventDispatcher;

pub mod about;
pub mod dialogue;
pub mod load;
pub mod mainmenu;
pub mod save;

pub mod script_def;
pub mod script_reader;
pub mod slot;
pub use slot::SAVE_MANAGER;

use crate::gameflow::{GameFlowMgr, NamedArgs};

pub mod pop_items;
pub use pop_items::UserItems;

pub mod pipeline;

pub trait Draw {
    fn draw(&self, frame: &mut ratatui::Frame, area: Rect);
}

pub struct ScreenActRespond {
    pub active_state_modifier: Option<Box<dyn FnOnce(&mut NamedArgs) -> anyhow::Result<()>>>,
    pub game_flow_modifier: Option<Box<dyn FnOnce(&mut GameFlowMgr) -> anyhow::Result<()>>>,
}

impl Default for ScreenActRespond {
    fn default() -> Self {
        Self {
            active_state_modifier: None,
            game_flow_modifier: None,
        }
    }
}

impl ScreenActRespond {
    pub fn set_as_handle<T>(&mut self, op: T)
    where
        T: FnOnce(&mut NamedArgs) -> anyhow::Result<()> + 'static,
    {
        self.active_state_modifier = Some(Box::new(op));
    }

    pub fn set_gf_handle<T>(&mut self, op: T)
    where
        T: FnOnce(&mut GameFlowMgr) -> anyhow::Result<()> + 'static,
    {
        self.game_flow_modifier = Some(Box::new(op));
    }
}

pub trait Screen: Draw + EventDispatcher + Any + 'static {
    fn active(&mut self, _named_args: &NamedArgs) -> anyhow::Result<ScreenActRespond> {
        let resp = ScreenActRespond::default();
        Ok(resp)
    }

    fn sleep(&mut self) -> anyhow::Result<ScreenActRespond> {
        let resp = ScreenActRespond::default();
        Ok(resp)
    }
}

impl dyn Screen {
    pub fn as_screen<T: Screen>(&mut self) -> Option<&mut T> {
        let any_self = self as &mut dyn Any;
        any_self.downcast_mut::<T>()
    }
}

#[derive(EnumString, Display, Debug)]
pub enum UserScreen {
    Main,
    Dialogue,
    Config,
    Save,
    Load,
    About,
    Setting,
    Review,
    Gallery,
}

impl UserScreen {
    pub fn spawn(&self) -> anyhow::Result<Box<dyn Screen>> {
        let name_args = HashMap::new();
        match *self {
            // 移动到game的一个函数里
            UserScreen::Main => {
                return Ok(Box::new(mainmenu::MainScreen::spawn(name_args)));
            }
            UserScreen::Dialogue => {
                return Ok(Box::new(dialogue::DialogueScene::spawn(name_args)));
            }
            UserScreen::About => {
                return Ok(Box::new(about::AboutScene::spawn(name_args)));
            }
            UserScreen::Save => {
                return Ok(Box::new(save::SaveScreen::spawn(name_args)));
            }
            UserScreen::Load => {
                return Ok(Box::new(load::LoadScreen::spawn(name_args)));
            }
            _ => anyhow::bail!("no screen"),
        };
    }
}

