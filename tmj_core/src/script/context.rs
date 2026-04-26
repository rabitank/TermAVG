use crate::script::{RustObjectTrait, ScriptFunction, ScriptValue, Table, TableRef, function::FnSignature};

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
    /// 下一个分配的表 id（从 1 起；0 表示非法）
    next_tuid: u64,
    /// 运行时 `tuid` → 表；不参与序列化
    tuid_table: HashMap<u64, TableRef>,
    /// 供 `Table::set` 点路径等分配子表；在 `Interpreter` / 场景创建后绑定
    context_ref: Option<ContextRef>,
}

impl ScriptContext {
    pub fn new() -> Self {
        let type_registry = TypeRegistry::new();
        ScriptContext {
            globals: HashMap::new(),
            type_registry,
            once_stack: Vec::new(),
            session_id: 0,
            next_tuid: 1,
            tuid_table: HashMap::new(),
            context_ref: None,
        }
    }

    /// 在将 `ScriptContext` 包进 `Rc<RefCell<_>>` 之后调用一次，供表内点路径分配子表
    pub fn bind_context_ref(&mut self, ctx: ContextRef) {
        self.context_ref = Some(ctx);
    }

    pub fn context_ref(&self) -> Option<ContextRef> {
        self.context_ref.clone()
    }

    /// 分配新 `tuid`（尚未与 `Rc` 绑定）
    pub fn alloc_table_id(&mut self) -> u64 {
        let id = self.next_tuid;
        self.next_tuid += 1;
        id
    }

    /// 注册已带 `tuid` 的表（同一 `tuid` 必须 `Rc` 指针一致）
    pub fn register_table_rc(&mut self, rc: &TableRef) -> Result<(), String> {
        let tuid = rc.borrow().tuid;
        if tuid == 0 {
            return Err("table tuid 0 is invalid".to_string());
        }
        if let Some(existing) = self.tuid_table.get(&tuid) {
            if !Rc::ptr_eq(existing, rc) {
                return Err(format!(
                    "duplicate tuid {tuid}: two different table Rc instances"
                ));
            }
            return Ok(());
        }
        self.tuid_table.insert(tuid, rc.clone());
        Ok(())
    }

    /// 分配空表并注册到 `tuid_table`
    pub fn alloc_table_rc(&mut self) -> TableRef {
        let tuid = self.alloc_table_id();
        let t = Table::with_tuid(tuid);
        let rc = Rc::new(RefCell::new(t));
        self.register_table_rc(&rc)
            .expect("alloc_table_rc: fresh tuid must register");
        rc
    }

    /// 将 `ScriptValue` 子树中所有 `Table` 按 `tuid` 登记（用于类型实例构建后）
    pub fn register_script_value_tables(&mut self, v: &ScriptValue) -> Result<(), String> {
        match v {
            ScriptValue::Table(rc) => self.register_table_rc_recursive(rc),
            ScriptValue::Nil
            | ScriptValue::Bool(_)
            | ScriptValue::Int(_)
            | ScriptValue::Float(_)
            | ScriptValue::String(_)
            | ScriptValue::Expression(_)
            | ScriptValue::TableHandle(_)
            | ScriptValue::Function(_)
            | ScriptValue::RustObject(_) => Ok(()),
        }
    }

    fn register_table_rc_recursive(&mut self, rc: &TableRef) -> Result<(), String> {
        self.register_table_rc(rc)?;
        let children: Vec<ScriptValue> = {
            let b = rc.borrow();
            b.iter()
                .map(|(_, v)| v.clone())
                .chain(b.int_iter().map(|(_, v)| v.clone()))
                .collect()
        };
        for child in children {
            self.register_script_value_tables(&child)?;
        }
        Ok(())
    }

    pub fn resolve_table_value(&self, v: &ScriptValue) -> Result<TableRef, String> {
        match v {
            ScriptValue::Table(rc) => Ok(rc.clone()),
            ScriptValue::TableHandle(tuid) => self
                .tuid_table
                .get(tuid)
                .cloned()
                .ok_or_else(|| format!("TableHandle: unknown tuid {tuid}")),
            _ => Err("value is not a table or table handle".to_string()),
        }
    }

    /// 读档入口：清空索引（`to_context` 前）
    pub fn clear_tuid_table_for_load(&mut self) {
        self.tuid_table.clear();
    }

    /// 从当前 `globals` 引用图重建 `tuid_table` 并校验 `TableHandle`；更新 `next_tuid`
    pub fn rebuild_tuid_table_from_live(&mut self) -> Result<(), String> {
        self.tuid_table.clear();
        let mut max_seen: u64 = 0;
        let roots: Vec<ScriptValue> = self.globals.values().cloned().collect();
        for v in &roots {
            self.visit_register_tables(v, &mut max_seen)?;
        }
        for v in roots.iter() {
            self.visit_validate_handles(v)?;
        }
        self.next_tuid = self.next_tuid.max(max_seen.saturating_add(1));
        Ok(())
    }

