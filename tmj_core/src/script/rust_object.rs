// src/script/rust_object.rs
use crate::script::ScriptValue;
use std::{
    any::Any,
    cell::{Ref, RefCell, RefMut},
    rc::Rc,
};
/// Rust 对象包装器 - 支持可变借用
pub struct RustObjectWrapper {
    data: Rc<RefCell<dyn RustObjectTrait>>,
}

impl RustObjectWrapper {
    pub fn new<T: RustObjectTrait + 'static>(obj: T) -> Self {
        RustObjectWrapper {
            data: Rc::new(RefCell::new(obj)),
        }
    }

    /// 不可变借用
    pub fn borrow(&self) -> Ref<'_, dyn RustObjectTrait> {
        self.data.borrow()
    }

    /// 可变借用
    pub fn borrow_mut(&self) -> RefMut<'_, dyn RustObjectTrait> {
        self.data.borrow_mut()
    }

    /// 尝试不可变向下转型
    pub fn downcast_ref<T: Any>(&self) -> Option<Ref<'_, T>> {
        Ref::filter_map(self.borrow(), |obj| obj.as_any().downcast_ref::<T>()).ok()
    }

    /// 尝试可变向下转型 (关键！)
    pub fn downcast_mut<T: Any>(&self) -> Option<RefMut<'_, T>> {
        RefMut::filter_map(self.borrow_mut(), |obj| {
            obj.as_any_mut().downcast_mut::<T>()
        })
        .ok()
    }

    /// 尝试获取强引用 (用于克隆)
    pub fn rc_clone(&self) -> Rc<RefCell<dyn RustObjectTrait>> {
        Rc::clone(&self.data)
    }
}

impl Clone for RustObjectWrapper {
    fn clone(&self) -> Self {
        RustObjectWrapper {
            data: Rc::clone(&self.data),
        }
    }
}

/// Rust 对象暴露给脚本的 trait
pub trait RustObjectTrait: Any + Send{
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;

    fn get_method(&self, _name: &str) -> Option<ScriptValue> {
        None
    }

    fn type_name(&self) -> &str {
        std::any::type_name::<Self>()
    }
}

/// 宏：快速实现 RustObjectTrait
#[macro_export]
macro_rules! impl_rust_object {
    ($type:ty) => {
        impl $crate::script::RustObjectTrait for $type {
            fn as_any(&self) -> &dyn std::any::Any {
                self
            }
            fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
                self
            }
        }
    };
}
