use tmj_core::script::{Interpreter, ScriptValue, TypeName, lower_str};

use crate::pages::script_def::BaseVariable;

// name
lower_str!(CHAPTER);

//member
lower_str!(ALPHA);
lower_str!(ALPHA_SPEED);
lower_str!(TITLE);
lower_str!(SUBTITLE);

//method
lower_str!(SHOW);
lower_str!(HIDE);

#[derive(TypeName)]
pub struct VChapter;

impl BaseVariable for VChapter {
    fn regist_to_ctx_impl(ctx: &mut tmj_core::script::ScriptContext) -> anyhow::Result<()> {
        ctx.set_global_table(CHAPTER);
        let _ = ctx.set_table_member(CHAPTER, ALPHA, ScriptValue::float(0.0));
        let _ = ctx.set_table_member(CHAPTER, ALPHA_SPEED, ScriptValue::float(0_f64));
        let _ = ctx.set_table_member(CHAPTER, TITLE, ScriptValue::string(""));
        let _ = ctx.set_table_member(CHAPTER, SUBTITLE, ScriptValue::string(""));

        {
            let _ = ctx
                .set_table_func(CHAPTER, SHOW, |ctx, _args| {
                    Interpreter::eval(constcat::concat!(
                        "set ", CHAPTER, ".", ALPHA, " ", 1.0, 
                        "\nonce ", CHAPTER, ".", ALPHA_SPEED, " ", 1.0,
                    ).into(), ctx.clone())?;

                    let chapter = ctx
                        .borrow()
                        .get_global_val(CHAPTER)
                        .ok_or(anyhow::anyhow!("chapter not found"))?
                        .as_table()
                        .ok_or(anyhow::anyhow!("chapter is not table"))?;
                    Ok(ScriptValue::Table(chapter))
                })
                .map_err(|e| anyhow::anyhow!(e))?;
        }

        {
            let _ = ctx
                .set_table_func(CHAPTER, "hide", |ctx, _args| {
                    Interpreter::eval(constcat::concat!(
                        "set ", CHAPTER, ".", ALPHA, " ", 0.0, 
                    ).into(), ctx.clone())?;
                    let chapter = ctx
                        .borrow()
                        .get_global_val(CHAPTER)
                        .ok_or(anyhow::anyhow!("chapter not found"))?
                        .as_table()
                        .ok_or(anyhow::anyhow!("chapter is not table"))?;
                    Ok(ScriptValue::Table(chapter))
                })
                .map_err(|e| anyhow::anyhow!(e))?;
        }

        Ok(())
    }
}
