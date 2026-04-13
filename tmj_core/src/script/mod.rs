// src/script/mod.rs

mod value;
mod table;
mod function;
mod rust_object;
mod context;
mod command;
mod command_executor;
mod task_queue;
mod session;
mod interpreter;
mod type_registry;

mod lexer;
mod parser;
mod script_parsers;

mod value_convert;

// 核心类型
pub use value::ScriptValue;
pub use table::Table;
pub use table::TableRef;
pub use function::ScriptFunction;

// Rust 对象互转
pub use rust_object::{RustObjectTrait, RustObjectWrapper};

// 上下文
pub use context::{ScriptContext, SerializableContext,  OnceRecord, ContextRef};

// 命令系统
pub use command::{Command, WaitCondition, CommandBlockType};

// 执行器
pub use command_executor::{CommandExecutor, ExecuteStatus, InputEvent};

// 任务队列
pub use task_queue::{TaskQueue, QueueStatus};

// Session
pub use session::{SessionExecutor, SessionStatus};

// 解释器
pub use interpreter::{Interpreter, InterpreterStatus};

// 宏
pub use crate::impl_rust_object;


pub use script_parsers::ScriptParser;

pub use tmj_macro::TypeName;
pub use tmj_macro::lower_str;
pub use type_registry::*;
pub use value_convert::*;
