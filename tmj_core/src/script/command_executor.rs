// src/script/command_executor.rs
use crate::script::{
    Command, CommandBlockType, ContextRef, ScriptContext, ScriptValue, WaitCondition,
};
use std::{cell::RefCell, rc::Rc};

#[derive(Debug, Clone)]
pub enum ExecuteStatus {
    Completed,
    Waiting(WaitCondition),
    Error(String),
}

pub struct CommandExecutor {
    command: Command,
    block_type: CommandBlockType,
    state: ExecutorState,
    /// 时间等待的剩余时间
    time_remaining: Option<f64>,
}

enum ExecutorState {
    Ready,
    Waiting(WaitCondition),
    Completed,
    Error(String),
}

impl CommandExecutor {
    pub fn new(command: Command) -> Self {
        let block_type = command.block_type();
        CommandExecutor {
            command,
            block_type,
            state: ExecutorState::Ready,
            time_remaining: None,
        }
    }

    pub fn step(&mut self, context: &ContextRef) -> ExecuteStatus {
        match &self.state {
            ExecutorState::Ready => self.execute(context),
            ExecutorState::Waiting(condition) => ExecuteStatus::Waiting(condition.clone()),
            ExecutorState::Completed => ExecuteStatus::Completed,
            ExecutorState::Error(e) => ExecuteStatus::Error(e.clone()),
        }
    }

    /// 更新时间等待
    pub fn update(&mut self, delta_time: f64) -> bool {
        if let ExecutorState::Waiting(WaitCondition::Time(total)) = &self.state {
            let remaining = self.time_remaining.get_or_insert(*total);
            *remaining -= delta_time;

            if *remaining <= 0.0 {
                // 时间到期，继续执行
                self.state = ExecutorState::Ready;
                return true;
            }
        }
        false
    }

    /// 处理事件
    pub fn handle_event(&mut self, event: &InputEvent) -> bool {
        if let ExecutorState::Waiting(condition) = &self.state {
            let should_resume = match condition {
                WaitCondition::Click => matches!(event, InputEvent::Click),
                WaitCondition::Input(expected) => {
                    matches!(event, InputEvent::Text(text) if text == expected)
                }
                WaitCondition::Any(conditions) => conditions.iter().any(|c| match c {
                    WaitCondition::Click => matches!(event, InputEvent::Click),
                    WaitCondition::Input(expected) => {
                        matches!(event, InputEvent::Text(text) if text == expected)
                    }
                    _ => false,
                }),
                _ => false,
            };

            if should_resume {
                self.state = ExecutorState::Ready;
                return true;
            }
        }
        false
    }

    pub fn is_waiting(&self) -> bool {
        matches!(self.state, ExecutorState::Waiting(_))
    }

    pub fn wait_condition(&self) -> Option<WaitCondition> {
        match &self.state {
            ExecutorState::Waiting(c) => Some(c.clone()),
            _ => None,
        }
    }

    pub fn is_completed(&self) -> bool {
        matches!(self.state, ExecutorState::Completed)
    }

    pub fn is_blocking(&self) -> bool {
        self.block_type == CommandBlockType::Blocking
    }

    /// 获取剩余等待时间 (用于外部计时器)
    pub fn remaining_time(&self) -> Option<f64> {
        if let ExecutorState::Waiting(WaitCondition::Time(_)) = &self.state {
            self.time_remaining
        } else {
            None
        }
    }

    fn execute(&mut self, context: &Rc<RefCell<ScriptContext>>) -> ExecuteStatus {
        let result = self.execute_command(context);

        match &result {
            ExecuteStatus::Waiting(WaitCondition::Time(total)) => {
                self.time_remaining = Some(*total);
                self.state = ExecutorState::Waiting(WaitCondition::Time(*total));
            }
            ExecuteStatus::Waiting(condition) => {
                self.state = ExecutorState::Waiting(condition.clone());
            }
            ExecuteStatus::Completed => {
                self.state = ExecutorState::Completed;
            }
            ExecuteStatus::Error(e) => {
                self.state = ExecutorState::Error(e.clone());
            }
        }

        result
    }

