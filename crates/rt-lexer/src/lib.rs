//! Hand-written lexer for the supported Rust teaching subset.

use rt_common::{Diagnostic, Span};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Token {
    pub kind: TokenKind,
    pub lexeme: String,
    pub span: Span,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TokenKind {
    Fn,
    Let,
    Mut,
    If,
    Else,
    While,
    Loop,
    Break,
    Continue,
    Return,
    True,
    False,
    Ident,
    Int,
    Char,
    Str,
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    Eq,
    EqEq,
    Bang,
    BangEq,
    Lt,
    LtEq,
    Gt,
    GtEq,
    AndAnd,
    OrOr,
    Arrow,
    Colon,
    Semicolon,
    Comma,
    LParen,
    RParen,
    LBrace,
    RBrace,
    Eof,
    Error,
}

impl TokenKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            TokenKind::Fn => "Fn",
            TokenKind::Let => "Let",
            TokenKind::Mut => "Mut",
            TokenKind::If => "If",
            TokenKind::Else => "Else",
            TokenKind::While => "While",
            TokenKind::Loop => "Loop",
            TokenKind::Break => "Break",
            TokenKind::Continue => "Continue",
            TokenKind::Return => "Return",
            TokenKind::True => "True",
            TokenKind::False => "False",
            TokenKind::Ident => "Ident",
            TokenKind::Int => "Int",
            TokenKind::Char => "Char",
            TokenKind::Str => "Str",
            TokenKind::Plus => "Plus",
            TokenKind::Minus => "Minus",
            TokenKind::Star => "Star",
            TokenKind::Slash => "Slash",
            TokenKind::Percent => "Percent",
            TokenKind::Eq => "Eq",
            TokenKind::EqEq => "EqEq",
            TokenKind::Bang => "Bang",
            TokenKind::BangEq => "BangEq",
            TokenKind::Lt => "Lt",
            TokenKind::LtEq => "LtEq",
            TokenKind::Gt => "Gt",
            TokenKind::GtEq => "GtEq",
            TokenKind::AndAnd => "AndAnd",
            TokenKind::OrOr => "OrOr",
            TokenKind::Arrow => "Arrow",
            TokenKind::Colon => "Colon",
            TokenKind::Semicolon => "Semicolon",
            TokenKind::Comma => "Comma",
            TokenKind::LParen => "LParen",
            TokenKind::RParen => "RParen",
            TokenKind::LBrace => "LBrace",
            TokenKind::RBrace => "RBrace",
            TokenKind::Eof => "Eof",
            TokenKind::Error => "Error",
        }
    }
}

#[derive(Debug)]
pub struct Lexer<'a> {
    source: &'a str,
    chars: Vec<char>,
    index: usize,
    byte: usize,
    line: usize,
    column: usize,
    diagnostics: Vec<Diagnostic>,
}

impl<'a> Lexer<'a> {
    pub fn new(source: &'a str) -> Self {
        Self {
            source,
            chars: source.chars().collect(),
            index: 0,
            byte: 0,
            line: 1,
            column: 1,
            diagnostics: Vec::new(),
        }
    }

    pub fn lex(mut self) -> LexResult {
        let mut tokens = Vec::new();

        while !self.is_at_end() {
            self.skip_whitespace_and_comments();
            if self.is_at_end() {
                break;
            }
            tokens.push(self.next_token());
        }

        tokens.push(Token {
            kind: TokenKind::Eof,
            lexeme: String::new(),
            span: Span::new(self.byte, self.byte, self.line, self.column),
        });

        LexResult {
            tokens,
            diagnostics: self.diagnostics,
        }
    }

    fn next_token(&mut self) -> Token {
        let start = self.mark();
        let ch = self.advance().expect("lexer advanced past end");

        match ch {
            c if is_ident_start(c) => self.identifier(start),
            c if c.is_ascii_digit() => self.number(start),
            '"' => self.string(start),
            '\'' => self.char_literal(start),
            '+' => self.simple(TokenKind::Plus, start),
            '*' => self.simple(TokenKind::Star, start),
            '%' => self.simple(TokenKind::Percent, start),
            ':' => self.simple(TokenKind::Colon, start),
            ';' => self.simple(TokenKind::Semicolon, start),
            ',' => self.simple(TokenKind::Comma, start),
            '(' => self.simple(TokenKind::LParen, start),
            ')' => self.simple(TokenKind::RParen, start),
            '{' => self.simple(TokenKind::LBrace, start),
            '}' => self.simple(TokenKind::RBrace, start),
            '-' if self.match_char('>') => self.composite(TokenKind::Arrow, start),
            '-' => self.simple(TokenKind::Minus, start),
            '/' => self.simple(TokenKind::Slash, start),
            '=' if self.match_char('=') => self.composite(TokenKind::EqEq, start),
            '=' => self.simple(TokenKind::Eq, start),
            '!' if self.match_char('=') => self.composite(TokenKind::BangEq, start),
            '!' => self.simple(TokenKind::Bang, start),
            '<' if self.match_char('=') => self.composite(TokenKind::LtEq, start),
            '<' => self.simple(TokenKind::Lt, start),
            '>' if self.match_char('=') => self.composite(TokenKind::GtEq, start),
            '>' => self.simple(TokenKind::Gt, start),
            '&' if self.match_char('&') => self.composite(TokenKind::AndAnd, start),
            '|' if self.match_char('|') => self.composite(TokenKind::OrOr, start),
            other => {
                let span = self.span_from(start);
                self.diagnostics.push(Diagnostic::error(
                    "E0001",
                    format!("unexpected character `{}`", other),
                    span,
                ));
                Token {
                    kind: TokenKind::Error,
                    lexeme: other.to_string(),
                    span,
                }
            }
        }
    }

