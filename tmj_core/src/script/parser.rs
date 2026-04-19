use super::lexer::Token;
use crate::script::{Command, ScriptValue, WaitCondition};

/// 语法分析器
pub struct Parser {
    tokens: Vec<Token>,
    position: usize,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Parser {
            tokens,
            position: 0,
        }
    }

    /// 解析整个脚本，返回多个 session 的命令
    pub fn parse(&mut self) -> Result<Vec<Vec<Command>>, String> {
        let mut sessions = Vec::new();
        let mut current_session = Vec::new();

        while !self.is_at_end() {
            // 跳过空行
            while self.check(&Token::Newline) {
                self.advance();
            }

            if self.is_at_end() {
                break;
            }

            // 检查 session 分隔符 (#number)
            if self.check(&Token::Session)
                && self.peek().map_or(false, |t| matches!(t, Token::Int(_)))
            {
                if !current_session.is_empty() {
                    sessions.push(current_session);
                    current_session = Vec::new();
                }
                self.advance(); // 跳过 #
                self.advance(); // 跳过数字
                while self.check(&Token::Newline) {
                    self.advance();
                }
                continue;
            }

            // 解析命令
            match self.parse_command() {
                Ok(cmd) => {
                    current_session.push(cmd);
                }
                Err(e) => return Err(format!("Parse error at position {}: {}", self.position, e)),
            }

            // 跳过命令分隔符
            while self.check(&Token::Newline) {
                self.advance();
            }
        }

        if !current_session.is_empty() {
            sessions.push(current_session);
        }

        Ok(sessions)
    }

    /// 解析单个命令
    fn parse_command(&mut self) -> Result<Command, String> {
        // 检查是否是赋值命令：ident = value
        if self.check(&Token::Ident("".to_string())) {
            let first = self.current().clone();
            self.advance();

            if self.check(&Token::Equal) {
                self.advance(); // 跳过 =

                // 获取变量名
                let var_name = match first {
                    Token::Ident(name) => name,
                    _ => return Err("Expected identifier before '='".to_string()),
                };

                // 解析右边的内容
                // 可能是值，也可能是命令调用
                return self.parse_assignment(var_name);
            } else {
                // 回退，作为调用命令
                self.position -= 1;
                return self.parse_call_or_set();
            }
        }

        self.parse_call_or_set()
    }

    /// 解析赋值命令 (值或命令调用)
    fn parse_assignment(&mut self, var_name: String) -> Result<Command, String> {
        // 跳过空白和换行
        while self.check(&Token::Newline) {
            self.advance();
        }

        // 检查右边是什么
        match self.current() {
            // 字符串、数字、布尔值 → 简单赋值
            Token::String(_) | Token::Int(_) | Token::Float(_) => {
                let value = self.parse_value()?;
                Ok(Command::Assignment {
                    name: var_name,
                    value,
                })
            }

            // 标识符 → 可能是命令调用或路径引用
            Token::Ident(_) => {
                // 解析路径/命令
                let path = self.parse_path()?;

                // 解析参数
                let args = self.parse_args()?;

                // 如果有参数，说明是命令调用
                // 如果没有参数，可能是路径引用 (也当作命令调用，返回对象)
                Ok(Command::CommandAssignment {
                    name: var_name,
                    command: path,
                    args,
                })
            }

            // 其他情况，尝试解析为值
            _ => {
                let value = self.parse_value()?;
                Ok(Command::Assignment {
                    name: var_name,
                    value,
                })
            }
        }
    }

    /// 解析调用或 set 命令
    fn parse_call_or_set(&mut self) -> Result<Command, String> {
        // 解析路径 (如 dialogue_frame.show)
        let path = self.parse_path()?;

        // 检查是否是 wait 命令
        if path == "wait" {
            let condition = self.parse_wait_condition()?;
            return Ok(Command::Wait { condition });
        }

        // 检查是否是 once 命令
        if path == "once" {
            let args = self.parse_args()?;
            if args.is_empty() {
                return Err("once requires at least one argument".to_string());
            }
            // once 的第一个参数是路径
            let target_path = match &args[0] {
                ScriptValue::Expression(s) => s.clone(),
                _ => return Err("once first argument must be path expression".to_string()),
            };
            let rest_args = args[1..].to_vec();
            return Ok(Command::Once {
                path: target_path,
                args: rest_args,
            });
        }

        // 检查是否是 set 命令
        if path == "set" {
            let args = self.parse_args()?;
            if args.is_empty() {
                return Err("set requires at least one argument".to_string());
            }
            // set 的第一个参数是路径
            let target_path = match &args[0] {
                ScriptValue::Expression(s) => s.clone(),
                _ => return Err("set first argument {} must be expression".to_string()),
            };
            let rest_args = args[1..].to_vec();
            return Ok(Command::Set {
                path: target_path,
                args: rest_args,
            });
        }

        // 解析参数
        let args = self.parse_args()?;

        // 检查是否有链式调用
        if self.check(&Token::Arrow) {
            return self.parse_chain(path, args);
        }

        // 普通调用命令
        Ok(Command::Call { path, args })
    }

    /// 解析链式调用
    fn parse_chain(
        &mut self,
        first_path: String,
        first_args: Vec<ScriptValue>,
    ) -> Result<Command, String> {
        let mut commands = Vec::new();

        // 第一个命令
        commands.push(Command::Call {
            path: first_path,
            args: first_args,
        });

        // 解析后续链式命令
        while self.check(&Token::Arrow) {
            self.advance(); // 跳过 ->

            // 链式命令可能以 . 开头 (方法调用)
            let path = if self.check(&Token::Dot) {
                // .method 形式，需要结合前一个命令的返回值
                // 简化：直接解析为路径
                self.parse_path()?
            } else {
                self.parse_path()?
            };

            let args = self.parse_args()?;
            commands.push(Command::Call { path, args });
        }

        Ok(Command::Chain { commands })
    }

    /// 解析路径 (如 ef.SnowEffect.begin_snow)
    fn parse_path(&mut self) -> Result<String, String> {
        let mut parts = Vec::<String>::new();

        // 可能以 . 开头 (链式调用的方法)
        let starts_with_dot = self.check(&Token::Dot);
        if starts_with_dot {
            self.advance();
        }

        // 第一部分 (标识符)
        if let Token::Ident(ident) = self.current() {
            parts.push(ident.to_string());
            self.advance();
        } else {
            return Err(format!("Expected identifier, got {:?}", self.current()));
        }

        // 后续部分 (.field)
        while self.check(&Token::Dot) {
            self.advance();
            if let Token::Ident(ident) = self.current() {
                parts.push(ident.to_string());
                self.advance();
            } else {
                return Err(format!(
                    "Expected identifier after '.', got {:?}",
                    self.current()
                ));
            }
        }

        Ok(parts.join("."))
    }

    /// 解析等待条件
    fn parse_wait_condition(&mut self) -> Result<WaitCondition, String> {
        // wait click
        if let Token::Ident(ident) = self.current() {
            if ident == "click" {
                self.advance();
                return Ok(WaitCondition::Click);
            }
        }

        // wait 0.5 (时间)
        let value = self.parse_value()?;
        match value {
            ScriptValue::Float(f) => Ok(WaitCondition::Time(f)),
            ScriptValue::Int(i) => Ok(WaitCondition::Time(i as f64)),
            _ => Err("wait requires time value or 'click'".to_string()),
        }
    }

    /// 解析参数列表
    fn parse_args(&mut self) -> Result<Vec<ScriptValue>, String> {
        let mut args = Vec::new();

        // 检查是否有参数 (不是命令分隔符)
        if self.is_at_end() || self.check(&Token::Newline) || self.check(&Token::Arrow) {
            return Ok(args);
        }

        // 解析后续参数 
        while !self.check(&Token::Newline) && !self.check(&Token::Arrow){
            args.push(self.parse_value()?);
            if self.is_at_end() {
                break;
            }
        }

        Ok(args)
    }

    /// 解析值
    fn parse_value(&mut self) -> Result<ScriptValue, String> {
        match self.current().clone() {
            Token::String(s) => {
                self.advance();
                Ok(ScriptValue::string(s))
            }
            Token::Int(i) => {
                self.advance();
                Ok(ScriptValue::int(i))
            }
            Token::Float(f) => {
                self.advance();
                Ok(ScriptValue::float(f))
            }
            Token::Ident(ident) => {
                self.advance();
                match ident.to_lowercase().as_str() {
                    "true" => Ok(ScriptValue::bool(true)),
                    "false" => Ok(ScriptValue::bool(false)),
                    "nil" => Ok(ScriptValue::nil()),
                    _ => {
                        // 可能是路径引用，解析完整路径
                        let mut path = ident;
                        while self.check(&Token::Dot) {
                            self.advance();
                            if let Token::Ident(part) = self.current() {
                                path.push('.');
                                path.push_str(&part);
                                self.advance();
                            } else {
                                return Err("Expected identifier after '.'".to_string());
                            }
                        }
                        Ok(ScriptValue::Expression(path))
                    }
                }
            }
            _ => Err(format!("Expected value, got {:?}", self.current())),
        }
    }

    fn current(&self) -> &Token {
        self.tokens.get(self.position).unwrap_or(&Token::Eof)
    }

    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.position + 1)
    }

    fn check(&self, token: &Token) -> bool {
        std::mem::discriminant(self.current()) == std::mem::discriminant(token)
    }

    fn advance(&mut self) -> &Token {
        if !self.is_at_end() {
            self.position += 1;
        }
        self.current()
    }

    fn is_at_end(&self) -> bool {
        self.position >= self.tokens.len() || matches!(self.current(), Token::Eof)
    }
}
