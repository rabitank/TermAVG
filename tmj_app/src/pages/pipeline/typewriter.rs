use std::{cell::RefCell, rc::Rc};

use tmj_core::script::{ScriptValue, Table};

const TYPEWRITER_ENABLE: &str = "typewriter_enable";
const TYPEWRITER_SPEED: &str = "typewriter_speed";
const TYPEWRITER_PROGRESS: &str = "typewriter_progress";
const TYPEWRITER_LAST_CONTENT: &str = "_typewriter_last_content";

pub fn typewriter_render_text(
    state_table: &Rc<RefCell<Table>>,
    full_text: &str,
    delta_secs: f64,
    default_enable: bool,
    default_speed: f64,
) -> String {
    let total_chars = full_text.chars().count() as f64;
    let mut table = state_table.borrow_mut();

    let enabled = table
        .get(TYPEWRITER_ENABLE)
        .and_then(|v| v.as_bool())
        .unwrap_or(default_enable);
    let speed = table
        .get(TYPEWRITER_SPEED)
        .and_then(|v| v.to_number())
        .unwrap_or(default_speed)
        .max(0.0);
    let mut progress = table
        .get(TYPEWRITER_PROGRESS)
        .and_then(|v| v.to_number())
        .unwrap_or(0.0);
    let last_content = table
        .get(TYPEWRITER_LAST_CONTENT)
        .and_then(|v| v.as_str().map(|s| s.to_string()))
        .unwrap_or_default();

    if last_content != full_text {
        progress = 0.0;
        table.set(
            TYPEWRITER_LAST_CONTENT,
            ScriptValue::string(full_text.to_string()),
        );
    }

    if enabled {
        progress = (progress + speed * delta_secs.max(0.0)).min(total_chars);
    } else {
        progress = total_chars;
    }
    table.set(TYPEWRITER_PROGRESS, ScriptValue::float(progress));

    let visible_chars = progress.floor() as usize;
    full_text.chars().take(visible_chars).collect()
}
