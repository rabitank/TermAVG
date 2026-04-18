// src/script/table.rs
use serde::{Deserialize, Serialize};
use std::{cell::RefCell, collections::HashMap, rc::Rc};

use crate::script::{ContextRef, ScriptValue, TypeName, value_convert::IntoScriptValue};
pub type TableRef = Rc<std::cell::RefCell<Table>>;

/// t 对象 - 类似 Lua 的 table
/// 支持字符串键和整数键，可存储任意 ScriptValue
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Table {
    /// 字符串键 -> 值
    string_keys: HashMap<String, ScriptValue>,
    /// 整数键 -> 值 (用于数组式访问)
    int_keys: HashMap<i64, ScriptValue>,
    /// 元表 (用于实现继承/原型链)
    #[serde(skip)]
    metatable: Option<Rc<RefCell<Table>>>,
    /// 类型标签 (可选，用于 Rust 层识别)
    type_tag: Option<String>,
}

pub trait TabelGet {
    fn get(&self, member: impl ToString) -> anyhow::Result<ScriptValue>;
}

impl TabelGet for Rc<RefCell<Table>> {
    fn get(&self, member: impl ToString) -> anyhow::Result<ScriptValue> {
        self.borrow()
            .get(&member.to_string())
            .ok_or(anyhow::anyhow!("member {:} get failed", member.to_string()))
    }
}

impl Table {
    /// 有的function里面持有table引用, 因此这里封装避免直接调用
    pub fn call_method(
        t: &Rc<RefCell<Table>>,
        func_name: &str,
        ctx: &ContextRef,
        args: Vec<ScriptValue>,
    ) -> Result<ScriptValue, anyhow::Error> {
        {
            t.borrow()
                .get(func_name)
                .clone()
                .unwrap()
                .as_function()
                .unwrap()
        }
        .call(&ctx, args)
    }

    // ---------- 辅助函数：单键操作 ----------
    fn get_single(&self, key: &str) -> Option<ScriptValue> {
        if let Ok(num) = key.parse::<i64>() {
            if let Some(val) = self.int_keys.get(&num).cloned() {
                return Some(val);
            }
        }
        if let Some(val) = self.string_keys.get(key).cloned() {
            return Some(val);
        }
        if let Some(val) = self.get_from_metatable(key) {
            return Some(val);
        }
        tracing::error!("table doesn't contain key '{}' ", key);
        None
    }

    fn set_single(&mut self, key: &str, value: ScriptValue) {
        if let Ok(num) = key.parse::<i64>() {
            self.int_keys.insert(num, value);
        } else {
            self.string_keys.insert(key.to_string(), value);
        }
    }

    // ---------- 公开接口 ----------
    pub fn get(&self, key: &str) -> Option<ScriptValue> {
        if !key.contains('.') {
            return self.get_single(key);
        }

        let parts: Vec<&str> = key.split('.').collect();
        if parts.is_empty() {
            return None;
        }

        let mut current = self.get_single(parts[0])?;
        for &part in &parts[1..] {
            current = match current {
                ScriptValue::Table(tbl) => tbl.borrow().get_single(part)?,
                ScriptValue::RustObject(obj) => obj.borrow().get_method(part)?,
                _ => {
                    tracing::error!("cannot access '{}' on non-table in path '{}'", part, key);
                    return None;
                }
            };
        }
        Some(current)
    }

    pub fn set(&mut self, key: impl Into<String>, value: ScriptValue) {
        let key = key.into();
        if !key.contains('.') {
            self.set_single(&key, value);
            return;
        }

        let parts: Vec<&str> = key.split('.').collect();
        if parts.is_empty() {
            return;
        }

        let mut current = self.get_single(parts[0]).unwrap().as_table().unwrap();
        // 遍历中间路径段，确保存在表
        for &part in &parts[1..parts.len() - 1] {
            let new_val = || ScriptValue::Table(Rc::new(RefCell::new(Table::new())));
            let mut target_rc: Option<Rc<RefCell<Table>>> = None;
            {
                // 限定借用范围，确保尽快结束
                let mut binding = current.borrow_mut();

                if let Ok(num) = part.parse::<i64>() {
                    // 插入或获取引用
                    let _ = binding.int_keys.entry(num).or_insert_with(new_val.clone());
                    // 直接通过 key 获取（因为刚才确保了存在）
                    if let Some(v) = binding.int_keys.get_mut(&num) {
                        if let ScriptValue::Table(r) = v {
                            target_rc = Some(r.clone());
                        }
                    }
                } else {
                    let _ = binding
                        .string_keys
                        .entry(part.to_string())
                        .or_insert_with(new_val.clone());
                    if let Some(v) = binding.string_keys.get_mut(&part.to_string()) {
                        if let ScriptValue::Table(r) = v {
                            target_rc = Some(r.clone());
                        }
                    }
                }

                // 这里 binding 会自动 drop
            }

            // 3. 借用结束后，安全更新 current
            if let Some(rc) = target_rc {
                current = rc;
            } else {
                // 处理错误
                return;
            }
        }

        // 设置最后一个键
        let last = parts.last().unwrap();
        current.borrow_mut().set_single(last, value);
    }

