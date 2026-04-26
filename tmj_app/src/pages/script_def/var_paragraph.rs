use tmj_core::script::{ScriptValue, TypeName, lower_str};

use crate::pages::{
    pipeline::{ParagraphBehaviour, with_behaviour_mut_from_ctx},
    script_def::BaseVariable,
};

lower_str!(PARAGRAPH);
lower_str!(M_VISIBLE);
lower_str!(M_CONTENT);

// methods
lower_str!(SHOW);
lower_str!(PRINT);
lower_str!(NEW);
lower_str!(HIDE);
lower_str!(CLEAR);

#[derive(TypeName)]
pub struct VParagraph;

impl BaseVariable for VParagraph {
    fn regist_to_ctx_impl(ctx: &mut tmj_core::script::ScriptContext) -> anyhow::Result<()> {
        ctx.set_global_table(PARAGRAPH);
        let _ = ctx.set_table_member(PARAGRAPH, M_VISIBLE, ScriptValue::bool(false));
        let _ = ctx.set_table_member(PARAGRAPH, M_CONTENT, ScriptValue::string(""));
        {
            let _ = ctx
                .set_table_func(PARAGRAPH, SHOW, |ctx, _args| {
                    let paragraph = ctx
                        .borrow()
                        .get_global_val(PARAGRAPH)
                        .ok_or(anyhow::anyhow!("paragraph not found"))?
                        .as_table_or_resolve(ctx)
                        .ok_or(anyhow::anyhow!("paragraph is not table"))?;
                    paragraph
                        .borrow_mut()
                        .set(M_VISIBLE, ScriptValue::bool(true), None);
                    Ok(ScriptValue::Table(paragraph))
                })
                .map_err(|e| anyhow::anyhow!(e))?;
        }

        {
            let _ = ctx
                .set_table_func(PARAGRAPH, HIDE, |ctx, _args| {
                    let paragraph = ctx
                        .borrow()
                        .get_global_val(PARAGRAPH)
                        .ok_or(anyhow::anyhow!("paragraph not found"))?
                        .as_table_or_resolve(ctx)
                        .ok_or(anyhow::anyhow!("paragraph is not table"))?;
                    paragraph
                        .borrow_mut()
                        .set(M_VISIBLE, ScriptValue::bool(false), None);
                    Ok(ScriptValue::Table(paragraph))
                })
                .map_err(|e| anyhow::anyhow!(e))?;
        }

        {
            let _ = ctx
                .set_table_func(PARAGRAPH, PRINT, |ctx, args| {
                    let text = args
                        .first()
                        .and_then(|x| x.as_str())
                        .ok_or(anyhow::anyhow!("paragraph.print requires text argument"))?;
                    let paragraph = ctx
                        .borrow()
                        .get_global_val(PARAGRAPH)
                        .ok_or(anyhow::anyhow!("paragraph not found"))?
                        .as_table_or_resolve(ctx)
                        .ok_or(anyhow::anyhow!("paragraph is not table"))?;
                    let old_content = paragraph
                        .borrow()
                        .get(M_CONTENT, None)
                        .and_then(|x| x.as_str().map(|s| s.to_string()))
                        .unwrap_or_default();
                    with_behaviour_mut_from_ctx::<ParagraphBehaviour, _>(
                        ctx,
                        |b: &mut ParagraphBehaviour| {
                            b.export_print(&text.to_string());
                        },
                    );
                    let content = old_content + text;
                    paragraph
                        .borrow_mut()
                        .set(M_CONTENT, ScriptValue::string(content), None);
                    Ok(ScriptValue::Table(paragraph))
                })
                .map_err(|e| anyhow::anyhow!(e))?;
        }

        {
            let _ = ctx
                .set_table_func(PARAGRAPH, NEW, |ctx, args| {
                    let text = args
                        .first()
                        .and_then(|x| x.as_str())
                        .ok_or(anyhow::anyhow!("paragraph.new requires text argument"))?;
                    let paragraph = ctx
                        .borrow()
                        .get_global_val(PARAGRAPH)
                        .ok_or(anyhow::anyhow!("paragraph not found"))?
                        .as_table_or_resolve(ctx)
                        .ok_or(anyhow::anyhow!("paragraph is not table"))?;
                    {
                        let mut p = paragraph.borrow_mut();
                        // Clear current page immediately, then push next-frame content via once command.
                        p.set(M_CONTENT, ScriptValue::string(""), None);
                        p.set(M_VISIBLE, ScriptValue::bool(true), None);
                    }
                    with_behaviour_mut_from_ctx::<ParagraphBehaviour, _>(
                        ctx,
                        |b: &mut ParagraphBehaviour| {
                            b.export_new(&text.to_string());
                        },
                    );
                    paragraph
                        .borrow_mut()
                        .set(M_CONTENT, ScriptValue::string(text), None);
                    Ok(ScriptValue::Table(paragraph))
                })
                .map_err(|e| anyhow::anyhow!(e))?;
        }

        {
            let _ = ctx
                .set_table_func(PARAGRAPH, CLEAR, |ctx, _args| {
                    let paragraph = ctx
                        .borrow()
                        .get_global_val(PARAGRAPH)
                        .ok_or(anyhow::anyhow!("paragraph not found"))?
                        .as_table_or_resolve(ctx)
                        .ok_or(anyhow::anyhow!("paragraph is not table"))?;
                    paragraph
                        .borrow_mut()
                        .set(M_CONTENT, ScriptValue::string(""), None);
                    with_behaviour_mut_from_ctx::<ParagraphBehaviour, _>(
                        ctx,
                        |b: &mut ParagraphBehaviour| {
                            b.export_clear();
                        },
                    );
                    Ok(ScriptValue::Table(paragraph))
                })
                .map_err(|e| anyhow::anyhow!(e))?;
        }

        Ok(())
    }
}
