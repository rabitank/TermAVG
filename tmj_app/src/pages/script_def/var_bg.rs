use tmj_core::script::{ScriptValue, TypeName, lower_str};

use crate::pages::{pipeline::{BackgroundBehaviour, with_behaviour_mut_from_ctx}, script_def::BaseVariable};

lower_str!(BG);

// method
lower_str!(SET);
lower_str!(TRANS_TO);
lower_str!(SHOW_EDGE);
lower_str!(HIDE_EDGE);

// member
lower_str!(M_IMAGE);
lower_str!(M_IS_EDGE);

/// Bg: Background Object
#[derive(TypeName)]
pub struct VBg;

impl BaseVariable for VBg {
    fn regist_to_ctx_impl(ctx: &mut tmj_core::script::ScriptContext) -> anyhow::Result<()> {
        ctx.set_global_table(BG);

        let _ = ctx.set_table_member(BG, M_IMAGE, ScriptValue::String("".into()));
        let _ = ctx.set_table_member(BG, M_IS_EDGE, ScriptValue::Bool(true));

        let _ = ctx.set_table_func(BG, HIDE_EDGE, |ctx, _args| {
            {
                let mut c = ctx.borrow_mut();
                c.set_table_member(BG, M_IS_EDGE, ScriptValue::bool(false))
                    .map_err(|e| anyhow::anyhow!(e))?;

            }
            with_behaviour_mut_from_ctx::<BackgroundBehaviour, _>(ctx, |b: &mut BackgroundBehaviour| {
                b.export_hide_edge();
            });

            Ok(ScriptValue::Nil)
        });


        let _ = ctx.set_table_func(BG, SHOW_EDGE, |ctx, _args| {
            {
                let mut c = ctx.borrow_mut();
                c.set_table_member(BG, M_IS_EDGE, ScriptValue::bool(true))
                    .map_err(|e| anyhow::anyhow!(e))?;

            }
            with_behaviour_mut_from_ctx::<BackgroundBehaviour, _>(ctx, |b: &mut BackgroundBehaviour| {
                b.export_show_edge();
            });

            Ok(ScriptValue::Nil)
        });

        let _ = ctx.set_table_func(BG, SET, |ctx, args| {
            let new_img_path = args.first().unwrap().clone();
            if !new_img_path.is_string() {
                anyhow::bail!("bg.set args should be string first");
            }
            {
                let mut c = ctx.borrow_mut();
                c.set_table_member(BG, M_IMAGE, new_img_path.clone())
                    .map_err(|e| anyhow::anyhow!(e))?;

            }
            let new_img_path = new_img_path.as_string().unwrap().to_string();
            with_behaviour_mut_from_ctx::<BackgroundBehaviour, _>(ctx, |b: &mut BackgroundBehaviour| {
                b.export_set(new_img_path);
            });
            Ok(ScriptValue::Nil)
        });

        let _ = ctx.set_table_func(BG, TRANS_TO, |ctx, args| {
            let new_path = args
                .first()
                .and_then(|v| v.as_str())
                .ok_or(anyhow::anyhow!("bg.trans_to requires (new_image, duration_sec)"))?
                .to_string();
            let duration_sec = args
                .get(1)
                .and_then(|v| v.to_number())
                .filter(|d| d.is_finite() && *d > 0.0)
                .ok_or(anyhow::anyhow!(
                    "bg.trans_to second arg should be a positive number (seconds)"
                ))?;

            let table = ctx
                .borrow()
                .get_global_val(BG)
                .ok_or(anyhow::anyhow!("bg not found"))?
                .as_table_or_resolve(ctx)
                .ok_or(anyhow::anyhow!("bg is not table"))?;
            {
                let mut t = table.borrow_mut();
                t.set(M_IMAGE, ScriptValue::String(new_path.clone()), None);
            }

            with_behaviour_mut_from_ctx::<BackgroundBehaviour, _>(ctx, |b: &mut BackgroundBehaviour| {
                b.export_trans_to(new_path, duration_sec);
            });
            Ok(ScriptValue::Nil)
        });

        Ok(())
    }
}
