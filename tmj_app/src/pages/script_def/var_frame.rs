use tmj_core::script::{ScriptValue, TypeName, lower_str};

use crate::pages::script_def::BaseVariable;

lower_str!(FRAME);

// members
lower_str!(SHOW);

#[derive(TypeName)]
pub struct VFrame;

impl BaseVariable for VFrame {
    fn regist_to_ctx_impl(ctx: &mut tmj_core::script::ScriptContext) -> anyhow::Result<()> {
        ctx.reg_table(FRAME);
        let _ = ctx.set_table_member(FRAME, SHOW, ScriptValue::bool(true));
        Ok(())
    }
}


