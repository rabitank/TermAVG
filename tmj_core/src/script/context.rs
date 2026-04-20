use crate::script::{RustObjectTrait, ScriptFunction, ScriptValue, Table, function::FnSignature};

use anyhow::Context;
use std::{cell::RefCell, collections::HashMap, rc::Rc};
use tracing::info;

#[derive(Clone)]
pub struct OnceRecord {
    /// 目标对象路径 (如 "dialogue_frame.show")
    pub path: String,
    /// 如果是字段访问，记录字段名
    pub field: Option<String>,
    /// 原始值
    pub old_value: ScriptValue,
}

pub type ContextRef = Rc<RefCell<ScriptContext>>;

pub struct ScriptContext {
    pub globals: HashMap<String, ScriptValue>,
    pub type_registry: TypeRegistry,
    once_stack: Vec<OnceRecord>,
    session_id: usize,
}

impl ScriptContext {
    pub fn new() -> Self {
        let type_registry = TypeRegistry::new();
        ScriptContext {
            globals: HashMap::new(),
            type_registry,
            once_stack: Vec::new(),
            session_id: 0,
        }
    }

    ///处理表达式类型的Script Value
    pub fn parse_args(&self, args: &Vec<ScriptValue>) -> Vec<ScriptValue> {
        let mut new_args: Vec<ScriptValue> = Vec::with_capacity(args.len());
        for i in args {
            if i.is_expression() {
                let i = self
                    .resolve_path(i.as_expression().unwrap().as_str())
                    .map_err(|e| anyhow::anyhow!(e))
                    .context("parse expression arg filed {i} -> parse as string")
                    .unwrap_or_else(|e| {
                        tracing::warn!("{:?}", e);
                        ScriptValue::String(i.as_expression().unwrap())
                    });
                new_args.push(i);
            } else {
                new_args.push(i.clone());
            }
        }
        new_args
    }

    pub fn get_val(&self, name: &str) -> Option<ScriptValue> {
        let res = self
            .resolve_path(name)
            .map_err(|e| anyhow::anyhow!(e))
            .context("parse expression arg filed {i} -> parse as string")
            .unwrap_or_else(|e| {
                tracing::error!("get {} failed!: {:?}", name, e);
                ScriptValue::Nil
            });
        Some(res)
    }

    pub fn get_global_val(&self, name: &str) -> Option<ScriptValue> {
        let res = self.globals.get(name).cloned();
        if res.is_none() {
            tracing::error!("global val: {name} ~ got failed");
            return None;
        }
        res
    }

    pub fn set_global_val(&mut self, name: impl Into<String>, value: ScriptValue) {
        let name = name.into();
        info!("Context: set_global {} = {:?}", name, value);
        self.globals.insert(name, value);
    }

    pub fn remove(&mut self, name: &str) -> Option<ScriptValue> {
        self.globals.remove(name)
    }

    pub fn global_contain(&self, name: &str) -> bool {
        self.globals.contains_key(name)
    }

    pub fn set_global_func<F>(&mut self, name: impl Into<String>, func: F)
    where
        F: FnSignature,
    {
        let name = name.into();
        info!("Context: register_global_method {}", name);
        let func = ScriptFunction::new(name.clone(), func);
        self.globals
            .insert(name, ScriptValue::Function(Rc::new(func)));
    }

    pub fn set_global_robj(
        &mut self,
        name: impl Into<String>,
        obj: impl RustObjectTrait + 'static,
    ) {
        let name = name.into();
        info!("Context: register_global_object {}", name);
        self.globals.insert(name, ScriptValue::rust_object(obj));
    }

    pub fn set_global_table(&mut self, name: impl Into<String>) {
        let name = name.into();
        info!("Context: register_global_table {}", name);
        self.globals.insert(name, ScriptValue::table());
    }

    pub fn set_table_member(
        &mut self,
        obj_path: &str,
        member_name: impl Into<String>,
        value: ScriptValue,
    ) -> Result<(), String> {
        let obj = self.resolve_path(obj_path)?;
        if let Some(table) = obj.as_table() {
            let member_name = member_name.into();
            info!(
                "Context: register_member {}.{}",
                obj_path,
                member_name.clone()
            );
            table.borrow_mut().set(member_name, value);
            Ok(())
        } else {
            Err(format!("'{}' is not a table", obj_path))
        }
    }

