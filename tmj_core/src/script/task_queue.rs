// src/script/task_queue.rs
use std::{rc::Rc, cell::RefCell, collections::VecDeque};
use crate::script::{Command, ScriptContext, WaitCondition, command_executor::{CommandExecutor, ExecuteStatus, InputEvent}};

pub struct TaskItem {
    executor: CommandExecutor,
    task_id: usize,
}

impl TaskItem {
    pub fn new(command: Command, task_id: usize) -> Self {
        TaskItem {
            executor: CommandExecutor::new(command),
            task_id,
        }
    }
}
//// 队列状态
#[derive(Debug)]
pub enum QueueStatus {
    Running,        // 正在执行
    Blocked(WaitCondition),  // 被阻塞，需要等待事件
    Completed,      // 所有任务完成
    Error(String),  // 错误
}

pub struct TaskQueue {
    pending_tasks: VecDeque<TaskItem>,
    active_tasks: Vec<TaskItem>,
    paused: bool,
    blocking_wait: Option<WaitCondition>,
    task_counter: usize,
}

impl TaskQueue {
    pub fn new() -> Self {
        TaskQueue {
            pending_tasks: VecDeque::new(),
            active_tasks: Vec::new(),
            paused: false,
            blocking_wait: None,
            task_counter: 0,
        }
    }

    pub fn push(&mut self, command: Command) {
        self.task_counter += 1;
        let task = TaskItem::new(command, self.task_counter);
        self.pending_tasks.push_back(task);
    }

    pub fn extend(&mut self, commands: Vec<Command>) {
        for cmd in commands {
            self.push(cmd);
        }
    }

    /// 更新时间等待
    pub fn update(&mut self, delta_time: f64, context: &Rc<RefCell<ScriptContext>>) -> bool {
        let mut any_completed = false;

        for task in &mut self.active_tasks {
            if task.executor.update(delta_time) {
                match task.executor.step(context) {
                    ExecuteStatus::Completed => {
                        any_completed = true;
                    }
                    _ => {}
                }
            }
        }

        self.active_tasks.retain(|task| !task.executor.is_completed());

        if any_completed && self.paused {
            let has_blocking_wait = self.active_tasks
                .iter()
                .any(|task| task.executor.is_blocking() && task.executor.is_waiting());

            if !has_blocking_wait {
                self.paused = false;
                self.blocking_wait = None;
            }
        }

        any_completed
    }

    /// 处理输入事件
    pub fn handle_event(&mut self, event: &InputEvent, context: &Rc<RefCell<ScriptContext>>) -> bool {
        let mut any_resumed = false;

        for task in &mut self.active_tasks {
            if task.executor.handle_event(event) {
                match task.executor.step(context) {
                    ExecuteStatus::Completed => {
                        any_resumed = true;
                    }
                    _ => {}
                }
            }
        }

        self.active_tasks.retain(|task| !task.executor.is_completed());

        if any_resumed && self.paused {
            let has_blocking_wait = self.active_tasks
                .iter()
                .any(|task| task.executor.is_blocking() && task.executor.is_waiting());

            if !has_blocking_wait {
                self.paused = false;
                self.blocking_wait = None;
            }
        }

        any_resumed
    }

    /// 执行命令逻辑
    pub fn step(&mut self, context: &Rc<RefCell<ScriptContext>>) -> QueueStatus {
        self.step_active_tasks(context);

        if self.paused {
            if let Some(ref condition) = self.blocking_wait {
                return QueueStatus::Blocked(condition.clone());
            }
        }

        if !self.paused {
            while let Some(mut task) = self.pending_tasks.pop_front() {
                match task.executor.step(context) {
                    ExecuteStatus::Completed => {
                        continue;
                    }
                    ExecuteStatus::Waiting(condition) => {
                        if task.executor.is_blocking() {
                            self.paused = true;
                            self.blocking_wait = Some(condition.clone());
                            self.active_tasks.push(task);
                            return QueueStatus::Blocked(condition);
                        } else {
                            self.active_tasks.push(task);
                            continue;
                        }
                    }
                    ExecuteStatus::Error(e) => {
                        return QueueStatus::Error(e);
                    }
                }
            }
        }

        if self.pending_tasks.is_empty() && self.active_tasks.is_empty() {
            QueueStatus::Completed
        } else {
            QueueStatus::Running
        }
    }

    fn step_active_tasks(&mut self, context: &Rc<RefCell<ScriptContext>>) {
        for task in &mut self.active_tasks {
            if !task.executor.is_waiting() {
                task.executor.step(context);
            }
        }

        self.active_tasks.retain(|task| !task.executor.is_completed());
    }

    pub fn is_completed(&self) -> bool {
        self.pending_tasks.is_empty() && self.active_tasks.is_empty()
    }

    pub fn is_paused(&self) -> bool {
        self.paused
    }

    pub fn blocking_wait_condition(&self) -> Option<WaitCondition> {
        self.blocking_wait.clone()
    }

    pub fn min_remaining_time(&self) -> Option<f64> {
        self.active_tasks
            .iter()
            .filter_map(|task| task.executor.remaining_time())
            .fold(None, |min, t| {
                Some(min.map_or(t, |m: f64| m.min(t)))
            })
    }

    pub fn clear(&mut self) {
        self.pending_tasks.clear();
        self.active_tasks.clear();
        self.paused = false;
        self.blocking_wait = None;
    }
}
