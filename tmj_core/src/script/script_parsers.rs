use tracing::info;
use super::lexer::Lexer;
use super::parser::Parser;
use crate::script::{Command, ContextRef};

/// 脚本解析器 - 将文本解析为命令
pub struct ScriptParser;

impl ScriptParser {
    /// 解析脚本文件内容
    pub fn parse(input: &str) -> Result<Vec<Vec<Command>>, String> {
        info!("ScriptParser: parsing {} bytes", input.len());

        // 词法分析
        let mut lexer = Lexer::new(input);
        let tokens = lexer.tokenize()?;
        info!("ScriptParser: tokenized into {} tokens", tokens.len());

        // 语法分析
        let mut parser = Parser::new(tokens);
        let sessions = parser.parse()?;
        info!("ScriptParser: parsed {} sessions", sessions.len());

        Ok(sessions)
    }

    /// 解析单个 session
    pub fn parse_session(input: &str) -> Result<Vec<Command>, String> {
        let sessions = Self::parse(input)?;
        Ok(sessions.into_iter().next().unwrap_or_default())
    }
}
