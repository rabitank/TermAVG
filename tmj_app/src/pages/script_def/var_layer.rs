use tmj_core::script::{TypeName, lower_str};

use crate::pages::script_def::BaseVariable;

lower_str!(LAYERS);
lower_str!(LAYER_TYPE);
lower_str!(SOURCE);
lower_str!(VISIBLE);

#[derive(TypeName)]
pub struct VLayer;

impl BaseVariable for VLayer {
    fn regist_to_ctx_impl(ctx: &mut tmj_core::script::ScriptContext) -> anyhow::Result<()> {
        ctx.set_global_table(LAYERS);
        Ok(())
    }
}
