use anyhow::Context;
use std::{cell::RefCell, collections::HashMap, rc::Rc, time::Duration};
use tmj_core::event::EventManager;

use crate::pages::{Screen, UserScreen};

pub type NamedArgs = HashMap<String, String>;
pub type ScreenRef = Rc<RefCell<Box<dyn Screen>>>;

pub struct GameFlowMgr {
    cur_scene: String,
    scenes: HashMap<String, Rc<RefCell<Box<dyn Screen>>>>,
    active_states: HashMap<String, NamedArgs>,
    state_str: String,
    jump_stack: Vec<String>,
}

impl GameFlowMgr {
    pub fn pre_active_states_set(&mut self, screen_name: String, name_args: NamedArgs) {
        self.active_states.insert(screen_name, name_args);
    }

    pub fn ensure(&mut self, screen_name: String) -> anyhow::Result<ScreenRef> {
        match self.scenes.get(&screen_name) {
            Some(s) => Ok(s.clone()),
            None => {
                let screen_type: UserScreen = screen_name.parse::<UserScreen>()?;
                let screen = Rc::new(RefCell::new(screen_type.spawn()?));
                self.scenes.insert(screen_name, screen.clone());
                Ok(screen)
            }
        }
    }

    fn set_current(&mut self, screen_name: &String) -> anyhow::Result<()> {
        let _screen = self
            .ensure(screen_name.clone())
            .context("gameflow set screen failed, screen not exist!")?;
        if self.cur_scene != *screen_name {
            // 1. 处理screen激活和休眠
            let sleep_res = if !self.cur_scene.is_empty() {
                let pre_screen = self.ensure(self.cur_scene.clone())?;
                Some(pre_screen.borrow_mut().sleep()?)
            } else {
                None
            };
            let active_res = _screen.borrow_mut().active(
                self.active_states
                    .entry(screen_name.clone())
                    .or_insert(HashMap::new()),
            )?;
            let res_vec = if sleep_res.is_none() {
                vec![(screen_name.clone(), active_res)]
            } else {
                vec![
                    (self.cur_scene.clone(), sleep_res.unwrap()),
                    (screen_name.clone(), active_res),
                ]
            };
            // 处理回调
            for (name, resp) in res_vec {
                if let Some(active_op) = resp.active_state_modifier {
                    let active_state = self.active_states.entry(name).or_insert(HashMap::new());
                    active_op(active_state)?;
                }

                if let Some(game_flow_op) = resp.game_flow_modifier {
                    game_flow_op(self)?;
                }
            }
        }

        EventManager::cool_down(Duration::from_millis(300));
        self.cur_scene = screen_name.clone();
        Ok(())
    }

    pub fn go_back_screen(&mut self) -> anyhow::Result<String> {
        let pre_screen = self
            .jump_stack
            .pop()
            .unwrap_or(UserScreen::Main.to_string());
        self.set_current(&pre_screen)?;
        Ok(pre_screen)
    }

    pub fn go_screen(&mut self, screen_name: &String) -> anyhow::Result<()> {
        self.set_current(screen_name)?;
        self.jump_stack.push(screen_name.clone());
        Ok(())
    }

    pub fn clear_jump_path(&mut self) {
        self.jump_stack.clear();
    }

    pub fn set_scene(&mut self, screen_name: String, scene: Box<dyn Screen>) {
        self.scenes
            .insert(screen_name, Rc::new(RefCell::new(scene)));
    }

    pub fn get_scene(&mut self, screen_name: &String) -> Option<Rc<RefCell<Box<dyn Screen>>>> {
        self.scenes.get(screen_name).cloned()
    }

    pub fn new() -> GameFlowMgr {
        GameFlowMgr {
            cur_scene: "".to_string(),
            scenes: HashMap::new(),
            active_states: HashMap::new(),
            state_str: "initing".to_string(),
            jump_stack: Vec::with_capacity(10),
        }
    }

    pub fn force_quit(&mut self) {
        self.state_str = "ready_quit".to_string();
    }
    pub fn is_ready_quit(&self) -> bool {
        self.state_str == "ready_quit".to_string()
    }

    pub fn cur_screen(&self) -> Option<Rc<RefCell<Box<dyn Screen>>>> {
        self.scenes.get(&self.cur_scene).cloned()
    }
}
