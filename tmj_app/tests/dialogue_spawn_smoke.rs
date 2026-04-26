//! Smoke: same init sequence as DialogueScene::spawn (without script reader).

use std::cell::RefCell;
use std::rc::Rc;
use tmj_app::pages::pipeline::{BehaviourMap, default_dialogue_ve_stages};
use tmj_app::pages::script_def::init_env;
use tmj_core::script::ScriptContext;

#[test]
fn init_env_then_rebuild_tuid_table() {
    let ctx = Rc::new(RefCell::new(ScriptContext::new()));
    ctx.borrow_mut().bind_context_ref(ctx.clone());
    let behaviours_map = BehaviourMap {
        behaviours: Rc::new(RefCell::new(default_dialogue_ve_stages())),
    };
    init_env(ctx.clone(), behaviours_map);
    let err = ctx.borrow_mut().rebuild_tuid_table_from_live();
    assert!(
        err.is_ok(),
        "rebuild_tuid_table_from_live failed: {:?}",
        err.err()
    );
}
