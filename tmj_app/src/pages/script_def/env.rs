use tmj_core::{
    audio::AudioOp,
    pathes,
    script::{
        ContextRef, Interpreter, IntoScriptValue, ScriptContext, ScriptValue, lower_str,
    },
};

use crate::audio::{AUDIOM, Tracks, load_audio};

use crate::
    pages::
        script_def::{
            BaseVariable, Character, TextObj, VBg, VBgm, VChapter, VCharacterLs, VEnvEffect, VFrame,
            VLayer, VParagraph, var_frame, var_layer,
        }
    
;

macro_rules! script_str {
    ($ctx:ident, $name:ident) => {
        $ctx.set_global_val($name, ScriptValue::String($name.to_string()));
    }; // 两个参数：ctx, name -> 值 = name 变量的字符串值
    // 三个参数：ctx, name, value -> 值 = value 转换为 String
    ($ctx:expr, $name:ident, $value:expr) => {
        $ctx.set_global_val($name, ScriptValue::String(Into::<String>::into($value)));
    };
}

// global member
lower_str!(BGIMG_PATH);
lower_str!(FACE_PATH);
lower_str!(BEHAVIOURS_MAP);
pub use super::var_bg::BG;
pub use super::var_bgm::BGM;
pub use super::var_env_effect::ENV_EFFECT;
pub use super::var_chapter::CHAPTER;
pub use super::var_character_ls::CHARACTER_LS;
pub use super::var_frame::FRAME;
pub use super::var_layer::LAYERS;
pub use super::var_paragraph::PARAGRAPH;

// global function
lower_str!(TEXT);
lower_str!(DISPLAY_NAME);
lower_str!(SAVE_TO);
lower_str!(ADD_LAYER);
lower_str!(DEL_LAYER);
lower_str!(SEE);
lower_str!(VOICE);

fn regist_base_gvar(ctx: &mut ScriptContext) -> anyhow::Result<()> {
    VCharacterLs::regist_to_ctx(ctx)?;
    VFrame::regist_to_ctx(ctx)?;
    VParagraph::regist_to_ctx(ctx)?;
    VLayer::regist_to_ctx(ctx)?;
    VBgm::regist_to_ctx(ctx)?;
    VEnvEffect::regist_to_ctx(ctx)?;
    VChapter::regist_to_ctx(ctx)?;
    VBg::regist_to_ctx(ctx)?;
    Ok(())
}

