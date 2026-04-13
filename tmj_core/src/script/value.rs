// src/script/value.rs
use serde::{Deserialize, Deserializer, Serialize, Serializer, de};
use std::{
    any::Any,
    cell::{Ref, RefCell, RefMut},
    collections::HashMap,
    fmt,
    rc::Rc,
};

use crate::script::{
    RustObjectTrait, RustObjectWrapper, ScriptFunction, Table, TableRef, function::FnSignature,
    value_convert::IntoScriptValue,
};

/// 脚本系统中的核心值类型
#[derive(Clone)]
pub enum ScriptValue {
    Nil,
    Bool(bool),
    Int(i64),
    Float(f64),
    String(String),
    Expression(String),  // 表达式,变量名一类的东西的索引, 并不是真的值也不会保存在任何环境中, 仅过度用
    Table(TableRef), // Rc + RefCell 足够 

    Function(Rc<ScriptFunction>), // 不需要 Send + Sync
    RustObject(Rc<RustObjectWrapper>), // 注意, rustobj不像 classtable那样能够序列化,
                                  // 平时几乎用不到
}
/// 内部可序列化的值 (用于 serde)
#[derive(Debug, Clone, Serialize, Deserialize)]
enum ScriptValueData {
    Nil,
    Bool(bool),
    Int(i64),
    Float(f64),
    String(String),
    Table(Table), // 直接序列化 Table 数据

    #[serde(skip)]
    Function,
    #[serde(skip)]
    RustObject,
}

impl Serialize for ScriptValue {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let data = match self {
            ScriptValue::Nil => ScriptValueData::Nil,
            ScriptValue::Bool(b) => ScriptValueData::Bool(*b),
            ScriptValue::Int(i) => ScriptValueData::Int(*i),
            ScriptValue::Float(f) => ScriptValueData::Float(*f),
            ScriptValue::String(s) => ScriptValueData::String(s.clone()),
            ScriptValue::Table(t) => ScriptValueData::Table(t.borrow().clone()),
            ScriptValue::Expression(_) => ScriptValueData::Nil,
            ScriptValue::Function(_) => ScriptValueData::Nil, // (serde::ser::Error::custom("Function cannot be serialized")),
            ScriptValue::RustObject(_) => ScriptValueData::Nil, // Err(serde::ser::Error::custom("RustObject cannot be serialized")),
        };
        data.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for ScriptValue {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let data = ScriptValueData::deserialize(deserializer)?;
        Ok(match data {
            ScriptValueData::Nil => ScriptValue::Nil,
            ScriptValueData::Bool(b) => ScriptValue::Bool(b),
            ScriptValueData::Int(i) => ScriptValue::Int(i),
            ScriptValueData::Float(f) => ScriptValue::Float(f),
            ScriptValueData::String(s) => ScriptValue::String(s),
            ScriptValueData::Table(t) => ScriptValue::Table(Rc::new(RefCell::new(t))),
            ScriptValueData::Function => {
                return Err(de::Error::custom("Function cannot be deserialized"));
            }
            ScriptValueData::RustObject => {
                return Err(de::Error::custom("RustObject cannot be deserialized"));
            }
        })
    }
}

impl ScriptValue {
    // ========== 构造函数 ==========
    pub fn nil() -> Self {
        ScriptValue::Nil
    }
    pub fn bool(v: bool) -> Self {
        ScriptValue::Bool(v)
    }
    pub fn int(v: i64) -> Self {
        ScriptValue::Int(v)
    }
    pub fn float(v: f64) -> Self {
        ScriptValue::Float(v)
    }
    pub fn string(v: impl Into<String>) -> Self {
        ScriptValue::String(v.into())
    }

    pub fn table() -> Self {
        ScriptValue::Table(Rc::new(RefCell::new(Table::new())))
    }

    pub fn table_from_hashmap<T: IntoScriptValue>(map: HashMap<String, T>) -> Self {
        ScriptValue::Table(Rc::new(RefCell::new(Table::from_hashmap(map))))
    }

    pub fn function<F>(name: impl Into<String>, func: F) -> Self
    where
        F: FnSignature,
    {
        ScriptValue::Function(Rc::new(ScriptFunction::new(name, func)))
    }

