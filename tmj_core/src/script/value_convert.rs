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

impl<T: RegistableType + IntoTable + TypeName> IntoScriptValue for T {
    fn into_script_class_table(self, ctx: &ContextRef) -> ScriptValue {
        let type_name = self.type_name();
        let table = {
            let mut sctx = ctx.borrow_mut();
            Rc::new(RefCell::new(self.into_data_table(&mut sctx)))
        };
        table.borrow_mut().set_type_tag(type_name);

        match T::attach_table_methods(ctx, &table) {
            Ok(_) => {
                let _ = ctx
                    .borrow_mut()
                    .register_script_value_tables(&ScriptValue::Table(table.clone()));
                ScriptValue::Table(table)
            }
            Err(info) => {
                tracing::error!("into_script_class_table faild: {}", info);
                let _ = ctx
                    .borrow_mut()
                    .register_script_value_tables(&ScriptValue::Table(table.clone()));
                ScriptValue::Table(table)
            }
        }
    }

    fn into_script_val(self) -> ScriptValue {
        ScriptValue::Nil
    }
}

