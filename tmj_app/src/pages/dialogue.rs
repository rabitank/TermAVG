use anyhow::Context;
use ratatui::buffer::Buffer;
use ratatui::crossterm::event::{KeyCode, KeyModifiers, MouseButton, MouseEventKind};
use ratatui::layout::{Constraint, Rect};
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use std::rc::Rc;
use tmj_core::command::{CmdBuffer, GameCmd};
use tmj_core::pathes;
use tmj_core::script::{
    ContextRef, Interpreter, InterpreterStatus, ScriptContext, ScriptParser, SerializableContext,
    TypeName,
};
use tmj_core::event::handler::EventDispatcher;
use tracing::info;

use crate::SETTING;
use crate::audio::AUDIOM;
use crate::pages::pipeline::{
    BackgrondStage, CharactersStage, DialogueFrameStage, FaceStage, PipeStage,
};

use crate::pages::pop_items::CmdInputItem;
use crate::pages::pop_items::PopItem;
use crate::pages::script_reader::{SectionReadResult, StreamSectionReader};
use crate::pages::{Draw, Screen, UserScreen};


pub struct DialogueScene {
    frame: usize,
    pub hide_dialouge: bool, // bool
    session_id: usize,
    script_reader: StreamSectionReader,
    interpreter: Rc<RefCell<Interpreter>>,
    #[cfg(debug_assertions)]
    cmd_input: Option<CmdInputItem>,
}
impl Screen for DialogueScene {}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DialogueSceneSave {
    pub session_id: usize,
    pub ctx: SerializableContext,
}

impl DialogueScene {
    pub fn spawn(_name_args: std::collections::HashMap<&str, &str>) -> Self {
        let _ = pathes::ensure_dir("resource");
        let ctx = ScriptContext::new();
        let ctx = Rc::new(RefCell::new(ctx));
        super::script_def::init_env(ctx.clone());
        let interpreter = Rc::new(RefCell::new(Interpreter::new(ctx)));

        let script_path = SETTING.entre_script_path().unwrap();
        let script_reader = StreamSectionReader::new(script_path, 1024).unwrap();
        let scene = DialogueScene {
            frame: 0,
            hide_dialouge: false,
            session_id: 0,
            script_reader,
            interpreter,
            #[cfg(debug_assertions)]
            cmd_input: None,
        };
        scene
    }

    pub fn get_interpreter(&mut self) -> Rc<RefCell<Interpreter>> {
        self.interpreter.clone()
    }
}

impl DialogueScene {
    pub fn save_to(&self) -> anyhow::Result<String> {
        let ctx = self.interpreter.borrow().context();
        let ctx = ScriptContext::serialize(&ctx);
        let save = DialogueSceneSave {
            session_id: self.session_id,
            ctx,
        };
        let res = toml::to_string(&save)?;
        Ok(res)
    }

    pub fn load_from(&mut self, save_str: String) -> anyhow::Result<()> {
        let save = toml::from_str::<DialogueSceneSave>(&save_str)
            .context("DialougeScene SaveStr Deserialize failed")?;
        self.session_id = save.session_id;
        let ctx = save.ctx;
        ScriptContext::deserialize(&self.interpreter.borrow_mut().context(), ctx)
            .map_err(|e| anyhow::anyhow!(e))?;
        Ok(())
    }
}

fn stage_draw_call<'a, T>(
    screen: &DialogueScene,
    ctx: &ContextRef,
    buffer: &'a mut Buffer,
    area: Rect,
) -> &'a mut Buffer
where
    T: TypeName + PipeStage,
{
    match T::draw(screen, ctx, buffer, area).context(format!("{} draw failed!", T::TYPE_NAME)) {
        Ok(_) => buffer,
        Err(e) => {
            tracing::error!("{:?}", e);
            buffer
        }
    }
}

impl Draw for DialogueScene {
    fn draw(&self, frame: &mut ratatui::Frame, area: Rect) {
        let interpreter = self.interpreter.borrow_mut();
        let ctx = interpreter.context();
        {
            let buffer = frame.buffer_mut();
            let buffer = stage_draw_call::<BackgrondStage>(&self, &ctx, buffer, area);
            let buffer = stage_draw_call::<CharactersStage>(&self, &ctx, buffer, area);
            let buffer = stage_draw_call::<DialogueFrameStage>(&self, &ctx, buffer, area);
            let _buffer = stage_draw_call::<FaceStage>(&self, &ctx, buffer, area);
        }
        // draw Cmd Input
        #[cfg(debug_assertions)]
        if self.cmd_input.is_some() {
            self.cmd_input.as_ref().unwrap().draw(
                frame,
                area.centered(Constraint::Length(80), Constraint::Length(3)),
            );
        }
    }
}

impl DialogueScene {
    fn toggle_dialouge(&mut self) {
        self.hide_dialouge = !self.hide_dialouge;
    }