pub fn init_env(ctx: ContextRef, behaviours: crate::pages::pipeline::BehaviourMap) {
    {
        ctx.borrow_mut()
            .set_global_val(DISPLAY_NAME, ScriptValue::string(""));
    }

    let mut ctx = ctx.borrow_mut();
    {
        use crate::audio::*;
        script_str!(ctx, FADE_IN);
        script_str!(ctx, FADE_OUT);
        script_str!(ctx, TRANSITION);
        script_str!(ctx, FACE_PATH, "");
        ctx.set_global_val(BEHAVIOURS_MAP, ScriptValue::rust_object(behaviours));
    }
    {
        ctx.type_registry.register::<Character>();
        ctx.type_registry.register::<TextObj>();
    }
    let _ = regist_base_gvar(&mut ctx);
    {
        ctx.set_global_func(SAVE_TO, |c, args| {
            let table = args[0]
                .as_table_or_resolve(c)
                .ok_or(anyhow::anyhow!("args 0 is not a table or handle"))?;
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
        ctx.set_global_func(ADD_LAYER, |c, args| {
            if args.len() < 2 {
                anyhow::bail!("add_layer requires at least type and source/name");
            }

            let layer_type = args[0]
                .as_str()
                .ok_or(anyhow::anyhow!(
                    "add_layer arg0 should be layer type string"
                ))?
                .to_string();

            let (name, source) = if args.len() >= 3 {
                let name = args[1]
                    .as_str()
                    .ok_or(anyhow::anyhow!(
                        "add_layer arg1 should be layer name string"
                    ))?
                    .to_string();
                let source = args[2]
                    .as_str()
                    .ok_or(anyhow::anyhow!("add_layer arg2 should be source string"))?
                    .to_string();
                (name, source)
            } else {
                let source = args[1]
                    .as_str()
                    .ok_or(anyhow::anyhow!("add_layer arg1 should be source string"))?
                    .to_string();
                let name = std::path::Path::new(&source)
                    .file_stem()
                    .and_then(|x| x.to_str())
                    .unwrap_or("layer")
                    .to_string();
                (name, source)
            };

            let layers = c
                .borrow()
                .get_global_val(LAYERS)
                .ok_or(anyhow::anyhow!("layers not found"))?
                .as_table_or_resolve(c)
                .ok_or(anyhow::anyhow!("layers should be table"))?;

            let layer_rc = c.borrow_mut().alloc_table_rc();
            {
                let mut layer_item = layer_rc.borrow_mut();
                layer_item.set(var_layer::LAYER_TYPE, ScriptValue::string(layer_type), None);
                layer_item.set(var_layer::SOURCE, ScriptValue::string(source), None);
                layer_item.set(var_layer::VISIBLE, ScriptValue::bool(true), None);
            }
            layers.borrow_mut().set(
                name,
                ScriptValue::Table(layer_rc),
                Some(c),
            );

            Ok(ScriptValue::Table(layers))
        });
    }

    {
        ctx.set_global_func(DEL_LAYER, |c, args| {
            let name = args
                .first()
                .and_then(|x| x.as_str())
                .ok_or(anyhow::anyhow!("del_layer requires name string"))?;
            let layers = c
                .borrow()
                .get_global_val(LAYERS)
                .ok_or(anyhow::anyhow!("layers not found"))?
                .as_table_or_resolve(c)
                .ok_or(anyhow::anyhow!("layers should be table"))?;
            layers.borrow_mut().remove(name);
            Ok(ScriptValue::Table(layers))
        });
    }

    {
        ctx.set_global_func(TEXT, |c, args| {
            let raw_text = args
                .first()
                .and_then(|x| x.as_str())
                .ok_or(anyhow::anyhow!("text requires content string"))?;

            Interpreter::eval_cmds(
                vec![
                    // 设置这一回的文本
                    tmj_core::script::Command::Once {
                        path: format!("{FRAME}.{:}", var_frame::CONTENT),
                        args: vec![ScriptValue::string(raw_text)],
                    },
                    // text 用于旁白：隐藏头像
                    tmj_core::script::Command::Once {
                        path: FACE_PATH.to_string(),
                        args: vec![ScriptValue::string("")],
                    },
                ],
                c.clone(),
            )
            .map_err(|e| anyhow::anyhow!(e))?;

            Ok(ScriptValue::Nil)
        });
    }

    {
        ctx.set_global_func("create_default_character", |_ctx, args| {
            let path = args[0].as_string().unwrap();
            let character = Character::default();
            let ct = toml::to_string(&character)?;
            let path = pathes::path(path);
            let _ = std::fs::write(path, ct)?;
            Ok(ScriptValue::Nil)
        })
    }

    {
        ctx.set_global_func(SEE, |_ctx, args| {
            let name = args
                .first()
                .and_then(|x| x.as_str())
                .ok_or(anyhow::anyhow!("see requires visual element name string"))?;
            crate::pages::dialogue::see_visual_element(name)?;
            Ok(ScriptValue::Nil)
        });
    }

    {
        ctx.set_global_func(VOICE, |_ctx, args| {
            let path = args
                .first()
                .and_then(|x| x.as_str())
                .ok_or(anyhow::anyhow!("voice requires audio file path string"))?;
            if path.is_empty() {
                AUDIOM.with_borrow_mut(|a| {
                    if let Some(t) = a.track_mut(&Tracks::Voice) {
                        t.stop();
                    }
                });
                return Ok(ScriptValue::Nil);
            }
            let source =
                load_audio(path).map_err(|e| anyhow::anyhow!("voice: failed to load audio: {e}"))?;
            AUDIOM.with_borrow_mut(|a| {
                if let Some(t) = a.track_mut(&Tracks::Voice) {
                    t.stop();
                    t.queue(AudioOp::play(source, 1.0));
                }
            });
            Ok(ScriptValue::Nil)
        });
    }

}