    fn visit_register_tables(
        &mut self,
        v: &ScriptValue,
        max_seen: &mut u64,
    ) -> Result<(), String> {
        match v {
            ScriptValue::Table(rc) => {
                let tuid = rc.borrow().tuid;
                *max_seen = (*max_seen).max(tuid);
                if let Some(existing) = self.tuid_table.get(&tuid) {
                    if !Rc::ptr_eq(existing, rc) {
                        return Err(format!(
                            "live graph: duplicate tuid {tuid} on different Rc"
                        ));
                    }
                } else {
                    self.tuid_table.insert(tuid, rc.clone());
                }
                let children: Vec<ScriptValue> = {
                    let b = rc.borrow();
                    b.iter()
                        .map(|(_, v)| v.clone())
                        .chain(b.int_iter().map(|(_, v)| v.clone()))
                        .collect()
                };
                for child in children {
                    self.visit_register_tables(&child, max_seen)?;
                }
                Ok(())
            }
            ScriptValue::TableHandle(_) => Ok(()),
            ScriptValue::Nil
            | ScriptValue::Bool(_)
            | ScriptValue::Int(_)
            | ScriptValue::Float(_)
            | ScriptValue::String(_)
            | ScriptValue::Expression(_)
            | ScriptValue::Function(_)
            | ScriptValue::RustObject(_) => Ok(()),
        }
    }

