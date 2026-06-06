use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::ops::{Div, Mul};
use std::time::Duration;

use ratatui::layout::Rect;
use tmj_core::script::{ContextRef, TableRef};
use tmj_core::script::{ScriptValue, TabelGet, TypeName};

use crate::pages::behaviour::Behaviour;
use crate::pages::behaviour::animation::img_trans::AniImgTrans;
use crate::pages::behaviour::animation::offset_shift::ShiftDirection;
use crate::{
    LAYOUT,
    pages::{
        behaviour::{
            animation::{
                OffsetShift, VeAniMap, VeTypedAnimationMap,
                alpha_shift::AniAlpha,
                rect_trans::{AniRectTrans, RectTransCurve},
            },
            logical_area,
            ve_z_index::Z_CHARACTER_BASE,
            visual_element::{VisualElement, VisualElementKind},
        },
        dialogue::DialogueScene,
        script_def::{
            character::{self},
            env::CHARACTER_LS,
        },
    },
};

/// 入场时自右向左滑入的像素距离。
pub const CHARACTER_FADE_IN_SLIDE_PX: u16 = 8;

/// character_ls 矩形布局/滑入动画的缓动（由快到慢）。
const CHARACTER_LAYOUT_CURVE: RectTransCurve = RectTransCurve::PowerOut { exponent: 2.0 };

#[derive(TypeName, Default)]
pub struct CharactersStage {
    character_ves_anim_map: RefCell<VeTypedAnimationMap>,
    /// `fade_in` / `fade_out` 后下一帧为其余角色补布局位移动画。
    pending_layout_duration: RefCell<Option<Duration>>,
    // 一次session中,不参与布局补位机制的character
    layout_skip_ves: RefCell<HashSet<String>>,
}

impl CharactersStage {
    /// 将角色加入 `character_ls`（已存在则复用槽位），并从右侧 8 像素滑入、alpha 0→1 进入布局位置。
    pub fn export_fade_in(
        &mut self,
        ctx: &ContextRef,
        character: &TableRef,
        duration: Duration,
    ) -> anyhow::Result<i64> {
        let ls_id = find_or_add_character_in_ls(ctx, character)?;
        let ve_name = format!("character_{ls_id}");
        let duration_secs = duration.as_secs_f64();
        let slide_px = i32::from(CHARACTER_FADE_IN_SLIDE_PX);

        let mut anim_map = self.character_ves_anim_map.borrow_mut();
        anim_map.insert_ani(&ve_name, {
            let mut offset_ani = OffsetShift::default();
            offset_ani.begin((slide_px, 0), (0, 0), duration_secs, CHARACTER_LAYOUT_CURVE);
            offset_ani
        })?;
        anim_map.insert_ani(&ve_name, AniAlpha::new(0.0, 1.0, duration))?;
        drop(anim_map);

        *self.pending_layout_duration.borrow_mut() = Some(duration);
        self.layout_skip_ves.borrow_mut().insert(ve_name);
        Ok(ls_id)
    }

    pub fn export_to_face(
        &mut self,
        ctx: &ContextRef,
        character: &TableRef,
        old_stand_path: &String,
        new_stand_path: &String,
        duration: Duration,
    ) -> anyhow::Result<()> {
        let ls_id = find_character_ls_id(ctx, character)?;
        let ve_name = format!("character_{ls_id}");
        self.character_ves_anim_map.borrow_mut().insert_ani(
            &ve_name,
            AniImgTrans {
                default_img_cover: tmj_core::img::halfblock::cover_halfblock,
                anim_time: duration,
                old_image: Some(old_stand_path.into()),
                new_image: Some(new_stand_path.into()),
                run_time: Duration::ZERO,
            },
        )
    }

