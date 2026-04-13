use std::time::Duration;

use tmj_core::{
    audio::{AudioOp, FadeCurve},
    pathes,
    script::{ContextRef, IntoScriptValue, ScriptContext, ScriptValue, lower_str},
};

use crate::{
    SETTING, audio::{self, AUDIOM, Tracks, load_audio}, pages::{pipeline::character_ls, script_def::{BaseVariable, Character, TextObj, VCharacterLs, VFrame, text_obj}}
};

macro_rules! script_str {
    ($ctx:ident, $name:ident) => {
        $ctx.reg_val($name, ScriptValue::String($name.to_string()));
    }; // 两个参数：ctx, name -> 值 = name 变量的字符串值
    // 三个参数：ctx, name, value -> 值 = value 转换为 String
    ($ctx:expr, $name:ident, $value:expr) => {
        $ctx.reg_val($name, ScriptValue::String(Into::<String>::into($value)));
    };
}

// global member
lower_str!(BGIMG_PATH);
lower_str!(_TEXT_OBJ);
lower_str!(FACE_PATH);

pub use super::var_character_ls::CHARACTER_LS;
pub use super::var_frame::FRAME;


// global function
lower_str!(BGM);
lower_str!(TEXT);
lower_str!(DISPLAY_NAME);
lower_str!(SAVE_TO);

fn regist_base_gvar(ctx: &mut ScriptContext) -> anyhow::Result<()> {
    VCharacterLs::regist_to_ctx(ctx)?;
    VFrame::regist_to_ctx(ctx)?;
    Ok(())
}

pub fn init_env(ctx: ContextRef) {
    {
        let ctx_ref = ctx.clone();
        let _text_obj = TextObj::default().into_script_class_table(&ctx_ref);
        ctx.borrow_mut().reg_val(_TEXT_OBJ, _text_obj);
    }

    let mut ctx = ctx.borrow_mut();
    {
        use audio::*;
        script_str!(ctx, FADE_IN);
        script_str!(ctx, FADE_OUT);
        script_str!(ctx, TRANSITION);
        script_str!(ctx, BGIMG_PATH, SETTING.default_bg_img.to_str().unwrap());
        script_str!(ctx, FACE_PATH, SETTING.default_face_img.to_str().unwrap());
    }
    {
        ctx.type_registry.register::<Character>();
        ctx.type_registry.register::<TextObj>();
    }
    {
        ctx.reg_func(SAVE_TO, |_c, args| {
            let table = args[0]
                .as_table()
                .ok_or(anyhow::anyhow!("args 0 is not table"))?;
            let target_path = args[1]
                .as_string()
                .ok_or(anyhow::anyhow!("args 1 is not str"))?;
            let ct = toml::to_string(&table.into_script_val())?;
            let target_path = pathes::path(target_path);
            std::fs::write(target_path, ct)?;
            Ok(ScriptValue::Nil)
        });
    }

    {
        ctx.reg_func(TEXT, |c, args| {
            let raw_text = args[0].as_str().expect("!!! error empty text");
            let text_obj = c
                .borrow()
                .get_val(_TEXT_OBJ)
                .expect("!!! no text obj")
                .as_table()
                .unwrap();
            text_obj
                .borrow_mut()
                .set(text_obj::CONTENT, raw_text.to_string().into_script_val());
            Ok(c.borrow().get_val(_TEXT_OBJ).unwrap())
        });
    }

    {
        ctx.reg_func(BGM, |_ctx, args| {
            let path = args[0].as_str().expect("!!! bgm error arg type");
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
                        a.track_mut(&Tracks::Bgm).unwrap().queue_batch(vec![
                            AudioOp::fade_out(Duration::from_millis(800)),
                            AudioOp::wait(Duration::from_millis(850)),
                            AudioOp::fade_in(source, Duration::from_millis(800)),
                        ]);
                    }
                    audio::TRANSITION => {
                        a.transition(
                            &Tracks::Bgm,
                            &Tracks::Bgm,
                            source,
                            Duration::from_millis(1000),
                            FadeCurve::EaseInOut,
                        );
                    }
                    _ => {}
                }
            });

            Ok(ScriptValue::Nil)
        });
    }

    {
        ctx.reg_func("create_default_character", |_ctx, args| {
            let path = args[0].as_string().unwrap();
            let character = Character::default();
            let ct = toml::to_string(&character)?;
            let path = pathes::path(path);
            let _ = std::fs::write(path, ct)?;
            Ok(ScriptValue::Nil)
        })
    }

    regist_base_gvar(&mut ctx);
}
