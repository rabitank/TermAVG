use anyhow::Context;
use ratatui::crossterm::event::{KeyCode, KeyModifiers, MouseButton, MouseEventKind};
use ratatui::layout::{Constraint, Rect};
use serde::{Deserialize};
use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;
use tmj_core::command::{CmdBuffer, GameCmd};
use tmj_core::event::handler::EventDispatcher;
use tmj_core::pathes;
use tmj_core::script::{
    Interpreter, InterpreterStatus, ScriptContext, ScriptParser, SerializableContext,
};
use tracing::info;

use crate::audio::{AUDIOM, load_audio};
use crate::pages::pipeline::ChapterStage;
use crate::pages::pipeline::{
    BackgroundStage, CharactersStage, DialogueFrameStage, FaceStage, LayersStage, ParagraphStage,
    RenderVeStage,
    visual_element::{VisualElement, VisualElementKind},
};
use crate::{SETTING, audio};

use crate::pages::pop_items::CmdInputItem;
use crate::pages::pop_items::PopItem;
use crate::pages::script_def::var_bgm;
use crate::pages::script_reader::{SectionReadResult, StreamSectionReader};
use crate::pages::{Draw, Screen, ScreenActRespond, UserScreen};

thread_local! {
    static LAST_VE_SNAPSHOT: RefCell<Vec<VisualElement>> = const { RefCell::new(Vec::new()) };
}

fn visual_element_debug_dump(ve: &VisualElement) -> String {
    let kind = match &ve.kind {
        VisualElementKind::Image { source } => format!("Image(source={source})"),
        VisualElementKind::Text { content } => {
            format!("Text(len={}, content={content:?})", content.chars().count())
        }
        VisualElementKind::Fill => "Fill".to_string(),
        VisualElementKind::Custom { .. } => "Custom".to_string(),
    };
    format!(
        "name={:?}, visible={}, is_animated={}, z_index={}, rect=({}, {}, {}, {}), clear_before_draw={}, use_typewriter={}, typewriter_speed={}, kind={}",
        ve.name,
        ve.visible,
        ve.is_animated,
        ve.z_index,
        ve.rect.x,
        ve.rect.y,
        ve.rect.width,
        ve.rect.height,
        ve.clear_before_draw,
        ve.use_typewriter,
        ve.typewriter_speed,
        kind
    )
}

pub fn see_visual_element(name: &str) -> anyhow::Result<()> {
    let message = LAST_VE_SNAPSHOT.with_borrow(|elements| {
        elements
            .iter()
            .find(|ve| ve.name == name)
            .map(visual_element_debug_dump)
            .unwrap_or_else(|| format!("see: visual element not found: {name}"))
    });
    println!("{message}");
    tracing::info!("{message}");
    Ok(())
}

pub struct DialogueScene {
    frame: usize,
    pub last_tick_secs: f64,
    pub hide_dialouge: bool, // bool
    session_id: usize,
    script_reader: StreamSectionReader,
    interpreter: Rc<RefCell<Interpreter>>,
    visual_elements: RefCell<Vec<VisualElement>>,
    last_draw_area: RefCell<Option<Rect>>,
    need_rebuild_ve: RefCell<bool>,
    #[cfg(debug_assertions)]
    cmd_input: Option<CmdInputItem>,
}

impl DialogueScene {
    fn init_audio(&self) -> anyhow::Result<()> {
        let bgm_path = format!("{}.{}", var_bgm::BGM, var_bgm::SOURCE);
        let bgm_path = self
            .get_interpreter()
            .borrow()
            .context()
            .borrow()
            .get_val(&bgm_path)
            .unwrap();

        AUDIOM.with_borrow_mut(|a| {
            if bgm_path.is_string() && !bgm_path.as_string().unwrap().is_empty() {
                let source = load_audio(bgm_path.as_string().unwrap())?;
                a.track_mut(&audio::Tracks::Bgm)
                    .unwrap()
                    .fade_in(source, Duration::from_millis(100));
            }
            Ok(())
        })
    }

    fn stop_audio(&self) -> anyhow::Result<()> {
        AUDIOM.with_borrow_mut(|a| {
            a.stop_all();
            Ok(())
        })
    }
}
impl Screen for DialogueScene {
    fn active(
        &mut self,
        _named_args: &crate::gameflow::NamedArgs,
    ) -> anyhow::Result<super::ScreenActRespond> {
        self.init_audio()?;
        let resp = ScreenActRespond::default();
        Ok(resp)
    }

    fn sleep(&mut self) -> anyhow::Result<super::ScreenActRespond> {
        self.stop_audio()?;
        let resp = ScreenActRespond::default();
        Ok(resp)
    }
}

#[derive(Debug, Clone, serde::Serialize, Deserialize)]
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
            last_tick_secs: 0.0,
            hide_dialouge: false,
            session_id: 0,
            script_reader,
            interpreter,
            visual_elements: RefCell::new(Vec::new()),
            last_draw_area: RefCell::new(None),
            need_rebuild_ve: RefCell::new(true),
            #[cfg(debug_assertions)]
            cmd_input: None,
        };
        scene
    }

    pub fn get_interpreter(&self) -> Rc<RefCell<Interpreter>> {
        self.interpreter.clone()
    }
}

