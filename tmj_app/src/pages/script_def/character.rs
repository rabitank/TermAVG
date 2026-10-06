use anyhow::Context;
use serde::{Deserialize, Serialize};
use std::{cell::RefCell, collections::HashMap, fs, rc::Rc};
use tmj_core::{
    pathes,
    script::{IntoScriptValue, RegistableType, ScriptValue, TabelGet, Table, TypeName, script_sym},
};

use crate::{
    pages::{
        behaviour::{
            CharactersStage, animation::offset_shift::ShiftDirection,
            with_behaviour_mut_from_ctx_rc,
        },
        pop_items::DialogueRecord,
    },
    utils::script_args::{
        parse_arg, parse_duration, parse_member, parse_required_arg, parse_required_member,
    },
};

script_sym!(CHARACTER, Type, "可构造的角色类型");
/// 创建新的 Character Table
#[derive(Serialize, Deserialize, Debug, Default, TypeName)]
pub struct Character {
    _current_face: String,
    display: String,
    #[serde(default = "default_show_face")]
    show_face: bool,
    stands: HashMap<String, String>,
    faces: HashMap<String, String>,
    voice: HashMap<String, String>,
    #[serde(flatten)] // 将额外字段展平到顶层
    extra: toml::Table, // 其他任意字典数据
}

/// `show_face` 缺省默认值：显示头像框
fn default_show_face() -> bool {
    true
}

script_sym!(DISPLAY, Member, "角色显示名");
script_sym!(M_SHOW_FACE, Member, "say/sadd 是否显示头像框");
script_sym!(_STANDS, Member, "立绘表（表情名 → 图片路径）");
script_sym!(_FACES, Member, "表情名列表");
script_sym!(_VOICES, Member, "语音表");
script_sym!(FACE, Member, "当前表情名");
script_sym!(SAY, Function, "角色说话（立绘、文本、语音）");
script_sym!(
    SADD,
    Function,
    "角色说话,直接追加到对话框（立绘、文本、语音）"
);
script_sym!(FADE_IN, Function, "入场：自右向左滑入 8 格并淡入到场上位置");
script_sym!(TO_FACE, Function, "切换表情，带过度动画");
script_sym!(UP, Function, "添加向上偏移动画");
script_sym!(DOWN, Function, "添加向下偏移动画");
script_sym!(LEFT, Function, "添加向左偏移动画");
script_sym!(RIGHT, Function, "添加向右偏移动画");

script_sym!(
    FADE_OUT,
    Function,
    "退场：淡出并从场上列表移除，其余角色平滑移位"
);