    pub fn set_table_func<F>(
        &mut self,
        obj_path: &str,
        method_name: impl Into<String>,
        func: F,
    ) -> Result<(), String>
    where
        F: FnSignature,
    {
        let obj = self.resolve_path(obj_path)?;
        if let Some(table) = obj.as_table() {
            let method_name = method_name.into();
            info!(
                "Context: register_object_method {}.{}",
                obj_path, method_name
            );
            let func = ScriptFunction::new(method_name.clone(), func);
            table
                .borrow_mut()
                .set(method_name, ScriptValue::Function(Rc::new(func)));
            Ok(())
        } else {
            Err(format!("'{}' is not a table", obj_path))
        }
    }

    pub fn resolve_path(&self, path: &str) -> Result<ScriptValue, String> {
        let parts: Vec<&str> = path.split('.').collect();
        if parts.is_empty() {
            return Err("Empty path".to_string());
        }

        let mut current = self
            .get_global_val(parts[0])
            .ok_or_else(|| format!("Global '{}' not found", parts[0]))?;

        for &part in &parts[1..] {
            current = match current {
                ScriptValue::Table(table) => table
                    .borrow()
                    .get(part)
                    .ok_or_else(|| format!("Field '{}' not found", part))?,
                ScriptValue::RustObject(ref obj) => obj
                    .borrow()
                    .get_method(part)
                    .ok_or_else(|| format!("Method '{}' not found", part))?,
                _ => return Err(format!("Cannot access field '{}' on non-table value", part)),
            };
        }

        Ok(current)
    }

    /// 设置对象字段 (支持嵌套路径)
    pub fn set_table(&mut self, path: &str, value: ScriptValue) -> Result<ScriptValue, String> {
        let parts: Vec<&str> = path.split('.').collect();
        if parts.is_empty() {
            return Err("Empty path".to_string());
        }

        // 先获取旧值 (不可变借用)
        let old_value = self.resolve_path(path).unwrap_or(ScriptValue::nil());

        if parts.len() == 1 {
            // 全局变量
            info!("Context: set_field {} = {:?}", path, value);
            self.globals.insert(parts[0].to_string(), value);
        } else {
            // 对象字段 - 分离借用以避免冲突
            let obj_name = parts[0].to_string();
            let field_path = parts[1..].join(".");

            // 先克隆出 table 引用，释放 globals 的借用
            let table = {
                let obj = self
                    .globals
                    .get(&obj_name)
                    .ok_or_else(|| format!("Global '{}' not found", obj_name))?;

                match obj {
                    ScriptValue::Table(table) => Rc::clone(table),
                    _ => return Err(format!("Cannot set field on {:?}", obj)),
                }
            };

            // 先记录日志 (在移动 value 之前)
            info!("Context: set_field {} = {:?}", path, value);

            // 现在移动 value
            self.set_table_relative(&table, &field_path, value)?;
        }

        Ok(old_value)
    }

    /// 递归设置 Table 字段
    fn set_table_relative(
        &self,
        table: &Rc<RefCell<Table>>,
        field_path: &str,
        value: ScriptValue,
    ) -> Result<(), String> {
        let parts: Vec<&str> = field_path.split('.').collect();

        if parts.len() == 1 {
            table.borrow_mut().set(parts[0], value);
        } else {
            let sub_table = table
                .borrow()
                .get(parts[0])
                .ok_or_else(|| format!("Field '{}' not found", parts[0]))?;

            if let ScriptValue::Table(sub_t) = sub_table {
                self.set_table_relative(&sub_t, &parts[1..].join("."), value)?;
            } else {
                return Err(format!("Field '{}' is not a table", parts[0]));
            }
        }

        Ok(())
    }

    pub fn push_once_record(&mut self, record: OnceRecord) {
        info!("Context: push_once_record path={}", record.path);
        self.once_stack.push(record);
    }

