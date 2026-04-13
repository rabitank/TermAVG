use std::collections::HashMap;
use crate::script::{ContextRef, IntoTable, RegistableType, ScriptValue, Table, TypeName};
use std::{rc::Rc, cell::RefCell};

/// 可转换为 ScriptValue 的类型
pub trait IntoScriptValue {
    fn into_script_val(self) -> ScriptValue;

    fn into_script_class_table(self, _ctx: &ContextRef) -> ScriptValue
    where 
    Self: Sized{
        self.into_script_val()
    }
}

// ========== 基础类型实现 ==========
impl IntoScriptValue for ScriptValue {
    fn into_script_val(self) -> ScriptValue {
        self
    }
}

impl IntoScriptValue for String {
    fn into_script_val(self) -> ScriptValue {
        ScriptValue::String(self)
    }
}

impl IntoScriptValue for &str {
    fn into_script_val(self) -> ScriptValue {
        ScriptValue::String(self.to_string())
    }
}

impl IntoScriptValue for i64 {
    fn into_script_val(self) -> ScriptValue {
        ScriptValue::Int(self)
    }
}

impl IntoScriptValue for i32 {
    fn into_script_val(self) -> ScriptValue {
        ScriptValue::Int(self as i64)
    }
}

impl IntoScriptValue for f64 {
    fn into_script_val(self) -> ScriptValue {
        ScriptValue::Float(self)
    }
}

impl IntoScriptValue for f32 {
    fn into_script_val(self) -> ScriptValue {
        ScriptValue::Float(self as f64)
    }
}

impl IntoScriptValue for bool {
    fn into_script_val(self) -> ScriptValue {
        ScriptValue::Bool(self)
    }
}

impl IntoScriptValue for () {
    fn into_script_val(self) -> ScriptValue {
        ScriptValue::Nil
    }
}

// ========== Option 实现 ==========
impl<T: IntoScriptValue> IntoScriptValue for Option<T> {
    fn into_script_val(self) -> ScriptValue {
        match self {
            Some(v) => v.into_script_val(),
            None => ScriptValue::Nil,
        }
    }
}

// ========== Table 实现 ==========
impl IntoScriptValue for Table {
    fn into_script_val(self) -> ScriptValue {
        ScriptValue::Table(Rc::new(RefCell::new(self)))
    }
}

impl IntoScriptValue for Rc<RefCell<Table>> {
    fn into_script_val(self) -> ScriptValue {
        ScriptValue::Table(self)
    }
}

// ========== HashMap 递归实现 ==========
impl<T: IntoScriptValue> IntoScriptValue for HashMap<String, T> {
    fn into_script_val(self) -> ScriptValue {
        let mut table = Table::new();
        for (key, value) in self {
            table.set(key, value.into_script_val());
        }
        ScriptValue::Table(Rc::new(RefCell::new(table)))
    }
}

// ========== Vec 实现 (数组) ==========
impl<T: IntoScriptValue> IntoScriptValue for Vec<T> {
    fn into_script_val(self) -> ScriptValue {
        let mut table = Table::new();
        for (i, value) in self.into_iter().enumerate() {
            table.set_int(i as i64, value.into_script_val());
        }
        ScriptValue::Table(Rc::new(RefCell::new(table)))
    }
}

impl<T: RegistableType + IntoTable + TypeName> IntoScriptValue for T {
    fn into_script_class_table(self, ctx: &ContextRef) -> ScriptValue {
        let type_name = self.type_name();
        let table = Rc::new(RefCell::new(self.into_data_table()));
        table.borrow_mut().set_type_tag(type_name);

        match T::attach_table_methods(ctx, &table) {
            Ok(_) => ScriptValue::Table(table),
            Err(info) => {
                tracing::error!("into_script_class_table faild: {}", info);
                ScriptValue::Table(table)
            }
        }
    }

    fn into_script_val(self) -> ScriptValue {
        ScriptValue::Nil
    }
}

