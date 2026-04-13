use ratatui::layout::Rect;
use ratatui::widgets::Widget;
use std::ops::{Div, Mul};
use tmj_core::{
    img::shape::Pic,
    pathes,
    script::{ScriptValue, Table, TypeName},
};

use crate::{
    SETTING,
    pages::{
        pipeline::PipeStage,
        script_def::{character::GET_CURRENT_STAND, env::CHARACTER_LS},
    },
};

#[derive(TypeName)]
pub struct CharactersStage;

impl PipeStage for CharactersStage {
    fn binding_vars() -> &'static [&'static str] {
        &[CHARACTER_LS]
    }

    fn draw<'a>(
        _screen: &crate::pages::dialogue::DialogueScene,
        ctx: &tmj_core::script::ContextRef,
        buffer: &'a mut ratatui::prelude::Buffer,
        area: ratatui::prelude::Rect,
    ) -> anyhow::Result<&'a mut ratatui::prelude::Buffer> {
        let character_ls = Self::get_script_vars(&ctx).pop().unwrap()?;
        let character_ls = character_ls
            .as_table()
            .ok_or(anyhow::anyhow!("{} should be table", CHARACTER_LS))?;
        let mut characters: Vec<(i64, ScriptValue)> = character_ls
            .borrow_mut()
            .int_iter()
            .map(|i| (i.0.clone(), i.1.clone()))
            .collect();
        characters.sort_by_key(|i| i.0);

        let character_num = characters.len();
        if character_num == 0 {
            return Ok(buffer);
        }

        let spec = match character_num {
            1 => 0,
            2 => SETTING.layout.two_character_spec,
            _ => SETTING.layout.x_character_spec,
        };

        for (idx, (ls_id, c)) in characters.iter().enumerate() {
            let x_offset = (idx as f32 - (character_num as f32).div(2_f32))
                .mul(spec as f32 + SETTING.layout.character_size.0 as f32)
                + spec.div(2) as f32;
            let x = (SETTING.resolution.0 as f32).div(2_f32) + x_offset + area.x as f32;
            let y = SETTING.layout.character_up_edge as u16 + area.y; 
            let c_rect = Rect {
                x: x.floor() as u16,
                y: y,
                width: SETTING.layout.character_size.0 as u16,
                height: SETTING.layout.character_size.1 as u16,
            };
            let c_rect = c_rect.clamp(area);
            let c = c.as_table().ok_or(anyhow::anyhow!(
                "pos {} ls {} is not table in character_ls",
                idx,
                ls_id
            ))?;

            if c.borrow().type_tag().unwrap() != "character" {
                tracing::error!("{:?} is not a character ins", c);
                continue;
            }
            let current_stand_img =
                Table::call_method(&c, &GET_CURRENT_STAND, &ctx, vec![]).unwrap();

            let current_stand_img = current_stand_img.as_str().unwrap();
            let pic = Pic::from(pathes::path(current_stand_img))?;
            pic.render(c_rect, buffer);
        }

        Ok(buffer)
    }
}
