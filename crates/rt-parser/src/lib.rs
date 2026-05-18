//! Recursive descent parser for the supported Rust teaching subset.

use rt_ast::AstNode;
use rt_common::{Diagnostic, Span};
use rt_lexer::{Token, TokenKind};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ParseResult {
    pub root: AstNode,
    pub diagnostics: Vec<Diagnostic>,
}

pub fn parse_tokens(tokens: &[Token]) -> ParseResult {
    Parser::new(tokens).parse_program()
}

struct Parser<'a> {
    tokens: &'a [Token],
    current: usize,
    next_id: usize,
    diagnostics: Vec<Diagnostic>,
}

impl<'a> Parser<'a> {
    fn new(tokens: &'a [Token]) -> Self {
        Self {
            tokens,
            current: 0,
            next_id: 0,
            diagnostics: Vec::new(),
        }
    }

    fn parse_program(mut self) -> ParseResult {
        let mut children = Vec::new();
        while !self.at(TokenKind::Eof) {
            if self.at(TokenKind::Fn) {
                children.push(self.parse_function());
            } else {
                let token = self.advance();
                self.error_at(
                    token.span,
                    "E1001",
                    format!("expected function item, found `{}`", token.lexeme),
                );
            }
        }

        let span = children
            .iter()
            .map(|child| child.span)
            .reduce(Span::join)
            .unwrap_or_else(|| self.peek().span);
        let mut root = self.node("Program", "", span);
        root.children = children;

        ParseResult {
            root,
            diagnostics: self.diagnostics,
        }
    }

    fn parse_function(&mut self) -> AstNode {
        let start = self.expect(TokenKind::Fn, "E1002", "expected `fn`").span;
        let name = self.expect_ident("E1003", "expected function name");
        self.expect(
            TokenKind::LParen,
            "E1004",
            "expected `(` after function name",
        );

        let mut children = Vec::new();
        while !self.at(TokenKind::RParen) && !self.at(TokenKind::Eof) {
            children.push(self.parse_param());
            if !self.match_kind(TokenKind::Comma) {
                break;
            }
        }
        self.expect(TokenKind::RParen, "E1005", "expected `)` after parameters");

        if self.match_kind(TokenKind::Arrow) {
            children.push(self.parse_type("ReturnType"));
        }

        let body = self.parse_block();
        let span = start.join(body.span);
        children.push(body);

        let mut node = self.node("Function", name.lexeme, span);
        node.children = children;
        node
    }

    fn parse_param(&mut self) -> AstNode {
        let name = self.expect_ident("E1006", "expected parameter name");
        self.expect(
            TokenKind::Colon,
            "E1007",
            "expected `:` after parameter name",
        );
        let type_node = self.parse_type("Type");
        let span = name.span.join(type_node.span);
        self.node_with_children("Param", name.lexeme, span, vec![type_node])
    }

    fn parse_type(&mut self, kind: &str) -> AstNode {
        let token = self.expect_ident("E1008", "expected type name");
        self.node(kind, token.lexeme, token.span)
    }

    fn parse_block(&mut self) -> AstNode {
        let start = self.expect(TokenKind::LBrace, "E1009", "expected `{`").span;
        let mut children = Vec::new();
        while !self.at(TokenKind::RBrace) && !self.at(TokenKind::Eof) {
            children.push(self.parse_stmt());
        }
        let end = self.expect(TokenKind::RBrace, "E1010", "expected `}`").span;
        self.node_with_children("Block", "", start.join(end), children)
    }

    fn parse_stmt(&mut self) -> AstNode {
        match self.peek().kind {
            TokenKind::Let => self.parse_let_stmt(),
            TokenKind::Return => self.parse_return_stmt(),
            TokenKind::If => self.parse_if_stmt(),
            TokenKind::While => self.parse_while_stmt(),
            TokenKind::Loop => self.parse_loop_stmt(),
            TokenKind::Break => self.parse_keyword_stmt("BreakStmt", TokenKind::Break),
            TokenKind::Continue => self.parse_keyword_stmt("ContinueStmt", TokenKind::Continue),
            _ => self.parse_expr_stmt(),
        }
    }

    fn parse_let_stmt(&mut self) -> AstNode {
        let start = self.expect(TokenKind::Let, "E1011", "expected `let`").span;
        let mutable = self.match_kind(TokenKind::Mut);
        let name = self.expect_ident("E1012", "expected binding name");
        let mut children = Vec::new();

        if self.match_kind(TokenKind::Colon) {
            children.push(self.parse_type("Type"));
        }
        if self.match_kind(TokenKind::Eq) {
            children.push(self.parse_expression(0));
        }

        let end = self
            .expect(TokenKind::Semicolon, "E1013", "expected `;` after let")
            .span;
        self.node_with_children(
            "LetStmt",
            if mutable {
                format!("mut {}", name.lexeme)
            } else {
                name.lexeme
            },
            start.join(end),
            children,
        )
    }

