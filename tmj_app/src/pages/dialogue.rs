use crate::pages::behaviour::BehaviourMap;
use anyhow::Context;
use rodio::Source;
use ratatui::crossterm::event::{KeyCode, KeyModifiers, MouseButton, MouseEventKind};
use ratatui::layout::Rect;
use serde::Deserialize;
use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;
use tmj_core::audio::{AudioOp, AudioSource};
use tmj_core::command::{CmdBuffer, GameCmd};
use tmj_core::event::handler::EventDispatcher;
use tmj_core::script::{
    DEFAULT_WAIT_SKIP_BUFFER_SECS, Interpreter, InterpreterStatus, ScriptContext, ScriptParser,
    ScriptValue, SerializableContext,
};
use tmj_core::{pathes, script};
use tracing::info;

use crate::audio::{AUDIOM, load_audio};
use crate::pages::behaviour::default_dialogue_ve_stages;
use crate::pages::behaviour::{
    DIALOGUE_VE_STAGE_ORDER, RenderVeStage, logical_area,
    visual_element::{VisualElement, VisualElementKind},
};
use crate::{SETTING, audio};
use ratatui::style::Style;

use crate::pages::pop_items::PopItem;
use crate::pages::pop_items::{
    CmdInputItem, DialogueHistoryLs, GameSettingPopItem, HISTORY_LS, LoadPopItem, PopItemStore,
    SavePopItem,
};
use crate::pages::script_def::var_bgm;
use crate::pages::script_def::var_env_effect;
use crate::pages::script_reader::{SectionReadResult, StreamSectionReader};
use crate::pages::{Draw, Screen, ScreenActRespond, UserScreen};

thread_local! {
    static LAST_VE_SNAPSHOT: RefCell<Vec<VisualElement>> = const { RefCell::new(Vec::new()) };
}

