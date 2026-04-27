use anyhow::{Context, Ok, Result};
use ratatui::Frame;
use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;
use std::time::Duration;
use tmj_core::audio::TrackConfig;
use tmj_core::command::CmdBuffer;
use tmj_core::pathes;
use tmj_core::{
    command::GameCmd,
    event::{GameEvent, handler::EventDispatcher, sender::EventSender},
};
use tracing::info;

use crate::pages::mainmenu::MainScreen;
use crate::{SETTING, utils};
use crate::art::theme;
use crate::audio::AUDIOM;
use crate::audio::Tracks;
use crate::gameflow::GameFlowMgr;
use crate::pages::dialogue::DialogueScene;
use crate::pages::{SAVE_MANAGER, Screen, UserScreen};
use crate::utils::ConstInfo;

pub struct Game {
    pub game_flow: RefCell<GameFlowMgr>,
}

impl Game {
    pub fn new() -> Game {
        let mut gameflow = GameFlowMgr::new();
        let _ = gameflow
            .ensure(UserScreen::Main.to_string())
            .inspect_err(|e| eprintln!("{:?}", e));

        let _ = gameflow
            .go_screen(&UserScreen::Main.to_string())
            .inspect_err(|e| eprintln!("Game Main Sceen Set Failded! Game Init Failed!: {e}"));

        // 初始化音频轨道
        AUDIOM.with_borrow_mut(|a| {
            a.create_track(
                Tracks::Bgm,
                Tracks::Bgm.to_string(),
                TrackConfig {
                    looped: true,
                    default_fade_duration: Duration::from_millis(800),
                    ..Default::default()
                },
            );
            a.create_track(
                Tracks::Voice,
                Tracks::Voice.to_string(),
                TrackConfig {
                    looped: false,
                    default_fade_duration: Duration::from_millis(10),
                    ..Default::default()
                },
            );
            a.create_track(
                Tracks::EnvEffect,
                Tracks::EnvEffect.to_string(),
                TrackConfig {
                    looped: true,
                    default_fade_duration: Duration::from_millis(200),
                    ..Default::default()
                },
            );
        });

        // 将要用的脚本环境常量写入文件
        let consts: Vec<_> = inventory::iter::<ConstInfo>.into_iter().collect();
        use std::io::Write;
        let output = std::fs::File::create(pathes::path("script_env.txt")).unwrap();
        for info in consts {
            writeln!(&output, "{}::{}", info.module, info.value).unwrap();
        }

        // 预处理脚本
        for origin_script in &SETTING.preprogress_script {
            let o_path = pathes::path(origin_script);
            let t_path = PathBuf::from("resource")
                .join(PathBuf::from(o_path.file_name().unwrap()).with_extension("fss"));
            match utils::preparse_script(&o_path, &t_path, None) {
                Err(e) => tracing::error!("{:?}", e),
                _ => {},
            };
        }

        Game {
            game_flow: RefCell::new(gameflow),
        }
    }

    fn on_cmd_save(&mut self, id: u8) -> anyhow::Result<()> {
        let binding = SAVE_MANAGER.with(|m| m.clone());
        let mut binding = binding.borrow_mut();
        let slot = binding.get_slot(id.into())?;
        let _ = slot.ensure_slot_path();
        tracing::info!("save slot path {:?}", slot.path);
        if slot.path.is_some() {
            let screen = match self
                .game_flow
                .borrow_mut()
                .get_scene(&UserScreen::Dialogue.to_string())
            {
                Some(_screen) => _screen,
                None => anyhow::bail!("No Dialouge Screen"),
            };
            let mut screen = screen.borrow_mut();
            let screen = screen.as_screen::<DialogueScene>().unwrap();
            let save_str = screen.save_to()?;
            std::fs::write(slot.path.clone().unwrap(), save_str)?;
        } else {
            return anyhow::bail!("on_cmd_save save path not exist: {:?}", slot);
        };
        Ok(())
    }