impl RegistableType for Character {
    fn create_class_table(
        ctx: &mut tmj_core::script::ScriptContext,
        args: Vec<ScriptValue>,
    ) -> Table {
        match parse_required_arg(&args, 0, ScriptValue::as_string) {
            Ok(setting_file) => {
                let file = pathes::path(&setting_file);
                if !file.is_file() {
                    tracing::error!("{} is not exist", &setting_file);
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
                table.set(M_SHOW_FACE, character.show_face.into_script_val(), None);
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
            Err(e) => {
                tracing::error!("character args error: {e}");
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
                    let text = parse_required_arg(&args, 0, ScriptValue::as_string)?;
                    let speed = parse_arg(&args, 1, 20.0, ScriptValue::to_number);
                    let speaker_name =
                        parse_required_member(&table_clone, DISPLAY, ScriptValue::as_string)?;
                    let show_face = parse_member(&table_clone, M_SHOW_FACE, true, ScriptValue::as_bool);
                    let cur_face =
                        parse_required_member(&table_clone, FACE, ScriptValue::as_string)?;
                    let faces_sv = table_clone.get(_FACES)?;
                    let face_path = if !show_face {
                        String::new()
                    } else {
                        ctx.borrow()
                            .resolve_table_value(&faces_sv)
                            .ok()
                            .and_then(|faces_tbl| faces_tbl.borrow().get(&cur_face, None))
                            .and_then(|v| v.as_string())
                            .unwrap_or_else(|| {
                                tracing::warn!("got character face img failed; set face none");
                                String::new()
                            })
                    };

                    tracing::info!("{speaker_name} is saying {text}");

                    crate::pages::pop_items::HISTORY_LS
                        .lock()
                        .unwrap()
                        .push(DialogueRecord {
                            id: ctx.borrow().session_counter(),
                            speaker: speaker_name.clone(),
                            content: text.to_string(),
                        });

                    with_behaviour_mut_from_ctx_rc::<
                        crate::pages::behaviour::dialogue_frame::FrameBehaviour,
                        _,
                    >(ctx, |b| {
                        b.export_say(speaker_name.clone(), face_path, text.to_string(), speed);
                    })?;

                    Ok(ScriptValue::nil())
                }),
                Some(ctx),
            );
        }
        {
            let table_clone = Rc::clone(table_rc);
            table_rc.borrow_mut().set(
                SADD,
                ScriptValue::function(SADD, move |ctx, args| {
                    let text = parse_required_arg(&args, 0, ScriptValue::as_string)?;
                    let speed = parse_arg(&args, 1, 20.0, ScriptValue::to_number);
                    let speaker_name =
                        parse_required_member(&table_clone, DISPLAY, ScriptValue::as_string)?;
                    let show_face = parse_member(&table_clone, M_SHOW_FACE, true, ScriptValue::as_bool);
                    let cur_face =
                        parse_required_member(&table_clone, FACE, ScriptValue::as_string)?;
                    let faces_sv = table_clone.get(_FACES)?;
                    let face_path = if !show_face {
                        String::new()
                    } else {
                        ctx.borrow()
                            .resolve_table_value(&faces_sv)
                            .ok()
                            .and_then(|faces_tbl| faces_tbl.borrow().get(&cur_face, None))
                            .and_then(|v| v.as_string())
                            .unwrap_or_else(|| {
                                tracing::warn!("got character face img failed; set face none");
                                String::new()
                            })
                    };

                    tracing::info!("{speaker_name} is saying {text}");

                    crate::pages::pop_items::HISTORY_LS
                        .lock()
                        .unwrap()
                        .push(DialogueRecord {
                            id: ctx.borrow().session_counter(),
                            speaker: speaker_name.clone(),
                            content: text.to_string(),
                        });

                    with_behaviour_mut_from_ctx_rc::<
                        crate::pages::behaviour::dialogue_frame::FrameBehaviour,
                        _,
                    >(ctx, |b| {
                        b.export_sadd(speaker_name.clone(), face_path, text.to_string(), speed);
                    })?;

                    Ok(ScriptValue::nil())
                }),
                Some(ctx),
            );
        }
        {
            let table_clone = Rc::clone(table_rc);
            table_rc.borrow_mut().set(
                FADE_IN,
                ScriptValue::function(FADE_IN, move |ctx, args| {
                    let duration = parse_duration(&args, 0, 0.6);
                    with_behaviour_mut_from_ctx_rc::<CharactersStage, _>(ctx, |b| {
                        b.export_fade_in(ctx, &table_clone, duration)
                    })?;
                    Ok(ScriptValue::nil())
                }),
                Some(ctx),
            );
        }

        {
            let table_clone = Rc::clone(table_rc);
            table_rc.borrow_mut().set(
                FADE_OUT,
                ScriptValue::function(FADE_OUT, move |ctx, args| {
                    let duration = parse_duration(&args, 0, 0.2);
                    with_behaviour_mut_from_ctx_rc::<CharactersStage, _>(ctx, |b| {
                        b.export_fade_out(ctx, &table_clone, duration)
                    })?;
                    Ok(ScriptValue::nil())
                }),
                Some(ctx),
            );
        }

        {
            let table_clone = Rc::clone(table_rc);
            table_rc.borrow_mut().set(
                TO_FACE,
                ScriptValue::function(TO_FACE, move |ctx, args| {
                    let face_name = parse_required_arg(&args, 0, ScriptValue::as_string)?;
                    let old_face_name =
                        parse_required_member(&table_clone, FACE, ScriptValue::as_string)?;
                    let duration = parse_duration(&args, 1, 0.2);

                    let old_path = parse_required_member(
                        &table_clone,
                        format!("{_STANDS}.{old_face_name}"),
                        ScriptValue::as_string,
                    )
                    .context("get old stand path field");

                    if old_path.is_err() {
                        tracing::warn!("{old_path:?} to_face pre face no stand image, skip cmd");
                        return Ok(ScriptValue::Nil);
                    }

                    let new_path = parse_required_member(
                        &table_clone,
                        format!("{_STANDS}.{face_name}"),
                        ScriptValue::as_string,
                    )
                    .context("get new stand path field");

                    if new_path.is_err() {
                        tracing::warn!("{new_path:?} to_face new face no stand image, skip cmd");
                        return Ok(ScriptValue::Nil);
                    }
                    table_clone
                        .borrow_mut()
                        .set(FACE, face_name.into_script_val(), None);
                    with_behaviour_mut_from_ctx_rc::<CharactersStage, _>(ctx, |b| {
                        let _ = b.export_to_face(
                            ctx,
                            &table_clone,
                            &old_path.unwrap(),
                            &new_path.unwrap(),
                            duration,
                        );
                    })?;
                    Ok(ScriptValue::nil())
                }),
                Some(ctx),
            );
        }

        {
            let table_clone = Rc::clone(table_rc);
            table_rc.borrow_mut().set(
                UP,
                ScriptValue::function(UP, move |ctx, args| {
                    let direction = ShiftDirection::Up;
                    let distance = parse_required_arg(&args, 0, ScriptValue::as_int)?;
                    let duration = parse_duration(&args, 1, 0.2);
                    with_behaviour_mut_from_ctx_rc::<CharactersStage, _>(ctx, |b| {
                        b.export_character_offset(
                            &ctx,
                            &table_clone,
                            &direction,
                            distance,
                            duration,
                        );
                    })?;
                    Ok(ScriptValue::nil())
                }),
                Some(ctx),
            );
        }

        {
            let table_clone = Rc::clone(table_rc);
            table_rc.borrow_mut().set(
                DOWN,
                ScriptValue::function(DOWN, move |ctx, args| {
                    let direction = ShiftDirection::Down;
                    let distance = parse_required_arg(&args, 0, ScriptValue::as_int)?;
                    let duration = parse_duration(&args, 1, 0.2);
                    with_behaviour_mut_from_ctx_rc::<CharactersStage, _>(ctx, |b| {
                        b.export_character_offset(
                            &ctx,
                            &table_clone,
                            &direction,
                            distance,
                            duration,
                        );
                    })?;
                    Ok(ScriptValue::nil())
                }),
                Some(ctx),
            );
        }
        {
            let table_clone = Rc::clone(table_rc);
            table_rc.borrow_mut().set(
                LEFT,
                ScriptValue::function(LEFT, move |ctx, args| {
                    let direction = ShiftDirection::Left;
                    let distance = parse_required_arg(&args, 0, ScriptValue::as_int)?;
                    let duration = parse_duration(&args, 1, 0.2);
                    with_behaviour_mut_from_ctx_rc::<CharactersStage, _>(ctx, |b| {
                        b.export_character_offset(
                            &ctx,
                            &table_clone,
                            &direction,
                            distance,
                            duration,
                        );
                    })?;
                    Ok(ScriptValue::nil())
                }),
                Some(ctx),
            );
        }
        {
            let table_clone = Rc::clone(table_rc);
            table_rc.borrow_mut().set(
                RIGHT,
                ScriptValue::function(RIGHT, move |ctx, args| {
                    let direction = ShiftDirection::Right;
                    let distance = parse_required_arg(&args, 0, ScriptValue::as_int)?;
                    let duration = parse_duration(&args, 1, 0.2);
                    with_behaviour_mut_from_ctx_rc::<CharactersStage, _>(ctx, |b| {
                        b.export_character_offset(
                            &ctx,
                            &table_clone,
                            &direction,
                            distance,
                            duration,
                        );
                    })?;
                    Ok(ScriptValue::nil())
                }),
                Some(ctx),
            );
        }
        Ok(())
    }
}