    fn execute_command(&mut self, context: &Rc<RefCell<ScriptContext>>) -> ExecuteStatus {
        match &self.command {
            Command::Assignment { name, value } => {
                let mut ctx = context.borrow_mut();
                ctx.set_global_val(name, value.clone());

                ExecuteStatus::Completed
            }
            Command::CommandAssignment {
                name,
                command,
                args,
            } => {
                let args = {
                    let ctx = context.borrow();
                    ctx.parse_args(args)
                };
                // 0. 如果command是类名
                if { context.borrow().type_registry.contains(command) } {
                    match {
                        context.borrow().type_registry.build_type_instance(
                            command,
                            args.to_vec(),
                            &context,
                        )
                    } {
                        Ok(val) => {
                            context.borrow_mut().set_global_val(name, val);
                            ExecuteStatus::Completed
                        }
                        Err(s) => ExecuteStatus::Error(format!(
                            "assign {} failed: when buiding instane: {}",
                            name, s
                        )),
                    }
                // 1. 执行命令
                } else {
                    let result = self.execute_command_call(context, command, &args);

                    match result {
                        Ok(return_value) => {
                            // 2. 将返回值赋给变量
                            let mut ctx = context.borrow_mut();
                            ctx.set_global_val(name, return_value);
                            ExecuteStatus::Completed
                        }
                        Err(e) => ExecuteStatus::Error(e.to_string()),
                    }
                }
            }
            Command::Set { path, args } => {
                let args = {
                    let ctx = context.borrow();
                    ctx.parse_args(args)
                };
                self.execute_set(context, path, &args, false)
            }

            Command::Once { path, args } => {
                let args = {
                    let ctx = context.borrow();
                    ctx.parse_args(args)
                };

                self.execute_set(context, path, &args, true)
            }

            Command::Wait { condition } => ExecuteStatus::Waiting(condition.clone()),

            Command::Call { path, args } => {
                let args = {
                    let ctx = context.borrow();
                    ctx.parse_args(args)
                };

                match {context.borrow().resolve_path(path)} {
                    Ok(ScriptValue::Function(func)) => match func.call(&context, args.clone()) {
                        Ok(_) => ExecuteStatus::Completed,
                        Err(e) => ExecuteStatus::Error(e.to_string()),
                    },
                    Ok(val) => ExecuteStatus::Error(format!("{:?} is not a function", val)),
                    Err(e) => ExecuteStatus::Error(e),
                }
            }

            Command::Chain { commands } => {
                for cmd in commands {
                    let mut sub_executor = CommandExecutor::new(cmd.clone());
                    match sub_executor.step(context) {
                        ExecuteStatus::Completed => continue,
                        ExecuteStatus::Waiting(condition) => {
                            return ExecuteStatus::Waiting(condition);
                        }
                        ExecuteStatus::Error(e) => return ExecuteStatus::Error(e),
                    }
                }
                ExecuteStatus::Completed
            }

            Command::Empty => ExecuteStatus::Completed,
        }
    }

    /// 执行命令调用并获取返回值
    fn execute_command_call(
        &self,
        context: &Rc<RefCell<ScriptContext>>,
        command: &str,
        args: &[ScriptValue],
    ) -> anyhow::Result<ScriptValue> {
        // 解析命令路径
        match context.borrow().resolve_path(command) {
            Ok(ScriptValue::Function(func)) => {
                // 调用函数，获取返回值
                func.call(&context, args.to_vec())
            }
            Ok(val) => {
                // 如果不是函数，返回对象本身 (如 UserData)
                Ok(val)
            }
            Err(_) => {
                // 路径不存在，可能是全局方法
                // 尝试从 globals 直接查找
                if let Some(func) = context.borrow().get_global_val(command) {
                    if let ScriptValue::Function(f) = func {
                        return f.call(&context, args.to_vec());
                    }
                }
                anyhow::bail!("Command '{}' not found", command)
            }
        }
    }

    fn execute_set(
        &self,
        context: &Rc<RefCell<ScriptContext>>,
        path: &str,
        args: &[ScriptValue],
        is_once: bool,
    ) -> ExecuteStatus {
        let mut ctx = context.borrow_mut();
        let parts: Vec<&str> = path.split('.').collect();

        if parts.len() == 1 {
            let old_value = ctx.get_global_val(path).unwrap_or(ScriptValue::nil());

            if args.is_empty() {
                return ExecuteStatus::Error("set requires at least one argument".to_string());
            }

            if is_once {
                ctx.push_once_record(crate::script::OnceRecord {
                    path: path.to_string(),
                    field: None,
                    old_value: old_value.clone(),
                });
            }

            ctx.set_global_val(path, args[0].clone());
            ExecuteStatus::Completed
        } else {
            let obj_name = parts[0];
            let field_path = parts[1..].join(".");

            let obj = match ctx.get_global_val(obj_name) {
                Some(val) => val,
                None => return ExecuteStatus::Error(format!("Global '{}' not found", obj_name)),
            };

            drop(ctx);

            match obj {
                ScriptValue::Table(table) => {
                    let old_value = table
                        .borrow()
                        .get(&field_path)
                        .unwrap_or(ScriptValue::nil());

                    if args.is_empty() {
                        return ExecuteStatus::Error(
                            "set requires at least one argument".to_string(),
                        );
                    }

                    table.borrow_mut().set(&field_path, args[0].clone());

                    if is_once {
                        let mut ctx = context.borrow_mut();
                        ctx.push_once_record(crate::script::OnceRecord {
                            path: path.to_string(),
                            field: Some(field_path),
                            old_value,
                        });
                    }

                    ExecuteStatus::Completed
                }
                _ => ExecuteStatus::Error(format!("Cannot set on {:?}", obj)),
            }
        }
    }
}

/// 输入事件
#[derive(Debug, Clone, PartialEq)]
pub enum InputEvent {
    Click,
    Text(String),
    Key(char),
}
