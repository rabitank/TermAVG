use ratatui::{layout::Rect, widgets::Wrap};
use std::collections::HashSet;
use std::ops::{Div, Mul};
use tmj_core::{
    script::{ScriptValue, TabelGet, TypeName},
};

use crate::{
    LAYOUT,
    pages::{
        pipeline::{PipeStage, logical_area},
        pipeline::ve_utils::clear_animations_by_prefix,
        pipeline::visual_element::{
            VisualElement, VisualElementKind,
        },
        script_def::{
            character::{self},
            env::CHARACTER_LS,
        },
    },
};

#[derive(TypeName)]
pub struct CharactersStage;

impl PipeStage for CharactersStage {
    fn binding_vars() -> &'static [&'static str] {
        &[CHARACTER_LS]
    }

}

fn read_character_entries(
    ctx: &tmj_core::script::ContextRef,
) -> anyhow::Result<Vec<(i64, ScriptValue)>> {
    let character_ls = CharactersStage::get_script_vars(&ctx).pop().unwrap()?;
    let character_ls = character_ls
        .as_table()
        .ok_or(anyhow::anyhow!("{} should be table", CHARACTER_LS))?;
    let mut characters: Vec<(i64, ScriptValue)> = character_ls
        .borrow_mut()
        .int_iter()
        .map(|i| (i.0.clone(), i.1.clone()))
        .collect();
    characters.sort_by_key(|i| i.0);
    Ok(characters)
}

fn character_spacing_spec(character_num: usize) -> u16 {
    match character_num {
        0 | 1 => 0,
        2 => LAYOUT.two_character_spec,
        _ => LAYOUT.x_character_spec,
    }
}

fn character_rect_at(idx: usize, character_num: usize, area: Rect) -> Rect {
    let spec = character_spacing_spec(character_num);
    let x_offset = (idx as f32 - (character_num as f32).div(2_f32))
        .mul(spec as f32 + LAYOUT.character_twh.1 as f32)
        + spec.div(2) as f32;
    let x = (area.width as f32).div(2_f32) + x_offset + area.x as f32;
    let y = LAYOUT.character_twh.0 + area.y;
    Rect {
        x: x.floor() as u16,
        y,
        width: LAYOUT.character_twh.1,
        height: LAYOUT.character_twh.2,
    }
    .clamp(area)
}

fn read_stand_image(c: &ScriptValue) -> anyhow::Result<Option<String>> {
    let c = match c.as_table() {
        Some(v) => v,
        None => return Ok(None),
    };
    if c.borrow().type_tag().unwrap_or_default() != "character" {
        return Ok(None);
    }
    let face = match c
        .get(character::FACE)
        .ok()
        .and_then(|v| v.as_str().map(str::to_string))
    {
        Some(v) => v,
        None => return Ok(None),
    };
    let current_stand_img = c.get(format!("{}.{}", character::_STANDS, face))?;
    Ok(current_stand_img.as_str().map(str::to_string))
}

fn make_character_element(ls_id: i64, rect: Rect, source: String) -> VisualElement {
    VisualElement {
        name: format!("character_{ls_id}"),
        z_index: 100 + ls_id as i32,
        rect,
        kind: VisualElementKind::Image { source },
        ..Default::default()
    }
}

impl CharactersStage {
    pub fn build_elements(
        ctx: &tmj_core::script::ContextRef,
    ) -> anyhow::Result<Vec<VisualElement>> {
        let area = logical_area();
        let characters = read_character_entries(ctx)?;
        let character_num = characters.len();
        let mut elements = Vec::new();
        for (idx, (ls_id, c)) in characters.into_iter().enumerate() {
            let c_rect = character_rect_at(idx, character_num, area);
            let current_stand_img = match read_stand_image(&c)? {
                Some(v) => v,
                None => continue,
            };
            elements.push(make_character_element(ls_id, c_rect, current_stand_img));
        }
        Ok(elements)
    }

    pub fn update_elements(
        ctx: &tmj_core::script::ContextRef,
        elements: &mut Vec<VisualElement>,
    ) -> anyhow::Result<()> {
        let area = logical_area();
        let characters = read_character_entries(ctx)?;
        let character_num = characters.len();
        if character_num == 0 {
            elements.retain(|ve| !ve.name.starts_with("character_"));
            return Ok(());
        }

        let mut desired = Vec::new();
        for (idx, (ls_id, c)) in characters.iter().enumerate() {
            let rect = character_rect_at(idx, character_num, area);
            let source = match read_stand_image(c)? {
                Some(v) => v,
                None => continue,
            };
            desired.push((*ls_id, rect, source));
        }

        let desired_names: HashSet<String> = desired
            .iter()
            .map(|(ls_id, _, _)| format!("character_{ls_id}"))
            .collect();

        elements.retain(|ve| !ve.name.starts_with("character_") || desired_names.contains(&ve.name));

        for (ls_id, rect, source) in desired {
            let ve_name = format!("character_{ls_id}");
            if let Some(ve) = elements.iter_mut().find(|x| x.name == ve_name) {
                if ve.is_animated {
                    continue;
                }
                ve.visible = true;
                ve.rect = rect;
                ve.z_index = 100 + ls_id as i32;
                if let VisualElementKind::Image { source: current } = &mut ve.kind {
                    *current = source;
                } else {
                    ve.kind = VisualElementKind::Image { source };
                }
            } else {
                elements.push(make_character_element(ls_id, rect, source));
            }
        }
        Ok(())
    }

    pub fn stage_clear(
        _ctx: &tmj_core::script::ContextRef,
        elements: &mut [VisualElement],
        _area: ratatui::prelude::Rect,
    ) -> anyhow::Result<()> {
        clear_animations_by_prefix(elements, "character_");
        Ok(())
    }
}
