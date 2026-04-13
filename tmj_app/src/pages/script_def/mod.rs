pub mod env;
pub use env::init_env;

pub mod character;
pub use character::Character;

pub mod text_obj;
pub use text_obj::TextObj;
use tmj_core::script::{ScriptContext, TypeName};

pub mod var_character_ls;
pub use var_character_ls::VCharacterLs;

pub mod var_frame;
pub use var_frame::VFrame;


pub trait BaseVariable: TypeName {
    fn regist_to_ctx_impl(ctx: &mut ScriptContext) -> anyhow::Result<()>;

    fn regist_to_ctx(ctx: &mut ScriptContext) -> anyhow::Result<()> {
        match Self::regist_to_ctx_impl(ctx) {
            Err(e) => {
                tracing::error!(
                    "failed because {e} when regist script base variable: {}",
                    Self::TYPE_NAME
                );
                Result::Err(e)
            }
            Ok(res) => {
                tracing::info!("script base var {} regist success", Self::TYPE_NAME);
                Ok(res)}
        }
    }
}
