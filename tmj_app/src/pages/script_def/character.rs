use serde::{Deserialize, Serialize};
use std::{collections::HashMap, fs, rc::Rc};
use tmj_core::{
    pathes,
    script::{
        Command, Interpreter, IntoScriptValue, RegistableType, ScriptValue, TabelGet, Table,
        TypeName, lower_str,
    },
};

use crate::pages::script_def::{env, var_frame};

lower_str!(CHARACTER);
/// 创建新的 Character Table
#[derive(Serialize, Deserialize, Debug, Default, TypeName)]
pub struct Character {
    _current_face: String,
    display: String,
    stands: HashMap<String, String>,
    faces: HashMap<String, String>,
    voice: HashMap<String, String>,
    #[serde(flatten)] // 将额外字段展平到顶层
    extra: toml::Table, // 其他任意字典数据
}

//character member
lower_str!(DISPLAY);
lower_str!(_STANDS);
lower_str!(_FACES);
lower_str!(_VOICES);
lower_str!(FACE);

//character methods
lower_str!(SAY);
lower_str!(GET_CURRENT_STAND);

impl RegistableType for Character {
    fn create_class_table(_ctx: &tmj_core::script::ScriptContext, args: Vec<ScriptValue>) -> Table {
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
                table.set(_STANDS, character.stands.into_script_val());
                table.set(_FACES, character.faces.into_script_val());
                table.set(_VOICES, character.voice.into_script_val());
                table.set(FACE, character._current_face.into_script_val());
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
                    let cur_face = table_clone.get(FACE)?;
                    let stand_img_path = table_clone
                        .get(_STANDS)?
                        .as_table()
                        .unwrap()
                        .get(cur_face.as_str().unwrap())?;
                    Ok(stand_img_path)
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
                    let speaker_name = table_clone.get(DISPLAY)?;

                    tracing::info!("{:?} is saying {}", speaker_name.as_str().unwrap(), _text);
                    let cur_face = table_clone.get(FACE)?;
                    let face_path = table_clone
                        .get(_FACES)?
                        .as_table()
                        .unwrap()
                        .get(cur_face.as_str().unwrap()).unwrap_or_else(|e| {
                            tracing::warn!("got character face img failed: {:?}\n set face none", e);
                            ScriptValue::String("".into())
                        });

                    Interpreter::eval_cmds(
                        vec![
                            Command::Once {
                                path: format!("{:}.{:}", env::FRAME, var_frame::SPEAKER),
                                args: vec![speaker_name],
                            },
                            Command::Once {
                                path: format!("{:}.{:}", env::FRAME, var_frame::CONTENT),
                                args: vec![ScriptValue::string(_text)],
                            },
                            Command::Once {
                                path: format!("{:}", env::FACE_PATH),
                                args: vec![face_path],
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