    fn identifier(&mut self, start: Mark) -> Token {
        while self.peek().is_some_and(is_ident_continue) {
            self.advance();
        }
        let lexeme = self.slice(start.byte, self.byte).to_string();
        let kind = match lexeme.as_str() {
            "fn" => TokenKind::Fn,
            "let" => TokenKind::Let,
            "mut" => TokenKind::Mut,
            "if" => TokenKind::If,
            "else" => TokenKind::Else,
            "while" => TokenKind::While,
            "loop" => TokenKind::Loop,
            "break" => TokenKind::Break,
            "continue" => TokenKind::Continue,
            "return" => TokenKind::Return,
            "true" => TokenKind::True,
            "false" => TokenKind::False,
            _ => TokenKind::Ident,
        };
        Token {
            kind,
            lexeme,
            span: self.span_from(start),
        }
    }

    fn number(&mut self, start: Mark) -> Token {
        while self.peek().is_some_and(|c| c.is_ascii_digit()) {
            self.advance();
        }
        Token {
            kind: TokenKind::Int,
            lexeme: self.slice(start.byte, self.byte).to_string(),
            span: self.span_from(start),
        }
    }

    fn string(&mut self, start: Mark) -> Token {
        let mut terminated = false;
        while let Some(ch) = self.peek() {
            if ch == '"' {
                self.advance();
                terminated = true;
                break;
            }
            if ch == '\\' {
                self.advance();
            }
            self.advance();
        }
        let span = self.span_from(start);
        if !terminated {
            self.diagnostics.push(Diagnostic::error(
                "E0002",
                "unterminated string literal",
                span,
            ));
        }
        Token {
            kind: if terminated {
                TokenKind::Str
            } else {
                TokenKind::Error
            },
            lexeme: self.slice(start.byte, self.byte).to_string(),
            span,
        }
    }

    fn char_literal(&mut self, start: Mark) -> Token {
        let mut terminated = false;
        let mut logical_chars = 0;
        while let Some(ch) = self.peek() {
            self.advance();
            if ch == '\\' {
                if self.advance().is_some() {
                    logical_chars += 1;
                }
                continue;
            }
            if ch == '\'' {
                terminated = true;
                break;
            }
            logical_chars += 1;
        }
        let span = self.span_from(start);
        if !terminated {
            self.diagnostics.push(Diagnostic::error(
                "E0003",
                "unterminated char literal",
                span,
            ));
        } else if logical_chars != 1 {
            self.diagnostics.push(Diagnostic::error(
                "E0005",
                "char literal must contain exactly one character",
                span,
            ));
        }
        Token {
            kind: if terminated && logical_chars == 1 {
                TokenKind::Char
            } else {
                TokenKind::Error
            },
            lexeme: self.slice(start.byte, self.byte).to_string(),
            span,
        }
    }

    fn simple(&self, kind: TokenKind, start: Mark) -> Token {
        Token {
            kind,
            lexeme: self.slice(start.byte, self.byte).to_string(),
            span: self.span_from(start),
        }
    }

    fn composite(&self, kind: TokenKind, start: Mark) -> Token {
        self.simple(kind, start)
    }

    fn skip_whitespace_and_comments(&mut self) {
        loop {
            while self.peek().is_some_and(|c| c.is_whitespace()) {
                self.advance();
            }
            if self.peek() == Some('/') && self.peek_next() == Some('/') {
                while self.peek().is_some_and(|c| c != '\n') {
                    self.advance();
                }
                continue;
            }
            if self.peek() == Some('/') && self.peek_next() == Some('*') {
                self.skip_block_comment();
                continue;
            }
            break;
        }
    }

    fn skip_block_comment(&mut self) {
        let start = self.mark();
        self.advance();
        self.advance();

        let mut depth = 1usize;
        while let Some(ch) = self.peek() {
            if ch == '/' && self.peek_next() == Some('*') {
                self.advance();
                self.advance();
                depth += 1;
                continue;
            }
            if ch == '*' && self.peek_next() == Some('/') {
                self.advance();
                self.advance();
                depth -= 1;
                if depth == 0 {
                    return;
                }
                continue;
            }
            self.advance();
        }

        self.diagnostics.push(Diagnostic::error(
            "E0004",
            "unterminated block comment",
            self.span_from(start),
        ));
    }

    fn match_char(&mut self, expected: char) -> bool {
        if self.peek() == Some(expected) {
            self.advance();
            true
        } else {
            false
        }
    }

