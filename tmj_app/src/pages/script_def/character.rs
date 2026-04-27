use serde::{Deserialize, Serialize};
use std::{cell::RefCell, collections::HashMap, fs, rc::Rc};
use tmj_core::{
    pathes,
    script::{IntoScriptValue, RegistableType, ScriptValue, TabelGet, Table, TypeName, lower_str},
};

use crate::pages::{pipeline::with_behaviour_mut_from_ctx, pop_items::DialogueRecord};

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

impl RegistableType for Character {
    fn create_class_table(
        ctx: &mut tmj_core::script::ScriptContext,
        args: Vec<ScriptValue>,
    ) -> Table {
        match args.get(0) {
            Some(setting_file) if setting_file.is_string() => {
                // 1. deserialize rust character
                let setting_file = setting_file.as_str().unwrap();
                let file = pathes::path(setting_file);
                if !file.is_file() {
                    tracing::error!("{} is not exist", setting_file);
                    let id = ctx.alloc_table_id();
                    return Table::with_tuid(id);
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
                let root_id = ctx.alloc_table_id();
                let mut table = Table::with_tuid(root_id);
                table.set(DISPLAY, character.display.into_script_val(), None);
                table.set(
                    _STANDS,
                    ScriptValue::Table(Rc::new(RefCell::new(Table::from_hashmap_with_tuid(
                        ctx.alloc_table_id(),
                        character.stands,
                    )))),
                    None,
                );
                table.set(
                    _FACES,
                    ScriptValue::Table(Rc::new(RefCell::new(Table::from_hashmap_with_tuid(
                        ctx.alloc_table_id(),
                        character.faces,
                    )))),
                    None,
                );
                table.set(
                    _VOICES,
                    ScriptValue::Table(Rc::new(RefCell::new(Table::from_hashmap_with_tuid(
                        ctx.alloc_table_id(),
                        character.voice,
                    )))),
                    None,
                );
                table.set(FACE, character._current_face.into_script_val(), None);
                table
            }
            None => {
                tracing::error!("character args error: No args ");
                Table::with_tuid(ctx.alloc_table_id())
            }
            _ => {
                tracing::error!("character args error: wrong arg 0");
                Table::with_tuid(ctx.alloc_table_id())
            }
        }
    }

    fn attach_table_methods(
        ctx: &tmj_core::script::ContextRef,
        table_rc: &Rc<std::cell::RefCell<Table>>,
    ) -> Result<(), String> {
        {
            let table_clone = Rc::clone(table_rc);
            table_rc.borrow_mut().set(
                SAY,
                ScriptValue::function(SAY, move |ctx, args| {
                    if args.is_empty() {
                        anyhow::bail!("say requires text argument".to_string());
                    }
                    let text = args[0].as_str().unwrap_or("");
                    let speaker_name = table_clone.get(DISPLAY)?;

                    tracing::info!("{:?} is saying {}", speaker_name.as_str().unwrap(), text);
                    let cur_face = table_clone.get(FACE)?;
                    let faces_sv = table_clone.get(_FACES)?;
                    let face_path = ctx
                        .borrow()
                        .resolve_table_value(&faces_sv)
                        .ok()
                        .and_then(|faces_tbl| {
                            faces_tbl.borrow().get(cur_face.as_str().unwrap(), None)
                        })
                        .unwrap_or_else(|| {
                            tracing::warn!("got character face img failed; set face none");
                            ScriptValue::String("".into())
                        });

                    let speaker_name = speaker_name.as_string().cloned().unwrap();
                    let face_path = face_path.as_string().cloned().unwrap();

                    crate::pages::pop_items::HISTORY_LS
                        .lock()
                        .unwrap()
                        .push(DialogueRecord {
                            id: ctx.borrow().session_counter(),
                            speaker: speaker_name.clone(),
                            content: text.to_string(),
                        });

                    with_behaviour_mut_from_ctx::<
                        crate::pages::pipeline::dialogue_frame::FrameBehaviour,
                        _,
                    >(ctx, |b| {
                        b.export_say(speaker_name, face_path, text.to_string());
                    })?;

                    Ok(ScriptValue::nil())
                }),
                Some(ctx),
            );
        }
        Ok(())
    }
}
