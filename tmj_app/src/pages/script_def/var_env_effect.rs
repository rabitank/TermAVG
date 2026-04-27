use tmj_core::{
    audio::AudioOp,
    script::{ScriptValue, TypeName, lower_str},
};

use crate::{
    audio::{AUDIOM, Tracks, load_audio},
    pages::script_def::BaseVariable,
};

lower_str!(ENV_EFFECT);
lower_str!(SET);
lower_str!(SOURCE);

#[derive(TypeName)]
pub struct VEnvEffect;

impl BaseVariable for VEnvEffect {
    fn regist_to_ctx_impl(ctx: &mut tmj_core::script::ScriptContext) -> anyhow::Result<()> {
        ctx.set_global_table(ENV_EFFECT);
        let _ = ctx.set_table_member(ENV_EFFECT, SOURCE, ScriptValue::Nil);

        let _ = ctx.set_table_func(ENV_EFFECT, SET, |ctx, args| {
            let path = args
                .first()
                .and_then(|x| x.as_str())
                .ok_or(anyhow::anyhow!("env_effect.set requires file path string"))?;

            {
                let mut c = ctx.borrow_mut();
                if path.is_empty() {
                    c.set_table_member(ENV_EFFECT, SOURCE, ScriptValue::Nil)
                        .map_err(|e| anyhow::anyhow!(e))?;
                } else {
                    c.set_table_member(
                        ENV_EFFECT,
                        SOURCE,
                        ScriptValue::String(path.to_string()),
                    )
                    .map_err(|e| anyhow::anyhow!(e))?;
                }
            }

            if path.is_empty() {
                AUDIOM.with_borrow_mut(|a| {
                    if let Some(t) = a.track_mut(&Tracks::EnvEffect) {
                        t.stop();
                    }
                });
            } else {
                let source = load_audio(path)
                    .map_err(|e| anyhow::anyhow!("env_effect: failed to load audio: {e}"))?;
                AUDIOM.with_borrow_mut(|a| {
                    if let Some(t) = a.track_mut(&Tracks::EnvEffect) {
                        t.stop();
                        t.queue(AudioOp::play(source, 1.0));
                    }
                });
            }

            Ok(ScriptValue::Nil)
        });

        Ok(())
    }
}