    fn visit_validate_handles(&self, v: &ScriptValue) -> Result<(), String> {
        match v {
            ScriptValue::TableHandle(tuid) => {
                if !self.tuid_table.contains_key(tuid) {
                    return Err(format!(
                        "TableHandle: dangling tuid {tuid} (not in tuid_table)"
                    ));
                }
                Ok(())
            }
            ScriptValue::Table(rc) => {
                let children: Vec<ScriptValue> = {
                    let b = rc.borrow();
                    b.iter()
                        .map(|(_, v)| v.clone())
                        .chain(b.int_iter().map(|(_, v)| v.clone()))
                        .collect()
                };
                for child in children {
                    self.visit_validate_handles(&child)?;
                }
                Ok(())
            }
            _ => Ok(()),
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
        let rc = self.alloc_table_rc();
        self.globals.insert(name, ScriptValue::Table(rc));
    }

    pub fn set_table_member(
        &mut self,
        obj_path: &str,
        member_name: impl Into<String>,
        value: ScriptValue,
    ) -> Result<(), String> {
        let obj = self.resolve_path(obj_path)?;
        let table = self.resolve_table_value(&obj)?;
        let member_name = member_name.into();
        info!(
            "Context: register_member {}.{}",
            obj_path,
            member_name.clone()
        );
        let ctx_opt = self.context_ref();
        table
            .borrow_mut()
            .set(member_name, value, ctx_opt.as_ref());
        Ok(())
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
        let table = self.resolve_table_value(&obj)?;
        let method_name = method_name.into();
        info!(
            "Context: register_object_method {}.{}",
            obj_path, method_name
        );
        let func = ScriptFunction::new(method_name.clone(), func);
        let ctx_opt = self.context_ref();
        table.borrow_mut().set(
            method_name,
            ScriptValue::Function(Rc::new(func)),
            ctx_opt.as_ref(),
        );
        Ok(())
    }

    pub fn resolve_path(&self, path: &str) -> Result<ScriptValue, String> {
        let parts: Vec<&str> = path.split('.').collect();
        if parts.is_empty() {
            return Err("Empty path".to_string());
        }

        let mut current = self
            .get_global_val(parts[0])
            .ok_or_else(|| format!("Global '{}' not found", parts[0]))?;

        let ctx_opt = self.context_ref();
        for &part in &parts[1..] {
            current = match current {
                ScriptValue::Table(_) | ScriptValue::TableHandle(_) => {
                    let table = self.resolve_table_value(&current)?;
                    table
                        .borrow()
                        .get(part, ctx_opt.as_ref())
                        .ok_or_else(|| format!("Field '{}' not found", part))?
                }
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
                    .ok_or_else(|| format!("Global '{}' not found", obj_name))?
                    .clone();
                self.resolve_table_value(&obj)
                    .map_err(|_| format!("Cannot set field on {:?}", obj))?
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
        let ctx_opt = self.context_ref();

        if parts.len() == 1 {
            table
                .borrow_mut()
                .set(parts[0], value, ctx_opt.as_ref());
        } else {
            let sub_sv = table
                .borrow()
                .get(parts[0], ctx_opt.as_ref())
                .ok_or_else(|| format!("Field '{}' not found", parts[0]))?;
            let sub_t = self.resolve_table_value(&sub_sv)?;
            self.set_table_relative(&sub_t, &parts[1..].join("."), value)?;
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
                let table = self
                    .globals
                    .get(&obj_name)
                    .and_then(|obj| self.resolve_table_value(obj).ok());
                if table.is_none() {
                    info!("Context: cannot restore on non-table: {}", obj_name);
                }

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
        self.tuid_table.clear();
        self.next_tuid = 1;
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
            if let ScriptValue::Table(table_rc) = &v {
                let table_rc = table_rc.clone();
                // 直接掏出来方法注册函数给table用上. 这里ai处理不赖借用问题所以直接内联了
                let type_name = table_rc.borrow().type_tag().map(|s| s.to_string());
                if let Some(type_name) = type_name {
                    let method_f = {
                        let b = ctx.borrow();
                        b.type_registry
                            .get_type_builders(&type_name)
                            .map(|(_, method_f)| method_f)
                    };
                    let build_res = if let Some(method_f) = method_f {
                        match method_f(ctx, &table_rc) {
                            Ok(_) => {
                                ctx.borrow_mut()
                                    .register_script_value_tables(&ScriptValue::Table(table_rc.clone()))?;
                                Ok(ScriptValue::Table(table_rc.clone()))
                            }
                            Err(err) => Err(err),
                        }
                    } else {
                        Err(format!("Unknown type: {}", type_name))
                    };
                    match build_res {
                        Ok(ins) => {
                            // typed table 通常不是内置变量；直接覆盖写入即可
                            ctx.borrow_mut().set_global_val(name, ins);
                        }
                        Err(s) => {
                            tracing::error!("{}", s);
                            return Err(s);
                        }
                    }
                } else {
                    // untyped table：优先与已存在内置 table 合并
                    let existing_val = { ctx.borrow().get_global_val(name) };
                    if let Some(existing_val) = existing_val {
                        if let Ok(dst) = ctx.borrow().resolve_table_value(&existing_val) {
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
        ctx_ref.borrow_mut().clear_tuid_table_for_load();
        serializable.to_context(ctx_ref)?;
        ctx_ref.borrow_mut().rebuild_tuid_table_from_live()
    }
}

#[cfg(test)]
mod tuid_tests {
    use super::*;

    #[test]
    fn rebuild_tuid_table_resolves_table_handle() {
        let ctx = Rc::new(RefCell::new(ScriptContext::new()));
        ctx.borrow_mut().bind_context_ref(ctx.clone());
        {
            let mut m = ctx.borrow_mut();
            let c = m.alloc_table_rc();
            let tuid = c.borrow().tuid;
            let ls = m.alloc_table_rc();
            ls.borrow_mut().set_int(0, ScriptValue::table_handle(tuid));
            m.set_global_val("character_alice", ScriptValue::Table(c.clone()));
            m.set_global_val("character_ls", ScriptValue::Table(ls));
            m.rebuild_tuid_table_from_live().unwrap();
            let resolved = m
                .resolve_table_value(&ScriptValue::table_handle(tuid))
                .unwrap();
            assert!(Rc::ptr_eq(&resolved, &c));
        }
    }

    #[test]
    fn serde_serializable_context_preserves_handle() {
        let ctx = Rc::new(RefCell::new(ScriptContext::new()));
        ctx.borrow_mut().bind_context_ref(ctx.clone());
        {
            let mut m = ctx.borrow_mut();
            let c = m.alloc_table_rc();
            c.borrow_mut().set("name", ScriptValue::string("alice"), None);
            let tuid = c.borrow().tuid;
            let ls = m.alloc_table_rc();
            ls.borrow_mut().set_int(0, ScriptValue::table_handle(tuid));
            m.set_global_val("character_alice", ScriptValue::Table(c));
            m.set_global_val("character_ls", ScriptValue::Table(ls));
            m.rebuild_tuid_table_from_live().unwrap();
        }

        let ser = ScriptContext::serialize(&ctx);
        let json = json5::to_string(&ser).unwrap();

        let ctx2 = Rc::new(RefCell::new(ScriptContext::new()));
        ctx2.borrow_mut().bind_context_ref(ctx2.clone());
        {
            let mut m = ctx2.borrow_mut();
            let a = m.alloc_table_rc();
            let l = m.alloc_table_rc();
            m.set_global_val("character_alice", ScriptValue::Table(a));
            m.set_global_val("character_ls", ScriptValue::Table(l));
            m.rebuild_tuid_table_from_live().unwrap();
        }

        let loaded: SerializableContext = json5::from_str(&json).unwrap();
        ScriptContext::deserialize(&ctx2, loaded).unwrap();

        let ls = ctx2
            .borrow()
            .get_global_val("character_ls")
            .unwrap()
            .as_table()
            .unwrap();
        let slot0 = ls.borrow().get_int(0).unwrap();
        let alice = ctx2.borrow().resolve_table_value(&slot0).unwrap();
        assert_eq!(
            alice.borrow().get("name", None).unwrap().as_str(),
            Some("alice")
        );
    }
}