    fn on_cmd_load(&mut self, id: u8) -> anyhow::Result<()> {
        let binding = SAVE_MANAGER.with(|m| m.clone());
        let mut binding = binding.borrow_mut();
        let slot = binding.get_slot(id.into())?;
        tracing::info!("load slot path {:?}", slot.path);
        if slot.path.is_some() {
            let save_str = std::fs::read_to_string(slot.path.clone().unwrap())?;
            let screen = self
                .game_flow
                .borrow_mut()
                .ensure(UserScreen::Dialogue.to_string())?;
            let mut screen = screen.borrow_mut();
            let screen = screen.as_screen::<DialogueScene>().unwrap();
            screen.load_from(save_str)?;
            CmdBuffer::push(GameCmd::GoScene(UserScreen::Dialogue.to_string()));
        } else {
            return anyhow::bail!("on_cmd_load path not exist: {:?}", slot);
        };
        Ok(())
    }

    fn on_cmd_go_screen(
        &mut self,
        name: String,
    ) -> anyhow::Result<Rc<RefCell<Box<dyn Screen + 'static>>>> {
        let res = if self.game_flow.borrow_mut().get_scene(&name).is_none() {
            let ins = self.game_flow.borrow_mut().ensure(name.clone())?;
            self.game_flow.borrow_mut().go_screen(&name)?;
            ins
        } else {
            self.game_flow.borrow_mut().go_screen(&name)?;
            self.game_flow.borrow_mut().ensure(name.clone())?
        };
        // attention!: 此处为特殊处理, 一般去往主菜单时脱离游戏环境没有后退需要
        if name == UserScreen::Main.to_string() {
            self.game_flow.borrow_mut().clear_jump_path();
        }
        Ok(res)
    }

    fn on_cmd_go_back_screen(&mut self) -> anyhow::Result<String> {
        let pre = self
            .game_flow
            .borrow_mut()
            .go_back_screen()
            .context("Cmd GoBack execute failed!!")?;
        Ok(pre)
    }

    pub fn handle_cmd(&mut self, cmd: &GameCmd) -> anyhow::Result<bool> {
        info!("{}", cmd);
        match cmd {
            GameCmd::GoScene(name) => {
                self.on_cmd_go_screen(name.to_string())?;
            }
            GameCmd::GoBack => {
                self.on_cmd_go_back_screen()?;
            }
            GameCmd::QuitGame => {
                EventSender::sender_event(GameEvent::QuitGame)?;
            }
            GameCmd::SaveTo(slot) => match slot {
                tmj_core::command::SaveSlot::Temp => {}
                tmj_core::command::SaveSlot::Slots(id) => {
                    self.on_cmd_save(*id)?;
                }
            },
            GameCmd::LoadFrom(slot) => match slot {
                tmj_core::command::SaveSlot::Temp => {}
                tmj_core::command::SaveSlot::Slots(id) => {
                    self.on_cmd_load(*id)?;
                }
            },
            _ => {}
        };

        Ok(true)
    }

    pub fn draw(&self, frame: &mut Frame) {
        let screen = self.game_flow.borrow_mut().cur_screen().unwrap();
        let area = frame.area();
        let area = area.centered(
            ratatui::layout::Constraint::Length(SETTING.resolution.0),
            ratatui::layout::Constraint::Length(SETTING.resolution.1),
        );
        frame.buffer_mut().set_style(area, theme::THEME.root);

        screen.borrow_mut().draw(frame, area);
    }
}

impl EventDispatcher for Game {
    fn handle_tick(&mut self, tick: std::time::Duration) {
        AUDIOM.with_borrow_mut(|a| {
            a.update(tick);
        });
        if self.game_flow.borrow_mut().cur_screen().is_none() {
            panic!("None Sceen be set to flow!");
        }
        let screen = self.game_flow.borrow_mut().cur_screen().unwrap();
        screen.borrow_mut().handle_tick(tick);
    }

    fn on_quit(&mut self) {
        self.game_flow.borrow_mut().force_quit();
    }

    fn handle_event(&mut self, event: &GameEvent) -> Result<bool> {
        match event {
            GameEvent::CtKeyEvent(key) => self.on_key(key),
            GameEvent::CtMouseEvent(mouse) => self.on_mouse(mouse),
            GameEvent::QuitGame => self.on_quit(),
            GameEvent::ResizeTerm(w, h) => self.on_resize(*w, *h),
            _ => (),
        }

        let screen = self.game_flow.borrow().cur_screen();

        if screen.is_none() {
            panic!("None Sceen be set to flow!");
        }
        screen.unwrap().borrow_mut().handle_event(event)
    }
}
