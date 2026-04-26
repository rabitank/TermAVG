use crate::pages::pipeline::BehaviourMap;
use anyhow::Context;
use ratatui::crossterm::event::{KeyCode, KeyModifiers, MouseButton, MouseEventKind};
use ratatui::layout::{Constraint, Rect};
use serde::Deserialize;
use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;
use tmj_core::command::{CmdBuffer, GameCmd};
use tmj_core::event::handler::EventDispatcher;
use tmj_core::script::{
    Interpreter, InterpreterStatus, ScriptContext, ScriptParser, SerializableContext,
};
use tmj_core::{pathes, script};
use tracing::info;

use crate::audio::{AUDIOM, load_audio};
use crate::pages::pipeline::default_dialogue_ve_stages;
use crate::pages::pipeline::{
    DIALOGUE_VE_STAGE_ORDER, RenderVeStage,
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
        "name={:?}, visible={}, z_index={}, rect=({}, {}, {}, {}), clear_before_draw={}, use_typewriter={}, typewriter_speed={}, kind={}",
        ve.name,
        ve.visible,
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
    pub script_behaviours: BehaviourMap,
    visual_elements: RefCell<Vec<VisualElement>>,
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

        for behaviour in self.script_behaviours.values_mut().values_mut() {
            behaviour.on_scene_active(self.interpreter.borrow_mut().context())?;
        }

        self.init_audio()?;
        if self.session_id == 0 {
            self.on_try_push_dialouge()?;
        } else {
            self.apply_current_session();
        }
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
        ctx.borrow_mut().bind_context_ref(ctx.clone());

        let behaviours_map: BehaviourMap = BehaviourMap {
            behaviours: Rc::new(RefCell::new(default_dialogue_ve_stages())),
        };

        super::script_def::init_env(ctx.clone(), behaviours_map.clone());
        ctx.borrow_mut()
            .rebuild_tuid_table_from_live()
            .expect("rebuild tuid_table after init_env");

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
            script_behaviours: behaviours_map,
            visual_elements: RefCell::new(Vec::new()),
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
    fn behaviour_update_ves(
        &self,
        ctx: &tmj_core::script::ContextRef,
        elements: &mut Vec<VisualElement>,
    ) -> anyhow::Result<()> {
        let behaviours = self.script_behaviours.behaviours.borrow();
        for b in behaviours.values() {
            b.update_elements(self, ctx, elements)?;
        }
        Ok(())
    }

    fn rebuild_visual_elements(&self) -> anyhow::Result<()> {
        let ctx = self.interpreter.borrow().context();
        let mut elements = self.visual_elements.borrow_mut();
        let mut rebuilt = Vec::new();
        let behaviours = self.script_behaviours.behaviours.borrow();
        for &name in DIALOGUE_VE_STAGE_ORDER {
            let st = behaviours
                .get(name)
                .with_context(|| format!("missing VE stage: {name}"))?;
            rebuilt.extend(
                st.build_elements(&ctx)
                    .with_context(|| format!("{name} build failed"))?,
            );
        }
        *elements = rebuilt;
        self.behaviour_update_ves(&ctx, &mut elements)?;
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
        let mut elements = self.visual_elements.borrow_mut();
        let buffer = frame.buffer_mut();
        let buffer = match RenderVeStage::draw(&mut elements, buffer, area) {
            Ok(buf) => buf,
            Err(e) => {
                tracing::error!("RenderVeStage draw failed: {:?}", e);
                buffer
            }
        };
        let _buffer = buffer;
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
    fn is_any_animating(&self) -> bool {
        for b in self.script_behaviours.behaviours.borrow().values() {
            if b.is_animating() {
                return true;
            }
        }
        false
        // self.script_behaviours
        //     .behaviours
        //     .borrow()
        //     .values()
        //     .any(|b| b.is_animating())
    }

    fn toggle_dialouge(&mut self) {
        self.hide_dialouge = !self.hide_dialouge;
    }

    fn load_sessions(&mut self) -> anyhow::Result<(Vec<script::Command>, bool)> {
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
                tracing::error!("Parse error: {}", e.clone());
                anyhow::bail!(e)
            }
        };
        info!("  Session {}: {} commands", self.session_id, session.len());
        for cmd in &session {
            info!("    - {:?}", cmd);
        }

        Ok((session, read_res.is_eof))
    }

    fn apply_current_session(&mut self) -> anyhow::Result<bool> {
        self.interpreter.borrow_mut().end_session();
        for b in self.script_behaviours.values_mut().values_mut() {
            let ctx = self.interpreter.borrow().context();
            b.on_end_session(ctx).context("behaviour on end session failed")?;
        }

        // 这里只是注入了命令没有step
        let (session, read_to_eof) = self
            .load_sessions()
            .context("apply current session load session failed")?;

        self.interpreter.borrow_mut().start_session(session);

        if read_to_eof {
            self.end_dialouge()?;
        }
        Ok(read_to_eof)
    }

    fn on_try_push_dialouge(&mut self) -> anyhow::Result<bool> {
        // First click during animation only forces VE to settle; no session advance.
        if self.is_any_animating() {
            for behaviour in self.script_behaviours.values_mut().values_mut() {
                if behaviour.is_animating() {
                    behaviour.on_force_over_animation()?;
                }
            }
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

        for behaviour in self.script_behaviours.values_mut().values_mut() {
            behaviour.on_end_dialouge()?;
        }

        self.visual_elements.borrow_mut().clear();
        *self.need_rebuild_ve.borrow_mut() = true;

        // todo! clear ves, clear interpreter env
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
                let _ = self.on_try_push_dialouge().or_else(|x| {
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
                    match self
                        .on_try_push_dialouge()
                        .context("try push dialouge failed!")
                    {
                        Err(e) => {
                            tracing::error!("{:?}", e);
                        }
                        _ => {}
                    };
                } else if btn == MouseButton::Right {
                    self.toggle_dialouge();
                }
            }
            _ => {}
        }
    }

    fn on_resize(&mut self, _w: u16, _h: u16) {}

    fn handle_tick(&mut self, tick: std::time::Duration) {
        self.last_tick_secs = tick.as_secs_f64();
        self.frame += 1;

        // VE are generated only with explicit game-computed draw area.
        if *self.need_rebuild_ve.borrow() {
            if let Err(e) = self.rebuild_visual_elements() {
                tracing::error!("rebuild visual elements failed: {:?}", e);
            } else {
                *self.need_rebuild_ve.borrow_mut() = false;
            }
        }

        let mut interpreter = self.interpreter.borrow_mut();
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
        let ctx = interpreter.context();

        for behaviour in self.script_behaviours.behaviours.borrow_mut().values_mut() {
            behaviour.tick_update(ctx.clone(),tick);
        }

        let mut elements = self.visual_elements.borrow_mut();
        if let Err(e) = self.behaviour_update_ves(&ctx, &mut elements) {
            tracing::error!("update visual elements failed: {:?}", e);
        }

        LAST_VE_SNAPSHOT.with_borrow_mut(|snapshot| {
            *snapshot = elements.clone();
        });
    }
}
