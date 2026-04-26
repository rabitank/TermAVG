use tmj_core::script::{FromCommand, IntoScriptValue, IntoTable, ScriptContext, Table, TypeName, lower_str};

lower_str!(CONTENT);
lower_str!(XPOS);
lower_str!(YPOS);


#[derive(Default)]
#[derive(TypeName)]
pub struct TextObj {
    pub content: String,
    pub pos: (i32, i32),
}


impl IntoTable for TextObj {
    fn into_data_table(self, ctx: &mut tmj_core::script::ScriptContext) -> tmj_core::script::Table {
        let tuid = ctx.alloc_table_id();
        let mut table = Table::with_tuid(tuid);
        table.set(CONTENT, self.content.into_script_val(), None);
        table.set(XPOS, self.pos.0.into_script_val(), None);
        table.set(YPOS, self.pos.1.into_script_val(), None);
        table
    }
}

impl FromCommand for TextObj {
    fn from_script_command(_ctx: &mut ScriptContext, args: Vec<tmj_core::script::ScriptValue>) -> Result<Self, String>
        where
            Self: Sized {
                let len = args.len();
                if len == 3 && args[0].is_string() && args[1].is_int() && args[2].is_int(){
                    let content = args[0].as_string().unwrap().clone();
                    return Ok(TextObj {
                        content,
                        ..Self::default()
                    })
                } else {
                    return Err("wrong args".to_string());
                }
    }
}