fn visual_element_debug_dump(ve: &VisualElement) -> String {
    let kind = match &ve.kind {
        VisualElementKind::Image { source, .. } => format!("Image(source={source})"),
        VisualElementKind::Text { content } => {
            format!("Text(len={}, content={content:?})", content.chars().count())
        }
        VisualElementKind::Fill => "Fill".to_string(),
        VisualElementKind::Custom { .. } => "Custom".to_string(),
    };
    format!(
        "name={:?}, visible={}, z_index={}, rect=({}, {}, {}, {}), offset=({}, {}), clear_before_draw={}, use_typewriter={}, typewriter_speed={}, kind={}",
        ve.name,
        ve.visible,
        ve.z_index,
        ve.rect.x,
        ve.rect.y,
        ve.rect.width,
        ve.rect.height,
        ve.offset.0,
        ve.offset.1,
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
    popitem_dark_ve: VisualElement,
    pop_items: PopItemStore,
    pre_session_ctx_dump: Option<String>,
}

impl DialogueScene {
    fn init_audio(&self) -> anyhow::Result<()> {
        let interpreter = self.get_interpreter();
        let i = interpreter.borrow();
        let c = i.context();
        let ctx = c.borrow();
        let get_val = |name: &str| -> ScriptValue {
            ctx.get_val(name).unwrap_or(ScriptValue::Nil)
        };

        let bgm_path = get_val(&format!("{}.{}", var_bgm::BGM, var_bgm::M_SOURCE));
        let bgm_vol = get_val(&format!("{}.{}", var_bgm::BGM, var_bgm::M_VOLUME));
        let env_path = get_val(&format!("{}.{}", var_env_effect::ENV_EFFECT, var_env_effect::M_SOURCE));
        let env_vol = get_val(&format!("{}.{}", var_env_effect::ENV_EFFECT, var_env_effect::M_VOLUME));

        let vol = |v: ScriptValue| v.to_number().unwrap_or(1.0) as f32;

        AUDIOM.with_borrow_mut(|a| { let a = a.as_mut().unwrap();
            if bgm_path.is_string() && !bgm_path.as_string().unwrap().is_empty() {
                let source = load_audio(bgm_path.as_string().unwrap())?;
                let source: AudioSource = Box::new(source.amplify(vol(bgm_vol)));
                let t = a.track_mut(&audio::Tracks::Bgm).unwrap();
                t.stop();
                t.fade_in(source, Duration::from_millis(100));
            }
            if env_path.is_string() && !env_path.as_string().unwrap().is_empty() {
                let source = load_audio(env_path.as_string().unwrap())?;
                let source: AudioSource = Box::new(source.amplify(vol(env_vol)));
                if let Some(t) = a.track_mut(&audio::Tracks::EnvEffect) {
                    t.stop();
                    t.queue(AudioOp::play(source, 1.0));
                }
            }
            Ok(())
        })
    }

    fn stop_audio(&self) -> anyhow::Result<()> {
        AUDIOM.with_borrow_mut(|a| { let a = a.as_mut().unwrap();
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
        let resp = ScreenActRespond::default();
        Ok(resp)
    }

    fn sleep(&mut self) -> anyhow::Result<super::ScreenActRespond> {
        // 点完自然退出的时候这里为空,不会去保存
        if self.pre_session_ctx_dump.is_some() {
            CmdBuffer::push(GameCmd::SaveTo(tmj_core::command::SaveSlot::Temp));
        } else {
            tracing::warn!("when dialouge sleep, no pre_session_ctx_dump, skip save temp");
        }
        self.stop_audio()?;
        let resp = ScreenActRespond::default();
        Ok(resp)
    }
}

#[derive(Debug, Clone, serde::Serialize, Deserialize)]
pub struct DialogueSceneSave {
    pub session_id: usize,
    pub ctx_dump: String,
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
            session_id: 1,
            script_reader,
            interpreter,
            script_behaviours: behaviours_map,
            visual_elements: RefCell::new(Vec::new()),
            need_rebuild_ve: RefCell::new(true),
            popitem_dark_ve: VisualElement {
                name: "_".into(),
                alpha: 0.4,
                style: Style::new().bg(crate::art::theme::BLACK),
                rect: logical_area(),
                fill_before_draw: true,
                kind: VisualElementKind::Text { content: "".into() },
                ..Default::default()
            },
            pop_items: PopItemStore::default(),
            pre_session_ctx_dump: None,
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
        let ctx_dump = self.pre_session_ctx_dump.as_ref().ok_or(anyhow::anyhow!(
            "save_to failed: pre_session_ctx_dump is None, cannot guarantee session-start snapshot"
        ))?;
        let save = DialogueSceneSave {
            session_id: self.session_id,
            ctx_dump: ctx_dump.clone(),
        };
        let res = json5::to_string(&save).context("save json serialize save failed")?;
        Ok(res)
    }

    pub fn on_newgame(&mut self) -> anyhow::Result<()> {
        HISTORY_LS.lock().unwrap().clear();
        self.reset_to_begin()?;
        for behaviour in self.script_behaviours.values_mut().values_mut() {
            behaviour.sync_from_ctx(self.interpreter.borrow().context())?;
        }
        self.apply_current_session()?;
        self.init_audio().context("init_audio failed")?;
        Ok(())
    }

    pub fn on_load(&mut self, save_str: String) -> anyhow::Result<()> {
        HISTORY_LS.lock().unwrap().clear();
        self.reset_to_begin()?;
        self.load_from(save_str)?;
        for behaviour in self.script_behaviours.values_mut().values_mut() {
            behaviour.sync_from_ctx(self.interpreter.borrow().context())?;
        }
        self.apply_current_session()
            .context("apply current session failed")?;
        self.init_audio().context("init_audio failed")?;
        Ok(())
    }

    pub fn on_continue(&mut self, save_str: String) -> anyhow::Result<()> {
        self.on_load(save_str)
    }

    pub fn load_from(&mut self, save_str: String) -> anyhow::Result<()> {
        let save = json5::from_str::<DialogueSceneSave>(&save_str)
            .context("DialougeScene SaveStr Deserialize failed")?;
        self.session_id = save.session_id;
        let ctx_dump = save.ctx_dump;
        let ctx = json5::from_str::<SerializableContext>(&ctx_dump)
            .context("load_from parse ctx_dump failed")?;
        self.pre_session_ctx_dump = Some(ctx_dump);
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
        if self.pop_items.has_visible() {
            let _ = self.popitem_dark_ve.render(frame.buffer_mut(), area);
        }
        self.pop_items.draw_visible(frame, area);
    }
}

impl DialogueScene {
    fn is_any_animating(&self) -> bool {
        let animing_bs: Vec<_> = self
            .script_behaviours
            .behaviours
            .borrow()
            // .values()
            .iter()
            .filter(|(_, b)| b.is_animating())
            .map(|(name, _)| name.clone())
            .collect();
        // .any(|b| b.is_animating())
        if !animing_bs.is_empty() {
            tracing::info!("animing behaviours {:?}", animing_bs);
            return true;
        }
        return false;
    }

    fn force_over_all_animations(&mut self) -> anyhow::Result<()> {
        for behaviour in self.script_behaviours.values_mut().values_mut() {
            if behaviour.is_animating() {
                behaviour.on_force_over_animation()?;
            }
        }
        Ok(())
    }

    fn wait_skip_buffer_secs(&self) -> f64 {
        if self.last_tick_secs > 0.0 {
            self.last_tick_secs
        } else {
            DEFAULT_WAIT_SKIP_BUFFER_SECS
        }
    }

    fn toggle_dialouge(&mut self) {
        self.hide_dialouge = !self.hide_dialouge;
    }

    fn load_sessions(&mut self) -> anyhow::Result<(Vec<script::Command>, bool)> {
        tracing::info!("reading script {:?}", SETTING.entre_script);
        let read_res = self
            .script_reader
            .read_section(self.session_id as u64)
            .unwrap_or_else(|e| {
                tracing::error!("{}", e.to_string());
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

    fn validate_session_exists(&self, target_session_id: usize) -> anyhow::Result<()> {
        let script_path = SETTING
            .entre_script_path()
            .context("validate session failed: get script path failed")?;
        let mut reader = StreamSectionReader::new(script_path, 1024)
            .context("validate session failed: create temp reader failed")?;
        reader.read_section(target_session_id as u64).map_err(|e| {
            anyhow::anyhow!(
                "next target session {} not found or unreadable: {}",
                target_session_id,
                e
            )
        })?;
        Ok(())
    }

    fn apply_current_session(&mut self) -> anyhow::Result<bool> {
        self.interpreter.borrow_mut().end_session();
        let ctx = self.interpreter.borrow().context();
        self.pre_session_ctx_dump = Some(
            json5::to_string(&ScriptContext::serialize(&ctx))
                .context("apply_current_session freeze pre_session_ctx_dump failed")?,
        );

        for b in self.script_behaviours.values_mut().values_mut() {
            let ctx = self.interpreter.borrow().context();
            b.on_end_session(ctx)
                .context("behaviour on end session failed")?;
        }

        // 这里只是注入了命令没有step
        let (session, read_to_eof) = self
            .load_sessions()
            .context("apply current session load session failed")?;

        self.interpreter.borrow_mut().start_session(session);

        if read_to_eof {
            CmdBuffer::push(GameCmd::GoScene(UserScreen::Main.to_string()));
            self.reset_to_begin()?;
        }
        Ok(read_to_eof)
    }

    fn on_try_push_dialouge(&mut self) -> anyhow::Result<bool> {
        if self.interpreter.borrow().is_any_executor_waiting() {
            self.interpreter
                .borrow_mut()
                .skip_blocking_waits_with_buffer(self.wait_skip_buffer_secs());
            self.force_over_all_animations()?;
            return Ok(false);
        }
        if self.is_any_animating() {
            self.force_over_all_animations()?;
            return Ok(false);
        }

        let current_session_id = self.session_id;
        let next_session_target = {
            let ctx = self.interpreter.borrow().context();
            ctx.borrow_mut().take_next_session_target()
        };
        let is_script_jump = next_session_target.is_some();
        let target_session_id = next_session_target.unwrap_or(current_session_id + 1);

        if is_script_jump {
            self.validate_session_exists(target_session_id)?;
        }

        self.session_id = target_session_id;
        match self.apply_current_session() {
            Ok(applied) => Ok(applied),
            Err(e) => {
                self.session_id = current_session_id;
                Err(e)
            }
        }
    }

    fn reset_to_begin(&mut self) -> anyhow::Result<()> {
        self.stop_audio()?;

        self.session_id = 1;
        self.script_reader.reset()?;

        for behaviour in self.script_behaviours.values_mut().values_mut() {
            behaviour.on_end_dialouge()?;
        }

        self.visual_elements.borrow_mut().clear();
        *self.need_rebuild_ve.borrow_mut() = true;
        self.pre_session_ctx_dump = None;

        // newgame/load reset should rebuild script globals from init_env,
        // otherwise old runtime variables may leak into the next run.
        self.interpreter.borrow_mut().clear();
        let ctx = self.interpreter.borrow().context();
        super::script_def::init_env(ctx.clone(), self.script_behaviours.clone());
        ctx.borrow_mut()
            .rebuild_tuid_table_from_live()
            .map_err(|e| anyhow::anyhow!(e))?;

        Ok(())
    }
}

impl EventDispatcher for DialogueScene {
    fn on_key(&mut self, key: &ratatui::crossterm::event::KeyEvent) {
        if self.pop_items.dispatch_key_to_top(key) {
            return;
        }
        if key.is_release() {
            return;
        }

        match key.code {
            KeyCode::Enter | KeyCode::Backspace | KeyCode::Char(' ') if key.is_press() => {
                if let Err(e) = self.on_try_push_dialouge() {
                    info!("On key next session failed: {}", e);
                }
            }
            KeyCode::Char('a') | KeyCode::Char('A') => {
                // allow press + repeat for quick skip
                if let Err(e) = self.on_try_push_dialouge() {
                    info!("On key next session failed: {}", e);
                }
            }
            KeyCode::Char('.')
                if key.modifiers.contains(KeyModifiers::CONTROL) && key.is_press() =>
            {
                #[cfg(debug_assertions)]
                {
                    let interpreter: Rc<RefCell<tmj_core::script::Interpreter>> =
                        self.get_interpreter();
                    self.pop_items
                        .get_or_insert_with(|| CmdInputItem::new(interpreter))
                        .show();
                }
            }
            KeyCode::Esc | KeyCode::Char('q') if key.is_press() => {
                CmdBuffer::push(GameCmd::SaveTo(tmj_core::command::SaveSlot::Temp));
                CmdBuffer::push(GameCmd::GoScene(UserScreen::Main.to_string()));
            }
            KeyCode::Char('s') => {
                self.pop_items.get_or_insert_with(SavePopItem::new).show();
            }
            KeyCode::Char('l') => {
                self.pop_items.get_or_insert_with(LoadPopItem::new).show();
            }
            KeyCode::Char('c') => {
                self.pop_items
                    .get_or_insert_with(GameSettingPopItem::new)
                    .show();
            }
            KeyCode::Up => {
                self.pop_items
                    .get_or_insert_with(DialogueHistoryLs::new)
                    .show();
            }
            _ => {}
        }
    }

    fn on_quit(&mut self) {
        CmdBuffer::push(GameCmd::SaveTo(tmj_core::command::SaveSlot::Temp));
    }

    fn on_mouse(&mut self, mouse: &ratatui::crossterm::event::MouseEvent) {
        if self.pop_items.has_visible() {
            return;
        }
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
            behaviour.tick_update(ctx.clone(), tick);
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
