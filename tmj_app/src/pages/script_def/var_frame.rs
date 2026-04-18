use tmj_core::script::{ScriptValue, TypeName, lower_str};

use crate::pages::script_def::BaseVariable;

lower_str!(FRAME);

// members
lower_str!(VISIBLE);
lower_str!(MODE);
lower_str!(CONTENT);
lower_str!(SPEAKER);
lower_str!(TYPEWRITER_ENABLE);
lower_str!(TYPEWRITER_SPEED);
lower_str!(TYPEWRITER_PROGRESS);
lower_str!(TYPEWRITER_LAST_CONTENT);

// methods
lower_str!(SHOW);
lower_str!(HIDE);
lower_str!(SET_MODE);

#[derive(TypeName)]
pub struct VFrame;

impl BaseVariable for VFrame {
    fn regist_to_ctx_impl(ctx: &mut tmj_core::script::ScriptContext) -> anyhow::Result<()> {
        ctx.set_global_table(FRAME);
        let _ = ctx.set_table_member(FRAME, VISIBLE, ScriptValue::bool(true));
        let _ = ctx.set_table_member(FRAME, MODE, ScriptValue::string("normal"));
        let _ = ctx.set_table_member(FRAME, CONTENT, ScriptValue::string(""));
        let _ = ctx.set_table_member(FRAME, SPEAKER, ScriptValue::string(""));
        let _ = ctx.set_table_member(FRAME, TYPEWRITER_ENABLE, ScriptValue::bool(true));
        let _ = ctx.set_table_member(FRAME, TYPEWRITER_SPEED, ScriptValue::float(40.0));
        let _ = ctx.set_table_member(FRAME, TYPEWRITER_PROGRESS, ScriptValue::float(0.0));
        let _ = ctx.set_table_member(FRAME, TYPEWRITER_LAST_CONTENT, ScriptValue::string(""));

        {
            let _ = ctx
                .set_table_func(FRAME, SHOW, |ctx, _args| {
                    let frame = ctx
                        .borrow()
                        .get_global_val(FRAME)
                        .ok_or(anyhow::anyhow!("frame not found"))?
                        .as_table()
                        .ok_or(anyhow::anyhow!("frame is not table"))?;
                    frame.borrow_mut().set(VISIBLE, ScriptValue::bool(true));
                    Ok(ScriptValue::Table(frame))
                })
                .map_err(|e| anyhow::anyhow!(e))?;
        }

        {
            let _ = ctx
                .set_table_func(FRAME, HIDE, |ctx, _args| {
                    let frame = ctx
                        .borrow()
                        .get_global_val(FRAME)
                        .ok_or(anyhow::anyhow!("frame not found"))?
                        .as_table()
                        .ok_or(anyhow::anyhow!("frame is not table"))?;
                    frame.borrow_mut().set(VISIBLE, ScriptValue::bool(false));
                    Ok(ScriptValue::Table(frame))
                })
                .map_err(|e| anyhow::anyhow!(e))?;
        }

        {
            let _ = ctx
                .set_table_func(FRAME, SET_MODE, |ctx, args| {
                    let mode = args
                        .first()
                        .and_then(|x| x.as_str())
                        .ok_or(anyhow::anyhow!("frame.set_mode requires mode string"))?;
                    let frame = ctx
                        .borrow()
                        .get_global_val(FRAME)
                        .ok_or(anyhow::anyhow!("frame not found"))?
                        .as_table()
                        .ok_or(anyhow::anyhow!("frame is not table"))?;
                    frame.borrow_mut().set(MODE, ScriptValue::string(mode));
                    Ok(ScriptValue::Table(frame))
                })
                .map_err(|e| anyhow::anyhow!(e))?;
        }
        Ok(())
    }
}
