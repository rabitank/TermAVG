// src/script/command.rs

use crate::script::ScriptValue;

/// 命令阻塞类型
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CommandBlockType {
    /// 阻塞型：wait 会阻塞整个 session
    Blocking,
    /// 非阻塞型：wait 只阻塞当前命令链
    NonBlocking,
}

/// 脚本命令类型
#[derive(Debug, Clone)]
pub enum Command {
    /// 赋值命令：var = value
    Assignment {
        name: String,
        value: ScriptValue,
    },
    /// var = call args
    CommandAssignment {
        name: String,
        command: String,
        args: Vec<ScriptValue>,
    },
    
    /// 调用命令：set obj params...
    Set {
        path: String,
        args: Vec<ScriptValue>,
    },
    
    /// 一次性命令：once obj params...
    Once {
        path: String,
        args: Vec<ScriptValue>,
    },
    
    /// 等待命令：wait click / wait 0.5
    Wait {
        condition: WaitCondition,
    },
    
    /// 方法调用：obj.method args...
    Call {
        path: String,
        args: Vec<ScriptValue>,
    },
    
    /// 链式调用：cmd1 -> cmd2 -> cmd3
    /// 链式命令整体是非阻塞的
    Chain {
        commands: Vec<Command>,
    },
    
    /// 空命令
    Empty,
}

impl Command {
    /// 获取命令的阻塞类型
    pub fn block_type(&self) -> CommandBlockType {
        match self {
            // 链式命令是非阻塞的
            Command::Chain { .. } => CommandBlockType::NonBlocking,
            // 其他命令默认是阻塞的
            _ => CommandBlockType::Blocking,
        }
    }
}
// src/script/command.rs

/// 等待条件
#[derive(Debug, Clone, PartialEq)]
pub enum WaitCondition {
    /// 等待点击
    Click,
    /// 等待指定秒数
    Time(f64),
    /// 等待特定输入
    Input(String),
    /// 组合等待 (如点击或时间)
    Any(Vec<WaitCondition>),
}

impl WaitCondition {
    /// 是否是时间等待
    pub fn is_time(&self) -> bool {
        matches!(self, WaitCondition::Time(_))
    }

    /// 是否是事件等待
    pub fn is_event(&self) -> bool {
        matches!(self, WaitCondition::Click | WaitCondition::Input(_))
    }

    /// 获取时间值 (如果是时间等待)
    pub fn as_time(&self) -> Option<f64> {
        match self {
            WaitCondition::Time(t) => Some(*t),
            _ => None,
        }
    }
}
