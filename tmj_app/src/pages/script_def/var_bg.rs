use std::time::Duration;

use tmj_core::{
    audio::AudioOp,
    script::{Interpreter, ScriptValue, TypeName, lower_str},
};

use crate::{
    audio::{self, AUDIOM, load_audio},
    pages::script_def::BaseVariable,
};

lower_str!(BG);

// method
lower_str!(SET);

// member
lower_str!(IMAGE);
lower_str!(IS_EDGE);

/// Bg: Background Object
#[derive(TypeName)]
pub struct VBg;

impl BaseVariable for VBg {
    fn regist_to_ctx_impl(ctx: &mut tmj_core::script::ScriptContext) -> anyhow::Result<()> {
        ctx.set_global_table(BG);

        let _ = ctx.set_table_member(BG, IMAGE, ScriptValue::String("".into()));
        let _ = ctx.set_table_member(BG, IS_EDGE, ScriptValue::Bool(true));

        let _ = ctx.set_table_func(BG, SET, |_ctx, args| {
            let new_img_path = args.first().unwrap().clone();
            if !new_img_path.is_string() {
                anyhow::bail!("bg.set args should be string first");
            }
            let _ = _ctx.borrow_mut().set_table_member(BG, IMAGE, new_img_path);
            // todo! 当然不是只设置图像那么简单,后面可以改出动画效果

            Ok(ScriptValue::Nil)});

        Ok(())
    }
}
