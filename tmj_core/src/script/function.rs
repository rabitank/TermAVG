// src/script/function.rs
use std::{fmt, rc::Rc};
use crate::script::{ContextRef, ScriptValue};

/// 脚本函数包装 - 单线程版本
pub struct ScriptFunction {
    name: String,
    // 不需要 Send + Sync 约束
    func: Rc<dyn Fn(&ContextRef, Vec<ScriptValue>) -> anyhow::Result<ScriptValue>>,
}

pub trait FnSignature: Fn(&ContextRef, Vec<ScriptValue>) -> anyhow::Result<ScriptValue > + 'static {}
impl<T> FnSignature for T where T: Fn(&ContextRef, Vec<ScriptValue>) -> anyhow::Result<ScriptValue> + 'static {}

impl ScriptFunction {
    pub fn new<F>(name: impl Into<String>, func: F) -> Self
    where
        F: FnSignature
    {
        ScriptFunction {
            name: name.into(),
            func: Rc::new(func),
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn call(&self, context: &ContextRef, args: Vec<ScriptValue>) -> anyhow::Result<ScriptValue> {
        (self.func)(context, args)
    }
}

impl fmt::Debug for ScriptFunction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Function({})", self.name)
    }
}

impl Clone for ScriptFunction {
    fn clone(&self) -> Self {
        ScriptFunction {
            name: self.name.clone(),
            func: Rc::clone(&self.func),
        }
    }
}