impl DialogueScene {
    fn update_all_stage_elements(
        &self,
        ctx: &tmj_core::script::ContextRef,
        elements: &mut Vec<VisualElement>,
        _area: Rect,
    ) -> anyhow::Result<()> {
        BackgroundStage::update_elements(ctx, elements).context("background update failed")?;
        DialogueFrameStage::update_elements(self, ctx, elements)
            .context("dialogue frame update failed")?;
        FaceStage::update_elements(self, ctx, elements).context("face update failed")?;
        ParagraphStage::update_elements(self, ctx, elements)
            .context("paragraph update failed")?;
        CharactersStage::update_elements(ctx, elements).context("character update failed")?;
        LayersStage::update_elements(ctx, elements).context("layers update failed")?;
        ChapterStage::update_elements(ctx, elements).context("chapter title update failed")?;
        Ok(())
    }

    fn rebuild_visual_elements(&self, area: Rect) -> anyhow::Result<()> {
        let ctx = self.interpreter.borrow().context();
        let mut elements = self.visual_elements.borrow_mut();
        let mut rebuilt = Vec::new();
        rebuilt.extend(BackgroundStage::build_elements(&ctx)?);
        rebuilt.extend(DialogueFrameStage::build_elements());
        rebuilt.extend(FaceStage::build_elements());
        rebuilt.extend(ParagraphStage::build_elements());
        rebuilt.extend(CharactersStage::build_elements(&ctx)?);
        rebuilt.extend(LayersStage::build_elements(&ctx)?);
        rebuilt.extend(ChapterStage::build_elements(&ctx)?);
        *elements = rebuilt;
        self.update_all_stage_elements(&ctx, &mut elements, area)?;
        for ve in elements.iter_mut() {
            ve.apply_props();
        }
        Ok(())
    }

    pub fn save_to(&self) -> anyhow::Result<String> {
        let ctx = self.interpreter.borrow().context();
        let ctx = ScriptContext::serialize(&ctx);
        let save = DialogueSceneSave {
            session_id: self.session_id,
            ctx,
        };
        let res = json5::to_string(&save).context("save json serialize save failed")?;
        Ok(res)
    }

    pub fn load_from(&mut self, save_str: String) -> anyhow::Result<()> {
        let save = json5::from_str::<DialogueSceneSave>(&save_str)
            .context("DialougeScene SaveStr Deserialize failed")?;
        self.session_id = save.session_id;
        let ctx = save.ctx;
        ScriptContext::deserialize(&self.interpreter.borrow_mut().context(), ctx)
            .map_err(|e| anyhow::anyhow!(e))?;
        *self.need_rebuild_ve.borrow_mut() = true;
        Ok(())
    }
}

impl Draw for DialogueScene {
    fn draw(&self, frame: &mut ratatui::Frame, area: Rect) {
        *self.last_draw_area.borrow_mut() = Some(area);
        // VE are generated only with explicit game-computed draw area.
        if *self.need_rebuild_ve.borrow() {
            if let Err(e) = self.rebuild_visual_elements(area) {
                tracing::error!("rebuild visual elements failed: {:?}", e);
            } else {
                *self.need_rebuild_ve.borrow_mut() = false;
            }
        }
        let interpreter = self.interpreter.borrow_mut();
        let ctx = interpreter.context();
        {
            let buffer = frame.buffer_mut();
            let mut elements = self.visual_elements.borrow_mut();
            if let Err(e) = self.update_all_stage_elements(&ctx, &mut elements, area) {
                tracing::error!("update visual elements failed: {:?}", e);
            }
            LAST_VE_SNAPSHOT.with_borrow_mut(|snapshot| {
                *snapshot = elements.clone();
            });
            let buffer = match RenderVeStage::draw(&mut elements, buffer, self.last_tick_secs, area) {
                Ok(buf) => buf,
                Err(e) => {
                    tracing::error!("RenderVeStage draw failed: {:?}", e);
                    buffer
                }
            };
            let _buffer = buffer;
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
    fn has_active_animations(&self) -> bool {
        self.visual_elements
            .borrow()
            .iter()
            .any(|ve| ve.is_animated)
    }

    fn apply_props_now(&mut self) {
        for ve in self.visual_elements.borrow_mut().iter_mut() {
            ve.apply_props();
        }
    }

    fn apply_stage_state_now(&mut self) {
        let area = match *self.last_draw_area.borrow() {
            Some(a) => a,
            None => return,
        };
        let ctx = self.interpreter.borrow().context();
        let mut elements = self.visual_elements.borrow_mut();
        if let Err(e) = self.update_all_stage_elements(&ctx, &mut elements, area) {
            tracing::error!("apply stage state failed: {:?}", e);
        }
    }

    fn toggle_dialouge(&mut self) {
        self.hide_dialouge = !self.hide_dialouge;
    }

    fn apply_current_session(&mut self) -> anyhow::Result<bool> {
        self.interpreter.borrow_mut().end_session();
        // Transition contract:
        // 1) force-settle running VE animations,
        // 2) stage writes latest props from script state,
        // 3) force-apply once again.
        self.apply_props_now();
        self.apply_stage_state_now();
        self.apply_props_now();
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

        let session = match ScriptParser::parse_session(&session_text) {
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
        // Do not rebuild or append VE on each session; only apply current state.
        self.apply_stage_state_now();
        if read_res.is_eof {
            self.end_dialouge()?;
        }
        Ok(read_res.is_eof)
    }

    fn push_dialouge_session(&mut self) -> anyhow::Result<bool> {
        // First click during animation only forces VE to settle; no session advance.
        if self.has_active_animations() {
            self.apply_props_now();
            self.apply_stage_state_now();
            self.apply_props_now();
            return Ok(false);
        }
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
        self.last_tick_secs = tick.as_secs_f64();
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