    pub fn export_character_offset(
        &mut self,
        ctx: &ContextRef,
        character: &TableRef,
        direction: &ShiftDirection,
        distance: i64,
        duration: Duration,
    ) -> anyhow::Result<()> {
        let ls_id = find_character_ls_id(ctx, character)?;
        let ve_name = format!("character_{ls_id}");
        let duration_secs = duration.as_secs_f64();
        let mut anim_mp = self.character_ves_anim_map.borrow_mut();

        let ani = anim_mp.get_ani_mut::<OffsetShift>(&ve_name);
        match ani {
            Ok(offset_ani) => {
                let target_offset = direction.apply(offset_ani.target_offset, distance);
                tracing::info!("{ve_name} has pre offset ani, begin from {:?} to {:?}", offset_ani.target_offset, target_offset);
                offset_ani.begin(
                    offset_ani.target_offset,
                    target_offset,
                    duration_secs,
                    CHARACTER_LAYOUT_CURVE,
                );
            }
            Err(_) => {
                anim_mp.insert_ani(&ve_name, {
                    tracing::info!("{ve_name} no pre offset ani, begin from 0,0");
                    let target_offset = direction.apply((0, 0), distance);
                    let mut offset_ani = OffsetShift::default();
                    offset_ani.begin(
                        offset_ani.target_offset,
                        target_offset,
                        duration_secs,
                        CHARACTER_LAYOUT_CURVE,
                    );
                    offset_ani
                })?;
            }
        };
        Ok(())
    }

    /// 从 `character_ls` 移除角色，仅 alpha 1→0 淡出；其余角色平滑移动到新布局位。
    pub fn export_fade_out(
        &mut self,
        ctx: &ContextRef,
        character: &TableRef,
        duration: Duration,
    ) -> anyhow::Result<()> {
        let ls_id = find_character_ls_id(ctx, character)?;
        let ve_name = format!("character_{ls_id}");
        remove_character_from_ls(ctx, ls_id)?;

        self.character_ves_anim_map
            .borrow_mut()
            .insert_ani(&ve_name, AniAlpha::new(1.0, 0.0, duration))?;

        *self.pending_layout_duration.borrow_mut() = Some(duration);
        self.layout_skip_ves.borrow_mut().insert(ve_name);
        Ok(())
    }

    /// 清空 `character_ls`，并为当前场上角色 VE 挂上 alpha 归零动画。
    pub fn export_clear(&mut self, ctx: &ContextRef, duration: Duration) -> anyhow::Result<()> {
        let characters = read_character_entries(ctx)?;
        let mut anim_map = self.character_ves_anim_map.borrow_mut();
        for (ls_id, _) in &characters {
            let ve_name = format!("character_{ls_id}");
            anim_map.insert_ani(&ve_name, AniAlpha::new(1.0, 0.0, duration))?;
        }
        let character_ls = CharactersStage::default()
            .get_bind_vars(ctx)
            .pop()
            .unwrap()?
            .as_table_or_resolve(ctx)
            .ok_or(anyhow::anyhow!("{} should be table", CHARACTER_LS))?;
        character_ls.borrow_mut().clear_int_keys();
        Ok(())
    }

    fn schedule_layout_rect_moves(
        &self,
        elements: &[VisualElement],
        desired: &[(i64, Rect, String)],
        duration: Duration,
        skip: &HashSet<String>,
    ) -> anyhow::Result<()> {
        let duration_secs = duration.as_secs_f64();
        let mut anim_map = self.character_ves_anim_map.borrow_mut();
        for (ls_id, target_rect, _) in desired {
            let ve_name = format!("character_{ls_id}");
            if skip.contains(&ve_name) {
                continue;
            }
            let Some(ve) = elements.iter().find(|v| v.name == ve_name) else {
                continue;
            };
            if ve.rect == *target_rect {
                continue;
            }
            let mut rect_ani = AniRectTrans::default();
            rect_ani.begin(ve.rect, *target_rect, duration_secs, CHARACTER_LAYOUT_CURVE);
            anim_map.insert_ani(&ve_name, rect_ani)?;
        }
        Ok(())
    }

    fn apply_character_anims(&self, ve: &mut VisualElement) -> anyhow::Result<()> {
        if let Some(anims) = self.character_ves_anim_map.borrow().get(&ve.name) {
            for (_, anim) in anims.iter() {
                anim.apply_to_ve(ve)?;
            }
        }
        Ok(())
    }

    fn character_ve_has_active_anims(&self, ve_name: &str) -> bool {
        self.character_ves_anim_map
            .borrow()
            .get(ve_name)
            .is_some_and(|anims| anims.values().any(|anim| anim.is_animing()))
    }
}

