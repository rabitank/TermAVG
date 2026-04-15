pub mod character_ls;
pub use character_ls::CharactersStage;
pub mod background;
pub use background::BackgrondStage;
pub mod dialogue_frame;
pub use dialogue_frame::DialogueFrameStage;
pub mod paragraph;
pub use paragraph::ParagraphStage;
pub mod face;
pub use face::FaceStage;
pub mod layers;
pub use layers::LayersStage;
pub mod typewriter;


use ratatui::{buffer::Buffer, layout::Rect};
use tmj_core::script::{ContextRef, ScriptValue};


/// 这里是管线
/// 根据脚本环境中的预设变量, 获取这些变量并且进行相应处理

pub trait PipeStage {
    // 这里写需要的变量路径
    fn binding_vars() -> &'static [&'static str];

    // 这是直接获取脚本变量的默认实现, 算是一个辅助函数
    fn get_script_vars(ctx: &ContextRef) -> Vec<anyhow::Result<ScriptValue>> {
        let vars = Self::binding_vars();
        let vars = {
            let ct = ctx.borrow();
            let res: Vec<Result<ScriptValue, anyhow::Error>> = vars.iter().map(|s| {
                let var = ct.resolve_path(s).map_err(|e| anyhow::anyhow!(e));
                var
            }).collect();
            res
        };
        vars
    }

    /// 这里接受上一stage绘制结果然后绘制, 是最终接口
    fn draw<'a>(screen: &crate::pages::dialogue::DialogueScene, ctx: &ContextRef, buffer: &'a mut Buffer, area: Rect) -> anyhow::Result<&'a mut Buffer>;
}
