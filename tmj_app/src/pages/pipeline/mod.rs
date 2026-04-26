pub mod animation;
pub mod visual_element;


mod behaviour;
pub use behaviour::Behaviour;
pub use behaviour::BehaviourMap;
pub use behaviour::with_behaviour_mut_from_ctx;


pub mod character_ls;
pub use character_ls::CharactersStage;

pub mod background;
pub use background::BackgroundBehaviour;
pub mod dialogue_frame;
pub use dialogue_frame::FrameBehaviour;

pub mod paragraph;
pub use paragraph::ParagraphBehaviour;
pub mod layers;
pub use layers::LayerBehaviour;
pub mod render_ve;
pub use render_ve::RenderVeStage;
pub mod chapter;
pub use chapter::ChapterBehaviour;

use std::collections::HashMap;

use ratatui::layout::Rect;
use tmj_core::script::TypeName;


use crate::SETTING;

pub fn logical_area() -> Rect {
    let (w, h) = SETTING.resolution;
    Rect::new(0, 0, w, h)
}

/// 与 `#[derive(TypeName)]` 默认一致（类型名小写）。`HashMap` 无稳定顺序，build/update 须按
/// `DIALOGUE_VE_STAGE_ORDER`。
pub const DIALOGUE_VE_STAGE_ORDER: &[&str] = &[
    BackgroundBehaviour::TYPE_NAME,
    FrameBehaviour::TYPE_NAME,
    ParagraphBehaviour::TYPE_NAME,
    CharactersStage::TYPE_NAME,
    LayerBehaviour::TYPE_NAME,
    ChapterBehaviour::TYPE_NAME,
];

pub fn default_dialogue_ve_stages() -> HashMap<String, Box<dyn Behaviour>> {

    let mut m: HashMap<String, Box<dyn Behaviour>> = HashMap::new();
    m.insert(BackgroundBehaviour::TYPE_NAME.to_string(), Box::new(BackgroundBehaviour::default()));
    m.insert(
        FrameBehaviour::TYPE_NAME.to_string(),
        Box::new(FrameBehaviour::default()),
    );

    m.insert(ParagraphBehaviour::TYPE_NAME.to_string(), Box::new(ParagraphBehaviour::default()));
    m.insert(CharactersStage::TYPE_NAME.to_string(), Box::new(CharactersStage::default()));
    m.insert(LayerBehaviour::TYPE_NAME.to_string(), Box::new(LayerBehaviour::default()));
    m.insert(ChapterBehaviour::TYPE_NAME.to_string(), Box::new(ChapterBehaviour::default()));
    m
}