impl Behaviour for CharactersStage {
    fn binding_vars(&self) -> &'static [&'static str] {
        &[CHARACTER_LS]
    }

    fn build_elements(
        &self,
        ctx: &tmj_core::script::ContextRef,
    ) -> anyhow::Result<Vec<VisualElement>> {
        let area = logical_area();
        let characters = read_character_entries(ctx)?;
        let character_num = characters.len();
        let mut elements = Vec::new();
        for (idx, (ls_id, c)) in characters.into_iter().enumerate() {
            let c_rect = character_rect_at(idx, character_num, area);
            let current_stand_img = match read_stand_image(ctx, &c)? {
                Some(v) => v,
                None => continue,
            };
            elements.push(make_character_element(ls_id, c_rect, current_stand_img));
        }
        Ok(elements)
    }

    fn update_elements(
        &self,
        _screen: &DialogueScene,
        ctx: &tmj_core::script::ContextRef,
        elements: &mut Vec<VisualElement>,
    ) -> anyhow::Result<()> {
        let area = logical_area();
        let characters = read_character_entries(ctx)?;
        let character_num = characters.len();

        let mut desired = Vec::new();
        for (idx, (ls_id, c)) in characters.iter().enumerate() {
            let rect = character_rect_at(idx, character_num, area);
            let source = match read_stand_image(ctx, c)? {
                Some(v) => v,
                None => continue,
            };
            desired.push((*ls_id, rect, source));
        }

        let desired_names: HashSet<String> = desired
            .iter()
            .map(|(ls_id, _, _)| format!("character_{ls_id}"))
            .collect();

        elements.retain(|ve| {
            !ve.name.starts_with("character_")
                || desired_names.contains(&ve.name)
                || self.character_ve_has_active_anims(&ve.name)
        });
        // take拿走后,当前实现里其他角色的布局补位动画只会触发一次
        if let Some(dur) = self.pending_layout_duration.borrow_mut().take() {
            let skip = std::mem::take(&mut *self.layout_skip_ves.borrow_mut());
            self.schedule_layout_rect_moves(elements, &desired, dur, &skip)?;
        }

        for ve in elements
            .iter_mut()
            .filter(|ve| ve.name.starts_with("character_"))
        {
            self.apply_character_anims(ve)?;
        }

        if character_num == 0 {
            return Ok(());
        }

        for (ls_id, rect, source) in desired {
            let ve_name = format!("character_{ls_id}");
            if let Some(ve) = elements.iter_mut().find(|x| x.name == ve_name) {
                ve.z_index = Z_CHARACTER_BASE + ls_id as i32;
                if !self.character_ve_has_active_anims(&ve_name) {
                    ve.rect = rect;
                    if let VisualElementKind::Image {
                        source: current, ..
                    } = &mut ve.kind {
                        *current = source;
                    }
                }
                self.apply_character_anims(ve)?;
            } else {
                let map = self.character_ves_anim_map.borrow();
                let entrance = map.get(&ve_name).is_some_and(|anims| {
                    anims.contains_key(OffsetShift::TYPE_NAME)
                        && anims.contains_key(AniAlpha::TYPE_NAME)
                });
                let slide_px = i32::from(CHARACTER_FADE_IN_SLIDE_PX);
                let (spawn_offset, spawn_alpha) = if entrance {
                    ((slide_px, 0), 0.0)
                } else {
                    ((0, 0), 1.0)
                };
                let mut ve = make_character_element(ls_id, rect, source);
                ve.offset = spawn_offset;
                ve.alpha = spawn_alpha;
                self.apply_character_anims(&mut ve)?;
                elements.push(ve);
            }
        }
        Ok(())
    }

    fn tick_update(&mut self, _ctx: ContextRef, delta_time: Duration) {
        for (_, anims) in self.character_ves_anim_map.borrow_mut().iter_mut() {
            for (_, anim) in anims.iter_mut() {
                anim.update(delta_time);
            }
        }
    }

    fn is_animating(&self) -> bool {
        self.character_ves_anim_map
            .borrow()
            .values()
            .flat_map(|anims| anims.values())
            .any(|anim| anim.is_animing())
    }

    fn on_force_over_animation(&mut self) -> anyhow::Result<()> {
        for (_, anims) in self.character_ves_anim_map.borrow_mut().iter_mut() {
            for (_, anim) in anims.iter_mut() {
                anim.force_over();
            }
        }
        *self.pending_layout_duration.borrow_mut() = None;
        self.layout_skip_ves.borrow_mut().clear();
        Ok(())
    }

    fn on_end_dialouge(&mut self) -> anyhow::Result<()> {
        self.character_ves_anim_map.borrow_mut().clear();
        *self.pending_layout_duration.borrow_mut() = None;
        self.layout_skip_ves.borrow_mut().clear();
        Ok(())
    }

    fn on_end_session(&mut self, _ctx: tmj_core::script::ContextRef) -> anyhow::Result<()> {
        for (_, anims) in self.character_ves_anim_map.borrow_mut().iter_mut() {
            anims.retain(|_, anim| anim.is_indeterminate() || anim.is_animing());
        }
        Ok(())
    }
}

