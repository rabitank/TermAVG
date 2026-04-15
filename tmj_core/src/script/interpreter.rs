// src/script/interpreter.rs
use std::{rc::Rc, cell::RefCell};
use tracing::info;
use crate::script::{Command, CommandExecutor, ScriptContext, WaitCondition, script_parsers::ScriptParser, session::{SessionExecutor, SessionStatus}};
use crate::script::command_executor::InputEvent;

#[derive(Debug, Clone, PartialEq)]
pub enum InterpreterStatus {
    Idle,
    Running,
    Waiting(WaitCondition),
    SessionEnd,
    Error(String),
}

pub struct Interpreter {
    context: Rc<RefCell<ScriptContext>>,
    current_session: Option<SessionExecutor>,
    session_counter: usize, // session 计数用的
    status: InterpreterStatus,
}

impl Interpreter {
    pub fn new(context: Rc<RefCell<ScriptContext>>) -> Self {
        Interpreter {
            context,
            current_session: None,
            session_counter: 0,
            status: InterpreterStatus::Idle,
        }
    }

    pub fn context(&self) -> Rc<RefCell<ScriptContext>> {
        Rc::clone(&self.context)
    }

    // 可以考虑丰富返回内容，将等待的command executor往上提交之类的？
    pub fn eval(commands: Vec<Command>, ctx: Rc<RefCell<ScriptContext>>) -> Result<(), String> {
        for (pos, command ) in commands.into_iter().enumerate() {
            let mut executor = CommandExecutor::new(command.clone());
            match executor.step(&ctx) {
                super::ExecuteStatus::Completed => {},
                super::ExecuteStatus::Waiting(_wait_condition) => {
                    return Err(format!("eval current dont suppor waitting command: {:?}\n line: {}", command, pos));
                },
                super::ExecuteStatus::Error(info) => {
                    return Err(format!("eval failed!, reason: {}\n {:?}\n line: {}", info, command, pos));
                }
            }
        };
        Ok(())
    }

    pub fn eval_new_session(&mut self, session_text: String) -> anyhow::Result<()> {
        let session = match ScriptParser::parse_session(&session_text) {
            Ok(s) => s,
            Err(e) => {
                info!("Parse error: {}", e.clone());
                anyhow::bail!(e)
            }
        };
        info!("eval {} as session", session_text);
        self.start_session(session);
        Ok(())
    }

    pub fn status(&self) -> InterpreterStatus {
        self.status.clone()
    }

    pub fn has_active_session(&self) -> bool {
        self.current_session.as_ref().map_or(false, |s| !s.is_completed())
    }

    pub fn is_waiting(&self) -> bool {
        matches!(self.status, InterpreterStatus::Waiting(_))
    }

    pub fn wait_condition(&self) -> Option<WaitCondition> {
        match &self.status {
            InterpreterStatus::Waiting(c) => Some(c.clone()),
            _ => None,
        }
    }

    /// 开始新 session
    pub fn start_session(&mut self, commands: Vec<Command>) {
        info!("Interpreter: start_session with {} commands", commands.len());

        if let Some(ref session) = self.current_session {
            if !session.is_completed() {
                info!("Interpreter: forcing end of previous session {}", session.session_id);
                self.end_session_internal();
            }
        }

        self.session_counter += 1;
        let session = SessionExecutor::new(self.session_counter, commands);

        {
            let mut ctx = self.context.borrow_mut();
            ctx.start_session();
        }

        self.current_session = Some(session);
        self.status = InterpreterStatus::Running;
    }

    /// 执行命令逻辑 (每帧调用)
    pub fn step(&mut self) -> InterpreterStatus {
        if self.current_session.is_none() {
            self.status = InterpreterStatus::Idle;
            return self.status.clone();
        }

        let session = self.current_session.as_mut().unwrap();

        match session.step(&self.context) {
            SessionStatus::Running => {
                self.status = InterpreterStatus::Running;
            }
            SessionStatus::Blocked(condition) => {
                info!("Interpreter: session blocked, waiting for {:?}", condition);
                self.status = InterpreterStatus::Waiting(condition);
            }
            SessionStatus::Completed => {
                // info!("Interpreter: session {} completed", session.session_id);
                self.status = InterpreterStatus::SessionEnd;
            }
            SessionStatus::Error(e) => {
                info!("Interpreter: session error: {}", e);
                self.end_session_internal();
                self.status = InterpreterStatus::Error(e);
            }
        }

        self.status.clone()
    }

    /// 更新时间等待 (每帧调用，在 step 之前)
    pub fn update(&mut self, delta_time: f64) -> InterpreterStatus {
        if let Some(ref mut session) = self.current_session {
            session.update(delta_time, &self.context);
            return self.step();
        } else {
            self.status.clone()
        }

    }

    /// 处理输入事件 (有事件时调用)
    pub fn handle_event(&mut self, event: InputEvent) -> InterpreterStatus {
        if !self.is_waiting() {
            return self.status.clone();
        }

        let is_event_wait = match &self.status {
            InterpreterStatus::Waiting(c) => c.is_event(),
            _ => false,
        };

        if !is_event_wait {
            return self.status.clone();
        }

        if let Some(ref mut session) = self.current_session {
            session.handle_event(&event, &self.context);
            return self.step();
        }

        self.status.clone()
    }

    pub fn end_session(&mut self) {
        info!("Interpreter: end_session called");
        self.end_session_internal();
        self.status = InterpreterStatus::SessionEnd;
    }

    fn end_session_internal(&mut self) {
        if let Some(ref mut session) = self.current_session {
            let mut ctx = self.context.borrow_mut();
            ctx.end_session();
            session.is_completed = true;
        }
        self.current_session = None;
    }

    pub fn clear(&mut self) {
        info!("Interpreter: clear");
        self.end_session_internal();
        self.session_counter = 0;
        self.status = InterpreterStatus::Idle;

        let mut ctx = self.context.borrow_mut();
        ctx.clear();
    }

    pub fn min_remaining_time(&self) -> Option<f64> {
        self.current_session.as_ref().and_then(|s| s.min_remaining_time())
    }
}

impl Default for Interpreter {
    fn default() -> Self {
        Self::new(Rc::new(RefCell::new(ScriptContext::new())))
    }
}