    fn parse_return_stmt(&mut self) -> AstNode {
        let start = self
            .expect(TokenKind::Return, "E1014", "expected `return`")
            .span;
        let mut children = Vec::new();
        if !self.at(TokenKind::Semicolon) {
            children.push(self.parse_expression(0));
        }
        let end = self
            .expect(TokenKind::Semicolon, "E1015", "expected `;` after return")
            .span;
        self.node_with_children("ReturnStmt", "", start.join(end), children)
    }

    fn parse_if_stmt(&mut self) -> AstNode {
        let start = self.expect(TokenKind::If, "E1016", "expected `if`").span;
        let condition = self.parse_expression(0);
        let then_block = self.parse_block();
        let mut children = vec![condition, then_block];
        let mut end = children.last().expect("then block exists").span;

        if self.match_kind(TokenKind::Else) {
            let else_node = if self.at(TokenKind::If) {
                self.parse_if_stmt()
            } else {
                self.parse_block()
            };
            end = else_node.span;
            children.push(else_node);
        }

        self.node_with_children("IfStmt", "", start.join(end), children)
    }

    fn parse_while_stmt(&mut self) -> AstNode {
        let start = self
            .expect(TokenKind::While, "E1017", "expected `while`")
            .span;
        let condition = self.parse_expression(0);
        let body = self.parse_block();
        self.node_with_children(
            "WhileStmt",
            "",
            start.join(body.span),
            vec![condition, body],
        )
    }

    fn parse_loop_stmt(&mut self) -> AstNode {
        let start = self
            .expect(TokenKind::Loop, "E1025", "expected `loop`")
            .span;
        let body = self.parse_block();
        self.node_with_children("LoopStmt", "", start.join(body.span), vec![body])
    }

    fn parse_keyword_stmt(&mut self, kind: &str, token_kind: TokenKind) -> AstNode {
        let start = self.expect(token_kind, "E1018", "expected statement").span;
        let end = self
            .expect(
                TokenKind::Semicolon,
                "E1019",
                "expected `;` after statement",
            )
            .span;
        self.node(kind, "", start.join(end))
    }

    fn parse_expr_stmt(&mut self) -> AstNode {
        let expr = self.parse_expression(0);
        let end = self
            .expect(
                TokenKind::Semicolon,
                "E1020",
                "expected `;` after expression",
            )
            .span;
        self.node_with_children("ExprStmt", "", expr.span.join(end), vec![expr])
    }

    fn parse_expression(&mut self, min_bp: u8) -> AstNode {
        let mut lhs = self.parse_prefix();

        loop {
            if self.at(TokenKind::LParen) {
                lhs = self.parse_call(lhs);
                continue;
            }

            let Some((left_bp, right_bp, label)) = infix_binding_power(&self.peek().kind) else {
                break;
            };
            if left_bp < min_bp {
                break;
            }

            let op = self.advance();
            let rhs = self.parse_expression(right_bp);
            let span = lhs.span.join(rhs.span);
            lhs = self.node_with_children(
                "BinaryExpr",
                label.unwrap_or(op.lexeme.as_str()),
                span,
                vec![lhs, rhs],
            );
        }

        lhs
    }

    fn parse_prefix(&mut self) -> AstNode {
        match self.peek().kind {
            TokenKind::Minus | TokenKind::Bang => {
                let op = self.advance();
                let rhs = self.parse_expression(7);
                self.node_with_children("UnaryExpr", op.lexeme, op.span.join(rhs.span), vec![rhs])
            }
            TokenKind::Int => {
                let token = self.advance();
                self.node("IntLiteral", token.lexeme, token.span)
            }
            TokenKind::True | TokenKind::False => {
                let token = self.advance();
                self.node("BoolLiteral", token.lexeme, token.span)
            }
            TokenKind::Char => {
                let token = self.advance();
                self.node("CharLiteral", token.lexeme, token.span)
            }
            TokenKind::Str => {
                let token = self.advance();
                self.node("StringLiteral", token.lexeme, token.span)
            }
            TokenKind::Ident => {
                let token = self.advance();
                self.node("IdentExpr", token.lexeme, token.span)
            }
            TokenKind::LParen => {
                let start = self.advance().span;
                let expr = self.parse_expression(0);
                let end = self.expect(TokenKind::RParen, "E1021", "expected `)`").span;
                self.node_with_children("ParenExpr", "", start.join(end), vec![expr])
            }
            _ => {
                let token = self.advance();
                self.error_at(
                    token.span,
                    "E1022",
                    format!("expected expression, found `{}`", token.lexeme),
                );
                self.node("ErrorExpr", token.lexeme, token.span)
            }
        }
    }

