use tracing::error;

use crate::script::{ContextRef, ScriptContext, ScriptValue, Table};
use std::{cell::RefCell, collections::HashMap, rc::Rc};

/// 类型构建函数
pub type BuildTableFn = fn(&ScriptContext, Vec<ScriptValue>) -> Table;
pub type AddMethodsFn = fn(&ContextRef, &Rc<RefCell<Table>>) -> Result<(), String>;

/// 类型注册表
pub struct TypeRegistry {
    types: HashMap<String, (BuildTableFn, AddMethodsFn)>,
}

pub trait IntoTable {
    fn into_data_table(self) -> Table;
}

pub trait FromCommand {
    fn from_script_command(ctx: &ScriptContext, args: Vec<ScriptValue>) -> Result<Self, String>
        where
            Self: Sized;
}

pub trait TypeName {
    const TYPE_NAME: &'static str;
    fn type_name(&self) -> &'static str {
        Self::TYPE_NAME
    }
    fn static_type_name() -> &'static str {
        Self::TYPE_NAME
    }
}

pub trait RegistableType: TypeName {

    fn create_class_table(ctx: &ScriptContext, args: Vec<ScriptValue>) -> Table;

    fn attach_table_methods(
        _ctx: &ContextRef,
        _table_rc: &Rc<RefCell<Table>>,
    ) -> Result<(), String> {
        Ok(())
    }
}

impl<T: IntoTable + FromCommand + TypeName> RegistableType for T {

    fn create_class_table(ctx: &ScriptContext, args: Vec<ScriptValue>) -> Table {
        match T::from_script_command(ctx, args) {
            Ok(rust_ins) => {
                let table = rust_ins.into_data_table();
        table
            }
            Err(info) => {
                error!("create type {} data table failed, reson: {}", T::TYPE_NAME, info);
                Table::with_type_tag(T::TYPE_NAME)
            }
        }
    }
}

impl TypeRegistry {
    pub fn new() -> Self {
        TypeRegistry {
            types: HashMap::new(),
        }
    }

    pub fn register<T: RegistableType>(&mut self) {
        self.types.insert(
            T::static_type_name().to_string(),
            (T::create_class_table, T::attach_table_methods),
        );
    }

    pub fn build_type_instance(
        &self,
        type_name: &str,
        args: Vec<ScriptValue>,
        ctx: &ContextRef,
    ) -> Result<ScriptValue, String> {
        let (data_f, method_f) = self
            .types
            .get(type_name)
            .ok_or_else(|| format!("Unknown type: {}", type_name))?;

        let mut table: Table = data_f(&ctx.borrow(), args);
        table.set_type_tag(type_name);
        let table_rc = Rc::new(RefCell::new(table));

        match method_f(ctx, &table_rc) {
            Ok(_) => Ok(ScriptValue::Table(table_rc)),
            Err(err) => Err(err),
        }
    }

    /// 添加类型实例方法 (场景为从序列化的 data table 添加附加方法)
    pub fn rebuild_type_methods(
        &self,
        type_name: &str,
        table_rc: Rc<RefCell<Table>>,
        ctx: &ContextRef,
    ) -> Result<ScriptValue, String> {
        let (_, method_f) = self
            .types
            .get(type_name)
            .ok_or_else(|| format!("Unknown type: {}", type_name))?;

        // 2. 添加方法
        match method_f(ctx, &table_rc) {
            Ok(_) => Ok(ScriptValue::Table(table_rc)),
            Err(err) => Err(err),
        }
    }

    pub fn contains(&self, type_name: &str) -> bool {
        self.types.contains_key(type_name)
    }
}

impl Default for TypeRegistry {
    fn default() -> Self {
        Self::new()
    }
}