    fn apply_current_session(&mut self) -> anyhow::Result<bool> {
        self.interpreter.borrow_mut().end_session();
        let read_res = self
            .script_reader
            .read_section(self.session_id as u64)
            .unwrap_or_else(|e| {
                info!("{}", e.to_string());
                SectionReadResult {
                    content: "".to_string(),
                    is_eof: true,
                }
            });
        let session_text = read_res.content;
        info!("Read script: {}", session_text);

        let session = match ScriptParser::parse_session(
            &session_text,
        ) {
            Ok(s) => s,
            Err(e) => {
                info!("Parse error: {}", e.clone());
                anyhow::bail!(e)
            }
        };

        info!("  Session {}: {} commands", self.session_id, session.len());
        for cmd in &session {
            info!("    - {:?}", cmd);
        }

        self.interpreter.borrow_mut().start_session(session);
        if read_res.is_eof {
            self.end_dialouge()?;
        }
        Ok(read_res.is_eof)
    }

    fn push_dialouge_session(&mut self) -> anyhow::Result<bool> {
        self.session_id += 1;
        self.apply_current_session()
    }

    fn end_dialouge(&mut self) -> anyhow::Result<()> {
        AUDIOM.with_borrow_mut(|a| {
            a.stop_all();
        });

        CmdBuffer::push(GameCmd::GoScene(UserScreen::Main.to_string()));
        self.session_id = 1;
        self.script_reader.reset()?;
        self.apply_current_session()?;
        Ok(())
    }
}

impl EventDispatcher for DialogueScene {
    fn on_key(&mut self, key: &ratatui::crossterm::event::KeyEvent) {
        #[cfg(debug_assertions)]
        if self.cmd_input.is_some() && self.cmd_input.as_ref().unwrap().is_show() {
            self.cmd_input.as_mut().unwrap().on_key(key);
            return;
        }

        if key.is_press() {
            return;
        }
        match key.code {
            KeyCode::Enter | KeyCode::Backspace => {
                let _ = self.push_dialouge_session().or_else(|x| {
                    info!("On Key next session faild: {}", x);
                    CmdBuffer::push(GameCmd::GoScene(UserScreen::Main.to_string()));
                    Err(x)
                });
            }
            KeyCode::Char('.')
                if key.modifiers.contains(KeyModifiers::CONTROL) && key.is_release() =>
            {
                #[cfg(debug_assertions)]
                {
                    if self.cmd_input.is_none() {
                        let interpreter: Rc<RefCell<tmj_core::script::Interpreter>> =
                            self.get_interpreter();
                        self.cmd_input = Some(CmdInputItem::new(interpreter));
                    } else {
                        self.cmd_input.as_mut().unwrap().show();
                    }
                }
            }
            KeyCode::Esc | KeyCode::Char('q') => {
                CmdBuffer::push(GameCmd::SaveTo(tmj_core::command::SaveSlot::Temp));
                CmdBuffer::push(GameCmd::GoScene(UserScreen::Main.to_string()));
            }
            KeyCode::Char('s') => {
                CmdBuffer::push(GameCmd::GoScene(UserScreen::Save.to_string()));
            }
            KeyCode::Char('l') => {
                CmdBuffer::push(GameCmd::GoScene(UserScreen::Load.to_string()));
            }
            KeyCode::Char('h') => {
                self.toggle_dialouge();
            }
            _ => {}
        }
    }

    fn on_quit(&mut self) {
        CmdBuffer::push(GameCmd::SaveTo(tmj_core::command::SaveSlot::Temp));
    }

    fn on_mouse(&mut self, mouse: &ratatui::crossterm::event::MouseEvent) {
        if mouse.kind.is_down() {
            return;
        }
        match mouse.kind {
            MouseEventKind::Up(btn) => {
                if btn == MouseButton::Left {
                    self.push_dialouge_session();
                } else if btn == MouseButton::Right {
                    self.toggle_dialouge();
                }
            }
            _ => {}
        }
    }

    fn on_resize(&mut self, _w: u16, _h: u16) {}

    fn handle_tick(&mut self, tick: std::time::Duration) {
        let mut interpreter = self.interpreter.borrow_mut();
        self.frame += 1;

        match interpreter.update(tick.as_secs_f64()) {
            InterpreterStatus::Running => {}
            InterpreterStatus::Waiting(cond) => {
                self.frame += 1;
                info!("Frame {}: Waiting for {:?}", self.frame, cond);

                match cond {
                    tmj_core::script::WaitCondition::Time(t) => {
                        info!("  (Auto-continue after {}s)", t);
                    }
                    _ => {}
                }
            }
            InterpreterStatus::SessionEnd => {
                // if is auto
                //self.try_start_next_session();
            }
            _ => {}
        }
    }
}