    pub fn new() -> Self {
        Table {
            string_keys: HashMap::new(),
            int_keys: HashMap::new(),
            metatable: None,
            type_tag: None,
        }
    }
    /// 从 HashMap 创建 Table
    pub fn from_hashmap<T: IntoScriptValue>(map: HashMap<String, T>) -> Self {
        let mut table = Table::new();
        for (key, value) in map {
            table.set(key, value.into_script_val());
        }
        table
    }

    /// 从迭代器创建 Table
    pub fn from_iter<K, V>(iter: impl IntoIterator<Item = (K, V)>) -> Self
    where
        K: Into<String>,
        V: IntoScriptValue,
    {
        let mut table = Table::new();
        for (key, value) in iter {
            table.set(key.into(), value.into_script_val());
        }
        table
    }
    pub fn with_type_tag(type_tag: impl Into<String>) -> Self {
        let mut t = Table::new();
        t.type_tag = Some(type_tag.into());
        t
    }

    pub fn remove(&mut self, key: &str) -> Option<ScriptValue> {
        self.string_keys.remove(key)
    }

    pub fn contains(&self, key: &str) -> bool {
        self.string_keys.contains_key(key)
    }

    // ========== 整数键操作 ==========
    pub fn get_int(&self, index: i64) -> Option<ScriptValue> {
        self.int_keys.get(&index).cloned()
    }

    pub fn set_int(&mut self, index: i64, value: ScriptValue) {
        self.int_keys.insert(index, value);
    }

    // ========== 数组式操作 ==========
    pub fn push(&mut self, value: ScriptValue) {
        let next_index = self.int_keys.len() as i64;
        self.set_int(next_index, value);
    }

    pub fn len(&self) -> usize {
        self.int_keys.len()
    }

    pub fn is_empty(&self) -> bool {
        self.int_keys.is_empty() && self.string_keys.is_empty()
    }

    // ========== 元表操作 ==========
    pub fn set_metatable(&mut self, mt: Rc<RefCell<Table>>) {
        self.metatable = Some(mt);
    }

    pub fn get_metatable(&self) -> Option<Rc<RefCell<Table>>> {
        self.metatable.clone()
    }

    fn get_from_metatable(&self, key: &str) -> Option<ScriptValue> {
        self.metatable.as_ref().and_then(|mt| mt.borrow().get(key))
    }

    // ========== 类型标签 ==========
    pub fn type_tag(&self) -> Option<&str> {
        self.type_tag.as_deref()
    }

    pub fn set_type_tag(&mut self, tag: impl Into<String>) {
        self.type_tag = Some(tag.into());
    }

    pub fn is_ins<T: TypeName>(&self) -> bool {
        if self.type_tag.is_some() {
            self.type_tag.as_ref().unwrap() == T::TYPE_NAME
        } else {
            false
        }
    }

    // ========== 迭代器 ==========
    pub fn keys(&self) -> impl Iterator<Item = &String> {
        self.string_keys.keys()
    }

    pub fn iter(&self) -> impl Iterator<Item = (&String, &ScriptValue)> {
        self.string_keys.iter()
    }
    pub fn int_keys(&self) -> impl Iterator<Item = &i64> {
        self.int_keys.keys()
    }

    pub fn int_iter(&self) -> impl Iterator<Item = (&i64, &ScriptValue)> {
        self.int_keys.iter()
    }

    /// 将 `other` 的键值合并到 `self`。
    ///
    /// 读档场景下 `ScriptValue::Function/RustObject` 会在序列化时退化为 `Nil`，
    /// 为避免把运行时注入的内置方法覆盖掉，这里采用：
    /// - `other` 为非 Nil：总是覆盖写入
    /// - `other` 为 Nil：仅当 `self` 对应键是 Function/RustObject 时跳过
    pub fn merge_from(&mut self, other: &Table) {
        // string keys
        for (k, v_other) in other.string_keys.iter() {
            let should_write = if v_other.is_nil() {
                match self.string_keys.get(k) {
                    None => true,
                    Some(v_self) => !(v_self.is_function() || v_self.is_rust_object()),
                }
            } else {
                true
            };
            if should_write {
                self.string_keys.insert(k.clone(), v_other.clone());
            }
        }

        // int keys
        for (k, v_other) in other.int_keys.iter() {
            let should_write = if v_other.is_nil() {
                match self.int_keys.get(k) {
                    None => true,
                    Some(v_self) => !(v_self.is_function() || v_self.is_rust_object()),
                }
            } else {
                true
            };
            if should_write {
                self.int_keys.insert(*k, v_other.clone());
            }
        }

        // type tag: keep existing unless missing
        if self.type_tag.is_none() {
            self.type_tag = other.type_tag.clone();
        }
    }
}

impl Default for Table {
    fn default() -> Self {
        Self::new()
    }
}
