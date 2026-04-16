use serde::{Deserialize, Serialize};
use std::{collections::HashMap, fs, rc::Rc};
use tmj_core::{
    pathes,
    script::{
        Command, Interpreter, IntoScriptValue, RegistableType, ScriptValue, Table, TypeName,
        lower_str,
    },
};

use crate::pages::script_def::{env, var_frame};

lower_str!(CHARACTER);
/// 创建新的 Character Table
#[derive(Serialize, Deserialize, Debug, Default, TypeName)]
pub struct Character {
    _current_face: String,
    display: String,
    face: HashMap<String, String>,
    voice: HashMap<String, String>,
    #[serde(flatten)] // 将额外字段展平到顶层
    extra: toml::Table, // 其他任意字典数据
}

///character member
lower_str!(DISPLAY);
lower_str!(_FACES);
lower_str!(_VOICES);
lower_str!(CURRENT_FACE);

///character methods
lower_str!(SAY);
lower_str!(GET_CURRENT_STAND);

impl RegistableType for Character {
    fn create_class_table(ctx: &tmj_core::script::ScriptContext, args: Vec<ScriptValue>) -> Table {
        match args.get(0) {
            Some(setting_file) if setting_file.is_string() => {
                // 1. deserialize rust character
                let setting_file = setting_file.as_str().unwrap();
                let file = pathes::path(setting_file);
                if !file.is_file() {
                    tracing::error!("{} is not exist", setting_file);
                    return Table::new();
                }
                let toml_str = fs::read_to_string(file).unwrap();
                let character: Character = match toml::from_str(&toml_str) {
                    Ok(res) => res,
                    Err(_info) => {
                        tracing::error!("when create character from file: {}", _info);
                        Character::default()
                    }
                };

                // 2. to table data
                let mut table = Table::new();
                table.set(DISPLAY, character.display.into_script_val());
                table.set(_FACES, character.face.into_script_val());
                table.set(_VOICES, character.voice.into_script_val());
                table.set(CURRENT_FACE, character._current_face.into_script_val());
                table
            }
            None => {
                tracing::error!("character args error: No args ");
                Table::new()
            }
            _ => {
                tracing::error!("character args error: wrong arg 0");
                Table::new()
            }
        }
    }

    fn attach_table_methods(
        _ctx: &tmj_core::script::ContextRef,
        table_rc: &Rc<std::cell::RefCell<Table>>,
    ) -> Result<(), String> {
        {
            let table_clone = Rc::clone(table_rc);
            table_rc.borrow_mut().set(
                GET_CURRENT_STAND,
                ScriptValue::function(GET_CURRENT_STAND, move |_ctx, _args| {
                    let cur_face = table_clone.borrow().get(CURRENT_FACE).clone().unwrap();
                    Ok(table_clone
                        .borrow()
                        .get(format!("{_FACES}.{}", cur_face.as_str().unwrap()).as_str())
                        .ok_or(anyhow::anyhow!("{:?} not in _FACE", cur_face))?)
                }),
            );
        }

        {
            let table_clone = Rc::clone(table_rc);
            table_rc.borrow_mut().set(
                SAY,
                ScriptValue::function(SAY, move |ctx, args| {
                    if args.is_empty() {
                        anyhow::bail!("say requires text argument".to_string());
                    }
                    let _text = args[0].as_str().unwrap_or("");
                    tracing::info!(
                        "{:?} is saying {}",
                        table_clone
                            .borrow()
                            .get(DISPLAY)
                            .as_ref()
                            .unwrap()
                            .as_str()
                            .unwrap(),
                        _text
                    );

                    Interpreter::eval(
                        vec![
                            Command::Once {
                                path: format!("{:}.{:}", env::FRAME, var_frame::SPEAKER),
                                args: vec![
                                    table_clone
                                        .borrow()
                                        .get(DISPLAY)
                                        .unwrap_or(ScriptValue::String("error_c_name".to_string())),
                                ],
                            },
                            Command::Once {
                                path: format!("{:}.{:}", env::FRAME, var_frame::CONTENT),
                                args: vec![
                                    ScriptValue::string(_text)
                                ],
                            },
                        ],
                        ctx.clone(),
                    )
                    .map_err(|e| anyhow::anyhow!(e))?;

                    Ok(ScriptValue::nil())
                }),
            );
        }
        Ok(())
    }
}
