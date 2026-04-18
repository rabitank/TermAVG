use tmj_core::script::{ScriptValue, TypeName, lower_str};

use crate::pages::{
    script_def::{BaseVariable, Character},
};

#[derive(TypeName)]
pub struct VCharacterLs;

// var name
lower_str!(CHARACTER_LS);

// funcs
lower_str!(SET_CHARACTERS);

impl BaseVariable for VCharacterLs {
    fn regist_to_ctx_impl(ctx: &mut tmj_core::script::ScriptContext) -> anyhow::Result<()> {
        ctx.set_global_table(CHARACTER_LS);

        // set characters
        {
            let _ = ctx
                .set_table_func(CHARACTER_LS, SET_CHARACTERS, |ctx, args| {
                    let c_ls = ctx
                        .borrow()
                        .get_global_val(CHARACTER_LS)
                        .unwrap()
                        .as_table()
                        .unwrap();
                    for (idx, i) in args.iter().enumerate() {
                        let c = i
                            .as_table()
                            .ok_or(anyhow::anyhow!("expect table but {idx} arg is not!"))
                            .map(|i| {
                                if i.borrow().is_ins::<Character>() {
                                    Ok(i)
                                } else {
                                    Err(anyhow::anyhow!("expect character but {idx} arg is not!"))
                                }
                            })??;
                        c_ls.borrow_mut().set_int(idx as i64, ScriptValue::Table(c));
                    }
                    Ok(ScriptValue::Table(c_ls))
                })
                .map_err(|e| anyhow::anyhow!(e))?;
        }
        Ok(())
    }
}
