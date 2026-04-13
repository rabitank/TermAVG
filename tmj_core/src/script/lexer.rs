use std::iter::Peekable;
use std::str::Chars;

/// Token 类型
#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    /// 标识符 (变量名、方法名等)
    Ident(String),
    /// 字符串字面量
    String(String),
    /// 整数
    Int(i64),
    /// 浮点数
    Float(f64),
    /// 布尔值
    Bool(bool),
    /// 等号 (赋值)
    Equal,
    /// session 标记
    Session,
    /// 点号 (字段访问)
    Dot,
    /// 箭头 (链式调用)
    Arrow,
    /// 逗号 (参数分隔)
    Comma,
    /// 换行/分号 (命令分隔)
    Newline,
    /// 文件结束
    Eof,
    /// 未知字符
    Unknown(char),
}

/// 词法分析器
pub struct Lexer<'a> {
    input: &'a str,
    chars: Peekable<Chars<'a>>,
    position: usize,
}

impl<'a> Lexer<'a> {
    pub fn new(input: &'a str) -> Self {
        Lexer {
            input,
            chars: input.chars().peekable(),
            position: 0,
        }
    }

    /// 词法分析，返回所有 token
    pub fn tokenize(&mut self) -> Result<Vec<Token>, String> {
        let mut tokens = Vec::new();

        while let Some(ch) = self.chars.peek() {
            let ch = *ch;

            match ch {
                // 空白字符 (跳过，但保留换行)
                ' ' | '\t' | '\r' => {
                    self.advance();
                }

                // 换行
                '\n' => {
                    self.advance();
                    tokens.push(Token::Newline);
                }

                // 注释 (跳过整行)
                '/' if self.peek_next() == Some('/') => {
                    self.skip_line();
                }

                // 字符串
                '"' => {
                    let s = self.read_string()?;
                    tokens.push(Token::String(s));
                }

                // 数字
                '0'..='9' => {
                    let num = self.read_number()?;
                    tokens.push(num);
                }

                // 标识符或关键字
                'a'..='z' | 'A'..='Z' | '_' => {
                    let ident = self.read_ident();
                    tokens.push(Token::Ident(ident));
                }

                '#' => {
                    self.advance();
                    tokens.push(Token::Session);
                }

                // 符号
                '=' => {
                    self.advance();
                    tokens.push(Token::Equal);
                }
                '.' => {
                    self.advance();
                    tokens.push(Token::Dot);
                }
                '-' if self.peek_next() == Some('>') => {
                    self.advance();
                    self.advance();
                    tokens.push(Token::Arrow);
                }
                ',' => {
                    self.advance();
                    tokens.push(Token::Comma);
                }

                // 未知字符
                _ => {
                    self.advance();
                    tokens.push(Token::Unknown(ch));
                }
            }
        }

        tokens.push(Token::Eof);
        Ok(tokens)
    }

    fn advance(&mut self) -> Option<char> {
        let ch = self.chars.next();
        if ch.is_some() {
            self.position += 1;
        }
        ch
    }

    fn peek_next(&self) -> Option<char> {
        let mut chars = self.input.chars().skip(self.position + 1);
        chars.next()
    }

    fn skip_line(&mut self) {
        while let Some(ch) = self.chars.peek() {
            if *ch == '\n' {
                break;
            }
            self.advance();
        }
    }

    fn read_string(&mut self) -> Result<String, String> {
        self.advance(); // 跳过开头的 "

        let mut s = String::new();
        while let Some(ch) = self.chars.peek() {
            match ch {
                '"' => {
                    self.advance();
                    return Ok(s);
                }
                '\\' => {
                    self.advance();
                    if let Some(escaped) = self.chars.next() {
                        match escaped {
                            'n' => s.push('\n'),
                            't' => s.push('\t'),
                            '"' => s.push('"'),
                            '\\' => s.push('\\'),
                            _ => s.push(escaped),
                        }
                    }
                }
                _ => {
                    s.push(*ch);
                    self.advance();
                }
            }
        }

        Err("Unterminated string".to_string())
    }

    fn read_number(&mut self) -> Result<Token, String> {
        let mut num_str = String::new();
        let mut is_float = false;

        while let Some(ch) = self.chars.peek().cloned() {
            match ch {
                '0'..='9' => {
                    num_str.push(ch);
                    self.advance();
                }
                '.' if !is_float => {
                    // 检查下一个字符是否是数字 (避免将 .method 误判为浮点数)
                    if self.peek_next().map_or(false, |c| c.is_ascii_digit()) {
                        is_float = true;
                        num_str.push(ch);
                        self.advance();
                    } else {
                        break;
                    }
                }
                _ => break,
            }
        }

        if is_float {
            let num: f64 = num_str.parse().map_err(|_| format!("Invalid float: {}", num_str))?;
            Ok(Token::Float(num))
        } else {
            let num: i64 = num_str.parse().map_err(|_| format!("Invalid int: {}", num_str))?;
            Ok(Token::Int(num))
        }
    }

    fn read_ident(&mut self) -> String {
        let mut ident = String::new();

        while let Some(ch) = self.chars.peek() {
            match ch {
                'a'..='z' | 'A'..='Z' | '0'..='9' | '_' => {
                    ident.push(*ch);
                    self.advance();
                }
                _ => break,
            }
        }

        ident
    }
}
