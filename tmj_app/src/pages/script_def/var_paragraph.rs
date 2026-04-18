use tmj_core::script::{ScriptValue, TypeName, lower_str};

use crate::pages::script_def::BaseVariable;

lower_str!(PARAGRAPH);
lower_str!(VISIBLE);
lower_str!(CONTENT);
lower_str!(TYPEWRITER_ENABLE);
lower_str!(TYPEWRITER_SPEED);
lower_str!(TYPEWRITER_PROGRESS);
lower_str!(TYPEWRITER_LAST_CONTENT);

#[derive(TypeName)]
pub struct VParagraph;

impl BaseVariable for VParagraph {
    fn regist_to_ctx_impl(ctx: &mut tmj_core::script::ScriptContext) -> anyhow::Result<()> {
        ctx.set_global_table(PARAGRAPH);
        let _ = ctx.set_table_member(PARAGRAPH, VISIBLE, ScriptValue::bool(false));
        let _ = ctx.set_table_member(PARAGRAPH, CONTENT, ScriptValue::string(""));
        let _ = ctx.set_table_member(PARAGRAPH, TYPEWRITER_ENABLE, ScriptValue::bool(true));
        let _ = ctx.set_table_member(PARAGRAPH, TYPEWRITER_SPEED, ScriptValue::float(60.0));
        let _ = ctx.set_table_member(PARAGRAPH, TYPEWRITER_PROGRESS, ScriptValue::float(0.0));
        let _ = ctx.set_table_member(PARAGRAPH, TYPEWRITER_LAST_CONTENT, ScriptValue::string(""));

        {
            let _ = ctx
                .set_table_func(PARAGRAPH, "show", |ctx, _args| {
                    let paragraph = ctx
                        .borrow()
                        .get_global_val(PARAGRAPH)
                        .ok_or(anyhow::anyhow!("paragraph not found"))?
                        .as_table()
                        .ok_or(anyhow::anyhow!("paragraph is not table"))?;
                    paragraph.borrow_mut().set(VISIBLE, ScriptValue::bool(true));
                    Ok(ScriptValue::Table(paragraph))
                })
                .map_err(|e| anyhow::anyhow!(e))?;
        }

        {
            let _ = ctx
                .set_table_func(PARAGRAPH, "hide", |ctx, _args| {
                    let paragraph = ctx
                        .borrow()
                        .get_global_val(PARAGRAPH)
                        .ok_or(anyhow::anyhow!("paragraph not found"))?
                        .as_table()
                        .ok_or(anyhow::anyhow!("paragraph is not table"))?;
                    paragraph.borrow_mut().set(VISIBLE, ScriptValue::bool(false));
                    Ok(ScriptValue::Table(paragraph))
                })
                .map_err(|e| anyhow::anyhow!(e))?;
        }

        {
            let _ = ctx
                .set_table_func(PARAGRAPH, "print", |ctx, args| {
                    let text = args
                        .first()
                        .and_then(|x| x.as_str())
                        .ok_or(anyhow::anyhow!("paragraph.print requires text argument"))?;
                    let paragraph = ctx
                        .borrow()
                        .get_global_val(PARAGRAPH)
                        .ok_or(anyhow::anyhow!("paragraph not found"))?
                        .as_table()
                        .ok_or(anyhow::anyhow!("paragraph is not table"))?;
                    paragraph.borrow_mut().set(CONTENT, ScriptValue::string(text));
                    paragraph.borrow_mut().set(VISIBLE, ScriptValue::bool(true));
                    Ok(ScriptValue::Table(paragraph))
                })
                .map_err(|e| anyhow::anyhow!(e))?;
        }

        {
            let _ = ctx
                .set_table_func(PARAGRAPH, "clear", |ctx, _args| {
                    let paragraph = ctx
                        .borrow()
                        .get_global_val(PARAGRAPH)
                        .ok_or(anyhow::anyhow!("paragraph not found"))?
                        .as_table()
                        .ok_or(anyhow::anyhow!("paragraph is not table"))?;
                    paragraph.borrow_mut().set(CONTENT, ScriptValue::string(""));
                    Ok(ScriptValue::Table(paragraph))
                })
                .map_err(|e| anyhow::anyhow!(e))?;
        }

        Ok(())
    }
}
