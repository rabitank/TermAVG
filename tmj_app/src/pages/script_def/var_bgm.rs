use std::time::Duration;

use tmj_core::{
    audio::AudioOp,
    script::{Interpreter, ScriptValue, TypeName, lower_str},
};

use crate::{
    audio::{self, AUDIOM, load_audio},
    pages::script_def::BaseVariable,
};

lower_str!(BGM);
// method
lower_str!(SET);
lower_str!(STOP);

// member
lower_str!(SOURCE);

#[derive(TypeName)]
pub struct VBgm;

impl BaseVariable for VBgm {
    fn regist_to_ctx_impl(ctx: &mut tmj_core::script::ScriptContext) -> anyhow::Result<()> {
        ctx.set_global_table(BGM);

        let _ = ctx.set_table_member(BGM, SOURCE, ScriptValue::Nil);

        let _ = ctx.set_table_func(BGM, SET, |_ctx, args| {
            
            let path = args[0].as_str().expect("!!! bgm error arg type");

            Interpreter::eval(
                format!(
                    "set {BGM}.{SOURCE} \"{}\"",
                    args.last().unwrap().as_string().unwrap()
                ),
                _ctx.clone(),
            )?;

            let source = load_audio(path).expect("!!! bgm load faild");
            let fade_type = args
                .get(1)
                .unwrap_or(&ScriptValue::Nil)
                .as_str()
                .unwrap_or(audio::FADE_IN);

            AUDIOM.with_borrow_mut(move |a| {
                tracing::info!("bgm fading! {}", path);
                match fade_type {
                    audio::FADE_IN => {
                        a.track_mut(&audio::Tracks::Bgm).unwrap().queue_batch(vec![
                            AudioOp::fade_out(Duration::from_millis(800)),
                            AudioOp::wait(Duration::from_millis(850)),
                            AudioOp::fade_in(source, Duration::from_millis(800)),
                        ]);
                    }
                    audio::TRANSITION => {
                        a.transition(
                            &audio::Tracks::Bgm,
                            &audio::Tracks::Bgm,
                            source,
                            Duration::from_millis(1000),
                            tmj_core::audio::FadeCurve::EaseInOut,
                        );
                    }
                    _ => {}
                }
            });

            Ok(ScriptValue::Nil)
        });

        Ok(())
    }
}