    fn parse_call(&mut self, callee: AstNode) -> AstNode {
        let start = callee.span;
        self.expect(TokenKind::LParen, "E1023", "expected `(`");
        let mut children = vec![callee];
        while !self.at(TokenKind::RParen) && !self.at(TokenKind::Eof) {
            children.push(self.parse_expression(0));
            if !self.match_kind(TokenKind::Comma) {
                break;
            }
        }
        let end = self.expect(TokenKind::RParen, "E1024", "expected `)`").span;
        self.node_with_children("CallExpr", "", start.join(end), children)
    }

    fn expect_ident(&mut self, code: &str, message: &str) -> Token {
        if self.at(TokenKind::Ident) {
            self.advance()
        } else {
            let token = self.peek().clone();
            self.error_at(token.span, code, message);
            token
        }
    }

    fn expect(&mut self, kind: TokenKind, code: &str, message: &str) -> Token {
        if self.at(kind) {
            self.advance()
        } else {
            let token = self.peek().clone();
            self.error_at(token.span, code, message);
            token
        }
    }

    fn match_kind(&mut self, kind: TokenKind) -> bool {
        if self.at(kind) {
            self.advance();
            true
        } else {
            false
        }
    }

    fn at(&self, kind: TokenKind) -> bool {
        self.peek().kind == kind
    }

    fn peek(&self) -> &Token {
        self.tokens
            .get(self.current)
            .or_else(|| self.tokens.last())
            .expect("parser requires EOF token")
    }

    fn advance(&mut self) -> Token {
        let token = self.peek().clone();
        if token.kind != TokenKind::Eof {
            self.current += 1;
        }
        token
    }

    fn node(&mut self, kind: impl Into<String>, label: impl Into<String>, span: Span) -> AstNode {
        let id = self.next_id;
        self.next_id += 1;
        AstNode::new(id, kind, label, span)
    }

    fn node_with_children(
        &mut self,
        kind: impl Into<String>,
        label: impl Into<String>,
        span: Span,
        children: Vec<AstNode>,
    ) -> AstNode {
        let mut node = self.node(kind, label, span);
        node.children = children;
        node
    }

    fn error_at(&mut self, span: Span, code: impl Into<String>, message: impl Into<String>) {
        self.diagnostics
            .push(Diagnostic::error(code, message, span));
    }
}

fn infix_binding_power(kind: &TokenKind) -> Option<(u8, u8, Option<&'static str>)> {
    match kind {
        TokenKind::Eq => Some((1, 1, Some("="))),
        TokenKind::OrOr => Some((2, 3, Some("||"))),
        TokenKind::AndAnd => Some((4, 5, Some("&&"))),
        TokenKind::EqEq => Some((6, 7, Some("=="))),
        TokenKind::BangEq => Some((6, 7, Some("!="))),
        TokenKind::Lt => Some((8, 9, Some("<"))),
        TokenKind::LtEq => Some((8, 9, Some("<="))),
        TokenKind::Gt => Some((8, 9, Some(">"))),
        TokenKind::GtEq => Some((8, 9, Some(">="))),
        TokenKind::Plus => Some((10, 11, Some("+"))),
        TokenKind::Minus => Some((10, 11, Some("-"))),
        TokenKind::Star => Some((12, 13, Some("*"))),
        TokenKind::Slash => Some((12, 13, Some("/"))),
        TokenKind::Percent => Some((12, 13, Some("%"))),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rt_lexer::Lexer;

    fn parse(source: &str) -> ParseResult {
        let lexed = Lexer::new(source).lex();
        parse_tokens(&lexed.tokens)
    }

    #[test]
    fn parses_basic_function() {
        let parsed = parse("fn main() -> i32 { return 42; }");
        assert_eq!(parsed.root.kind, "Program");
        assert_eq!(parsed.root.children[0].kind, "Function");
        assert_eq!(parsed.root.children[0].label, "main");
        assert!(parsed.diagnostics.is_empty());
    }

    #[test]
    fn parses_let_return_and_expression_precedence() {
        let parsed = parse("fn main() -> i32 { let x: i32 = 1 + 2 * 3; return x; }");
        let body = parsed
            .root
            .children
            .iter()
            .find(|node| node.kind == "Function")
            .and_then(|function| function.children.iter().find(|node| node.kind == "Block"))
            .expect("function body");
        assert_eq!(body.children[0].kind, "LetStmt");
        assert_eq!(body.children[1].kind, "ReturnStmt");
        assert!(parsed.diagnostics.is_empty());
    }

    #[test]
    fn parses_control_flow_and_calls() {
        let parsed = parse(
            "fn main() -> i32 { while check(1, 2) { if true { break; } else { continue; } } return 0; }",
        );
        assert!(parsed.diagnostics.is_empty());
    }

    #[test]
    fn reports_missing_semicolon() {
        let parsed = parse("fn main() -> i32 { return 42 }");
        assert_eq!(parsed.diagnostics[0].code, "E1015");
    }
}