fn find_character_ls_id(ctx: &ContextRef, character: &TableRef) -> anyhow::Result<i64> {
    let tuid = character.borrow().tuid;
    let character_ls = CharactersStage::default()
        .get_bind_vars(ctx)
        .pop()
        .unwrap()?
        .as_table_or_resolve(ctx)
        .ok_or(anyhow::anyhow!("{} should be table", CHARACTER_LS))?;

    for (id, val) in character_ls.borrow().int_iter() {
        if let Some(tbl) = val.as_table_or_resolve(ctx) {
            if tbl.borrow().tuid == tuid {
                return Ok(*id);
            }
        }
    }
    Err(anyhow::anyhow!("character is not on stage"))
}

fn remove_character_from_ls(ctx: &ContextRef, ls_id: i64) -> anyhow::Result<()> {
    let character_ls = CharactersStage::default()
        .get_bind_vars(ctx)
        .pop()
        .unwrap()?
        .as_table_or_resolve(ctx)
        .ok_or(anyhow::anyhow!("{} should be table", CHARACTER_LS))?;
    character_ls.borrow_mut().remove_int(ls_id);
    Ok(())
}

fn find_or_add_character_in_ls(ctx: &ContextRef, character: &TableRef) -> anyhow::Result<i64> {
    if let Ok(id) = find_character_ls_id(ctx, character) {
        return Ok(id);
    }
    let tuid = character.borrow().tuid;
    let character_ls = CharactersStage::default()
        .get_bind_vars(ctx)
        .pop()
        .unwrap()?
        .as_table_or_resolve(ctx)
        .ok_or(anyhow::anyhow!("{} should be table", CHARACTER_LS))?;
    let new_id = character_ls.borrow().len() as i64;
    character_ls
        .borrow_mut()
        .set_int(new_id, ScriptValue::table_handle(tuid));
    Ok(new_id)
}

fn read_character_entries(
    ctx: &tmj_core::script::ContextRef,
) -> anyhow::Result<Vec<(i64, ScriptValue)>> {
    let character_ls = CharactersStage::default()
        .get_bind_vars(ctx)
        .pop()
        .unwrap()?;
    let character_ls = character_ls
        .as_table_or_resolve(ctx)
        .ok_or(anyhow::anyhow!("{} should be table", CHARACTER_LS))?;
    let mut characters: Vec<(i64, ScriptValue)> = character_ls
        .borrow_mut()
        .int_iter()
        .map(|i| (i.0.clone(), i.1.clone()))
        .collect();
    characters.sort_by_key(|i| i.0);
    Ok(characters)
}

fn character_rect_at(idx: usize, character_num: usize, area: Rect) -> Rect {
    let spec = match character_num {
        0 | 1 => 0,
        2 => LAYOUT.two_character_spec,
        _ => LAYOUT.x_character_spec,
    };
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

fn read_stand_image(ctx: &ContextRef, c: &ScriptValue) -> anyhow::Result<Option<String>> {
    let c = match c.as_table_or_resolve(ctx) {
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
        z_index: Z_CHARACTER_BASE + ls_id as i32,
        rect,
        kind: VisualElementKind::Image {
            source,
            cover_method: tmj_core::img::halfblock::cover_halfblock,
        },
        ..Default::default()
    }
}
