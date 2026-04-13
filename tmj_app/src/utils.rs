
pub struct ConstInfo {
    pub module: &'static str,  // 模块路径::常量名
    pub value: &'static str, // 常量值（小写字符串）
}

inventory::collect!(ConstInfo);
