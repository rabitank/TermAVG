// src/script/session.rs
use std::{rc::Rc, cell::RefCell};
use tracing::info;
use crate::script::{ScriptContext, Command, task_queue::{TaskQueue, QueueStatus}, WaitCondition};
use crate::script::command_executor::InputEvent;

#[derive(Debug, Clone)]
pub enum SessionStatus {
    Running,
    Blocked(WaitCondition),
    Completed,
    Error(String),
}

pub struct SessionExecutor {
    pub session_id: usize,
    task_queue: TaskQueue,
    pub is_completed: bool,
}

impl SessionExecutor {
    pub fn new(session_id: usize, commands: Vec<Command>) -> Self {
        info!("SessionExecutor: create session {} with {} commands", session_id, commands.len());

        let mut queue = TaskQueue::new();
        queue.extend(commands);

        SessionExecutor {
            session_id,
            task_queue: queue,
            is_completed: false,
        }
    }

    /// 执行命令逻辑
    pub fn step(&mut self, context: &Rc<RefCell<ScriptContext>>) -> SessionStatus {
        if self.is_completed {
            return SessionStatus::Completed;
        }

        match self.task_queue.step(context) {
            QueueStatus::Running => SessionStatus::Running,
            QueueStatus::Blocked(condition) => SessionStatus::Blocked(condition),
            QueueStatus::Completed => {
                info!("SessionExecutor: session {} completed", self.session_id);
                self.is_completed = true;
                SessionStatus::Completed
            }
            QueueStatus::Error(e) => {
                info!("SessionExecutor: session {} error: {}", self.session_id, e);
                self.is_completed = true;
                SessionStatus::Error(e)
            }
        }
    }

    /// 更新时间等待
    pub fn update(&mut self, delta_time: f64, context: &Rc<RefCell<ScriptContext>>) {
        if self.is_completed {
            return;
        }

        if self.task_queue.update(delta_time, context) {
            info!("SessionExecutor: time wait completed in session {}", self.session_id);
        }
    }

    /// 处理输入事件
    pub fn handle_event(&mut self, event: &InputEvent, context: &Rc<RefCell<ScriptContext>>) {
        if self.is_completed {
            return;
        }

        if self.task_queue.handle_event(event, context) {
            info!("SessionExecutor: event handled in session {}: {:?}", self.session_id, event);
        }
    }

    pub fn is_completed(&self) -> bool {
        self.is_completed || self.task_queue.is_completed()
    }

    pub fn is_paused(&self) -> bool {
        self.task_queue.is_paused()
    }

    pub fn blocking_wait_condition(&self) -> Option<WaitCondition> {
        self.task_queue.blocking_wait_condition()
    }

    pub fn min_remaining_time(&self) -> Option<f64> {
        self.task_queue.min_remaining_time()
    }
}