    pub fn rust_object<T: RustObjectTrait + 'static>(obj: T) -> Self {
        ScriptValue::RustObject(Rc::new(RustObjectWrapper::new(obj)))
    }

    // ========== 类型判断 ==========
    pub fn is_expression(&self) -> bool {
        matches!(self, ScriptValue::Expression(_))
    }
    pub fn is_nil(&self) -> bool {
        matches!(self, ScriptValue::Nil)
    }
    pub fn is_bool(&self) -> bool {
        matches!(self, ScriptValue::Bool(_))
    }
    pub fn is_int(&self) -> bool {
        matches!(self, ScriptValue::Int(_))
    }
    pub fn is_float(&self) -> bool {
        matches!(self, ScriptValue::Float(_))
    }
    pub fn is_string(&self) -> bool {
        matches!(self, ScriptValue::String(_))
    }
    pub fn is_table(&self) -> bool {
        matches!(self, ScriptValue::Table(_))
    }
    pub fn is_function(&self) -> bool {
        matches!(self, ScriptValue::Function(_))
    }
    pub fn is_rust_object(&self) -> bool {
        matches!(self, ScriptValue::RustObject(_))
    }

    // ========== 类型转换 ==========
    pub fn as_expression(&self) -> Option<String> {
        if let ScriptValue::Expression(e) = self {
            Some(e.clone())
        } else {
            tracing::error!("{:?} is not expression", self);
            None
        }
    }
    pub fn as_bool(&self) -> Option<bool> {
        if let ScriptValue::Bool(v) = self {
            Some(*v)
        } else {
            tracing::error!("{:?} is not bool", self);
            None
        }
    }

    pub fn as_int(&self) -> Option<i64> {
        if let ScriptValue::Int(v) = self {
            Some(*v)
        } else {
            tracing::error!("{:?} is not int", self);
            None
        }
    }

    pub fn as_float(&self) -> Option<f64> {
        if let ScriptValue::Float(v) = self {
            Some(*v)
        } else {
            tracing::error!("{:?} is not float", self);
            None
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        if let ScriptValue::String(v) = self {
            Some(v.as_str())
        } else {
            tracing::error!("{:?} is not str", self);
            None
        }
    }
    pub fn as_string(&self) -> Option<&String> {
        if let ScriptValue::String(v) = self {
            Some(v)
        } else {
            tracing::error!("{:?} is not string", self);
            None
        }
    }

    pub fn as_table(&self) -> Option<Rc<RefCell<Table>>> {
        if let ScriptValue::Table(t) = self {
            Some(Rc::clone(t))
        } else {
            tracing::error!("{:?} is not table", self);
            None
        }
    }

    pub fn as_function(&self) -> Option<Rc<ScriptFunction>> {
        if let ScriptValue::Function(f) = self {
            Some(Rc::clone(f))
        } else {
            tracing::error!("{:?} is not function", self);
            None
        }
    }

    pub fn as_rust_object(&self) -> Option<Rc<RustObjectWrapper>> {
        if let ScriptValue::RustObject(obj) = self {
            Some(Rc::clone(obj))
        } else {
            tracing::error!("{:?} is not rustobj", self);
            None
        }
    }
    /// 可变向下转型 Rust 对象 (关键！)
    pub fn downcast_mut<T: Any>(&self) -> Option<RefMut<'_, T>> {
        if let ScriptValue::RustObject(obj) = self {
            obj.downcast_mut::<T>()
        } else {
            None
        }
    }

    /// 不可变向下转型 Rust 对象
    pub fn downcast_ref<T: Any>(&self) -> Option<Ref<'_, T>> {
        if let ScriptValue::RustObject(obj) = self {
            obj.downcast_ref::<T>()
        } else {
            None
        }
    }

    // ========== 数值转换 ==========
    pub fn to_number(&self) -> Option<f64> {
        match self {
            ScriptValue::Int(v) => Some(*v as f64),
            ScriptValue::Float(v) => Some(*v),
            ScriptValue::String(s) => s.parse::<f64>().ok(),
            _ => None,
        }
    }

    pub fn to_bool(&self) -> bool {
        match self {
            ScriptValue::Nil => false,
            ScriptValue::Bool(false) => false,
            _ => true,
        }
    }
}

impl fmt::Debug for ScriptValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ScriptValue::Nil => write!(f, "nil"),
            ScriptValue::Bool(v) => write!(f, "{}", v),
            ScriptValue::Int(v) => write!(f, "{}", v),
            ScriptValue::Float(v) => write!(f, "{}", v),
            ScriptValue::String(v) => write!(f, "{:?}", v),
            ScriptValue::Expression(v) => write!(f, "{v}"),
            ScriptValue::Table(_) => write!(f, "<table>"),
            ScriptValue::Function(func) => write!(f, "<fn:{}>", func.name()),
            ScriptValue::RustObject(_) => write!(f, "<rust_obj>"),
        }
    }
}

impl PartialEq for ScriptValue {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (ScriptValue::Nil, ScriptValue::Nil) => true,
            (ScriptValue::Bool(a), ScriptValue::Bool(b)) => a == b,
            (ScriptValue::Int(a), ScriptValue::Int(b)) => a == b,
            (ScriptValue::Float(a), ScriptValue::Float(b)) => {
                a.total_cmp(b) == std::cmp::Ordering::Equal
            }
            (ScriptValue::String(a), ScriptValue::String(b)) => a == b,
            (ScriptValue::Table(a), ScriptValue::Table(b)) => Rc::ptr_eq(a, b),
            (ScriptValue::Function(a), ScriptValue::Function(b)) => Rc::ptr_eq(a, b),
            (ScriptValue::RustObject(a), ScriptValue::RustObject(b)) => Rc::ptr_eq(a, b),
            _ => false,
        }
    }
}