    fn advance(&mut self) -> Option<char> {
        let ch = self.chars.get(self.index).copied()?;
        self.index += 1;
        self.byte += ch.len_utf8();
        if ch == '\n' {
            self.line += 1;
            self.column = 1;
        } else {
            self.column += 1;
        }
        Some(ch)
    }

    fn peek(&self) -> Option<char> {
        self.chars.get(self.index).copied()
    }

    fn peek_next(&self) -> Option<char> {
        self.chars.get(self.index + 1).copied()
    }

    fn is_at_end(&self) -> bool {
        self.index >= self.chars.len()
    }

    fn mark(&self) -> Mark {
        Mark {
            byte: self.byte,
            line: self.line,
            column: self.column,
        }
    }

    fn span_from(&self, mark: Mark) -> Span {
        Span::new(mark.byte, self.byte, mark.line, mark.column)
    }

    fn slice(&self, start: usize, end: usize) -> &str {
        &self.source[start..end]
    }
}

#[derive(Clone, Copy, Debug)]
struct Mark {
    byte: usize,
    line: usize,
    column: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LexResult {
    pub tokens: Vec<Token>,
    pub diagnostics: Vec<Diagnostic>,
}

fn is_ident_start(ch: char) -> bool {
    ch == '_' || ch.is_ascii_alphabetic()
}

fn is_ident_continue(ch: char) -> bool {
    is_ident_start(ch) || ch.is_ascii_digit()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kinds(source: &str) -> Vec<TokenKind> {
        Lexer::new(source)
            .lex()
            .tokens
            .into_iter()
            .map(|token| token.kind)
            .collect()
    }

    #[test]
    fn lexes_basic_function() {
        let result = Lexer::new("fn main() -> i32 { return 42; }").lex();
        let kinds: Vec<TokenKind> = result
            .tokens
            .iter()
            .map(|token| token.kind.clone())
            .collect();
        assert_eq!(
            kinds,
            vec![
                TokenKind::Fn,
                TokenKind::Ident,
                TokenKind::LParen,
                TokenKind::RParen,
                TokenKind::Arrow,
                TokenKind::Ident,
                TokenKind::LBrace,
                TokenKind::Return,
                TokenKind::Int,
                TokenKind::Semicolon,
                TokenKind::RBrace,
                TokenKind::Eof,
            ]
        );
        assert!(result.diagnostics.is_empty());
    }

    #[test]
    fn tracks_line_and_column() {
        let result = Lexer::new("fn\nmain").lex();
        assert_eq!(result.tokens[1].span.line, 2);
        assert_eq!(result.tokens[1].span.column, 1);
    }

    #[test]
    fn lexes_keywords() {
        assert_eq!(
            kinds("fn let mut if else while loop break continue return true false"),
            vec![
                TokenKind::Fn,
                TokenKind::Let,
                TokenKind::Mut,
                TokenKind::If,
                TokenKind::Else,
                TokenKind::While,
                TokenKind::Loop,
                TokenKind::Break,
                TokenKind::Continue,
                TokenKind::Return,
                TokenKind::True,
                TokenKind::False,
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn lexes_operators_and_delimiters() {
        assert_eq!(
            kinds("+ - * / % = == ! != < <= > >= && || -> : ; , ( ) { }"),
            vec![
                TokenKind::Plus,
                TokenKind::Minus,
                TokenKind::Star,
                TokenKind::Slash,
                TokenKind::Percent,
                TokenKind::Eq,
                TokenKind::EqEq,
                TokenKind::Bang,
                TokenKind::BangEq,
                TokenKind::Lt,
                TokenKind::LtEq,
                TokenKind::Gt,
                TokenKind::GtEq,
                TokenKind::AndAnd,
                TokenKind::OrOr,
                TokenKind::Arrow,
                TokenKind::Colon,
                TokenKind::Semicolon,
                TokenKind::Comma,
                TokenKind::LParen,
                TokenKind::RParen,
                TokenKind::LBrace,
                TokenKind::RBrace,
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn skips_line_and_nested_block_comments() {
        let result = Lexer::new("fn /* outer /* inner */ done */ main // tail\n()").lex();
        assert_eq!(
            result
                .tokens
                .iter()
                .map(|token| token.kind.clone())
                .collect::<Vec<_>>(),
            vec![
                TokenKind::Fn,
                TokenKind::Ident,
                TokenKind::LParen,
                TokenKind::RParen,
                TokenKind::Eof,
            ]
        );
        assert!(result.diagnostics.is_empty());
    }

    #[test]
    fn lexes_literals() {
        assert_eq!(
            kinds("123 'x' '\\n' \"hello\""),
            vec![
                TokenKind::Int,
                TokenKind::Char,
                TokenKind::Char,
                TokenKind::Str,
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn reports_unterminated_block_comment() {
        let result = Lexer::new("fn /* missing").lex();
        assert_eq!(result.diagnostics[0].code, "E0004");
    }

    #[test]
    fn reports_invalid_char_literal() {
        let result = Lexer::new("'ab'").lex();
        assert_eq!(result.tokens[0].kind, TokenKind::Error);
        assert_eq!(result.diagnostics[0].code, "E0005");
    }
}