    pub fn restore_once(&mut self) {
        info!("Context: restore_once, {} records", self.once_stack.len());

        // 关键修复：先收集到临时变量，释放 once_stack 的借用
        let records: Vec<OnceRecord> = self.once_stack.drain(..).rev().collect();

        for record in records {
            info!(
                "Context: restoring {} = {:?}",
                record.path, record.old_value
            );

            let parts: Vec<&str> = record.path.split('.').collect();

            if parts.len() == 1 {
                // 全局变量 - 直接恢复
                self.globals.insert(record.path, record.old_value);
            } else {
                // 对象字段 - 需要分离借用
                let obj_name = parts[0].to_string();
                let field_path = parts[1..].join(".");

                // 先获取 table 引用 (在独立作用域中，释放 globals 借用)
                let table = {
                    if let Some(obj) = self.globals.get(&obj_name).cloned() {
                        match obj {
                            ScriptValue::Table(table) => Some(table),
                            _ => {
                                info!("Context: cannot restore on non-table: {}", obj_name);
                                None
                            }
                        }
                    } else {
                        None
                    }
                };

                // 现在可以安全调用 set_table_field
                if let Some(t) = table {
                    let _ = self.set_table_relative(&t, &field_path, record.old_value);
                }
            }
        }
    }

    pub fn start_session(&mut self) {
        self.session_id += 1;
        self.once_stack.clear();
        info!("Context: start_session id={}", self.session_id);
    }

    pub fn end_session(&mut self) {
        info!("Context: end_session id={}", self.session_id);
        self.restore_once();
    }

    pub fn session_id(&self) -> usize {
        self.session_id
    }

    pub fn clear(&mut self) {
        info!("Context: clear");
        self.globals.clear();
        self.once_stack.clear();
        self.session_id = 0;
    }
}

impl Default for ScriptContext {
    fn default() -> Self {
        Self::new()
    }
}

use crate::script::TypeRegistry;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, serde::Serialize, Deserialize)]
pub struct SerializableContext {
    globals: HashMap<String, ScriptValue>,
    session_id: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SerializableOnceRecord {
    pub path: String,
    pub field: Option<String>,
    pub old_value: ScriptValue,
}

impl SerializableContext {
    pub fn from_context(ctx: &ScriptContext) -> Self {
        let mut globals = HashMap::new();

        for (name, value) in ctx.globals.iter() {
            // 跳过函数和 RustObject (它们会被 serde skip)
            if !value.is_function() && !value.is_rust_object() {
                globals.insert(name.clone(), value.clone());
            }
        }
        SerializableContext {
            globals,
            session_id: ctx.session_id,
        }
    }

    pub fn to_context(&self, ctx: &ContextRef) -> Result<(), String> {
        ctx.borrow_mut().session_id = self.session_id;

        for (name, value) in &self.globals {
            let v = value.clone();
            if value.is_table() {
                let table_rc = value.as_table().unwrap();
                let type_name = table_rc.borrow().type_tag().map(|s| s.to_string());
                if let Some(type_name) = type_name {
                    let build_res = ctx.borrow_mut().type_registry.rebuild_type_methods(
                        &type_name,
                        table_rc.clone(),
                        ctx,
                    );
                    match build_res {
                        Ok(ins) => {
                            // typed table 通常不是内置变量；直接覆盖写入即可
                            ctx.borrow_mut().set_global_val(name, ins);
                        }
                        Err(s) => {
                            tracing::error!(s);
                        }
                    }
                } else {
                    // untyped table：优先与已存在内置 table 合并
                    if let Some(existing) = ctx.borrow().get_global_val(name) {
                        if let Some(dst) = existing.as_table() {
                            let src_b = table_rc.borrow();
                            dst.borrow_mut().merge_from(&src_b);
                            continue;
                        }
                    }
                    ctx.borrow_mut()
                        .set_global_val(name, ScriptValue::Table(table_rc));
                }
            } else {
                // 非 table 值：当存档值是 Nil 时，避免把运行时注册的全局函数/对象覆盖掉
                if v.is_nil() {
                    if let Some(existing) = ctx.borrow().get_global_val(name) {
                        if existing.is_function() || existing.is_rust_object() {
                            continue;
                        }
                    }
                }
                ctx.borrow_mut().set_global_val(name, v);
            }
        }
        Ok(())
    }
}

// ScriptContext 原有方法保持不变
impl ScriptContext {
    pub fn serialize(ctx_ref: &ContextRef) -> SerializableContext {
        SerializableContext::from_context(&ctx_ref.borrow_mut())
    }

    pub fn deserialize(
        ctx_ref: &ContextRef,
        serializable: SerializableContext,
    ) -> Result<(), String> {
        serializable.to_context(ctx_ref)
    }
}
