use tracing::error;

use crate::script::{ContextRef, ScriptContext, ScriptValue, Table};
use std::{cell::RefCell, collections::HashMap, rc::Rc};

/// 类型构建函数
pub type BuildTableFn = fn(&mut ScriptContext, Vec<ScriptValue>) -> Table;
pub type AddMethodsFn = fn(&ContextRef, &Rc<RefCell<Table>>) -> Result<(), String>;

/// 类型注册表
pub struct TypeRegistry {
    types: HashMap<String, (BuildTableFn, AddMethodsFn)>,
}

pub trait IntoTable {
    fn into_data_table(self, ctx: &mut ScriptContext) -> Table;
}

pub trait FromCommand {
    fn from_script_command(ctx: &mut ScriptContext, args: Vec<ScriptValue>) -> Result<Self, String>
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

    fn create_class_table(ctx: &mut ScriptContext, args: Vec<ScriptValue>) -> Table;

    fn attach_table_methods(
        _ctx: &ContextRef,
        _table_rc: &Rc<RefCell<Table>>,
    ) -> Result<(), String> {
        Ok(())
    }
}

impl<T: IntoTable + FromCommand + TypeName> RegistableType for T {
    fn create_class_table(ctx: &mut ScriptContext, args: Vec<ScriptValue>) -> Table {
        match T::from_script_command(ctx, args) {
            Ok(rust_ins) => rust_ins.into_data_table(ctx),
            Err(info) => {
                error!("create type {} data table failed, reson: {}", T::TYPE_NAME, info);
                let id = ctx.alloc_table_id();
                Table::with_type_tag_and_tuid(T::TYPE_NAME, id)
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

    pub fn contains(&self, type_name: &str) -> bool {
        self.types.contains_key(type_name)
    }

    pub fn get_type_builders(&self, type_name: &str) -> Option<(BuildTableFn, AddMethodsFn)> {
        self.types.get(type_name).copied()
    }
}

impl Default for TypeRegistry {
    fn default() -> Self {
        Self::new()
    }
}
