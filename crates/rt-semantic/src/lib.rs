//! Semantic analysis for names, scopes, and basic types.

use std::collections::HashMap;

use rt_ast::AstNode;
use rt_common::{Diagnostic, Span};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SemanticTrace {
    pub scopes: Vec<ScopeTrace>,
    pub symbols: Vec<SymbolTrace>,
    pub expression_types: Vec<ExpressionType>,
    pub diagnostics: Vec<Diagnostic>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ScopeTrace {
    pub id: usize,
    pub parent: Option<usize>,
    pub label: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SymbolTrace {
    pub name: String,
    pub kind: SymbolKind,
    pub ty: Type,
    pub mutable: bool,
    pub scope_id: usize,
    pub span: Span,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExpressionType {
    pub node_id: usize,
    pub ty: Type,
    pub span: Span,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SymbolKind {
    Function,
    Parameter,
    Variable,
}

impl SymbolKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            SymbolKind::Function => "function",
            SymbolKind::Parameter => "parameter",
            SymbolKind::Variable => "variable",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Type {
    I32,
    Bool,
    Char,
    Str,
    Unit,
    Unknown,
}

impl Type {
    pub fn as_str(&self) -> &'static str {
        match self {
            Type::I32 => "i32",
            Type::Bool => "bool",
            Type::Char => "char",
            Type::Str => "str",
            Type::Unit => "()",
            Type::Unknown => "unknown",
        }
    }
}

#[derive(Clone, Debug)]
struct FunctionSig {
    name: String,
    params: Vec<Type>,
    return_type: Type,
}

#[derive(Clone, Debug)]
struct SymbolInfo {
    ty: Type,
    mutable: bool,
}

#[derive(Default)]
struct Analyzer {
    functions: HashMap<String, FunctionSig>,
    scope_stack: Vec<HashMap<String, SymbolInfo>>,
    scope_ids: Vec<usize>,
    trace: SemanticTraceBuilder,
}

#[derive(Default)]
struct SemanticTraceBuilder {
    scopes: Vec<ScopeTrace>,
    symbols: Vec<SymbolTrace>,
    expression_types: Vec<ExpressionType>,
    diagnostics: Vec<Diagnostic>,
}

impl SemanticTraceBuilder {
    fn finish(self) -> SemanticTrace {
        SemanticTrace {
            scopes: self.scopes,
            symbols: self.symbols,
            expression_types: self.expression_types,
            diagnostics: self.diagnostics,
        }
    }
}

pub fn analyze(root: &AstNode) -> SemanticTrace {
    let mut analyzer = Analyzer::default();
    analyzer.enter_scope("global");
    analyzer.declare_functions(root);
    analyzer.analyze_program(root);
    analyzer.leave_scope();
    analyzer.trace.finish()
}

impl Analyzer {
    fn declare_functions(&mut self, root: &AstNode) {
        for function in root.children.iter().filter(|node| node.kind == "Function") {
            let return_type = function
                .children
                .iter()
                .find(|child| child.kind == "ReturnType")
                .map(|node| parse_type(&node.label))
                .unwrap_or(Type::Unit);
            let params = function
                .children
                .iter()
                .filter(|child| child.kind == "Param")
                .map(|param| {
                    param
                        .children
                        .first()
                        .map(|ty| parse_type(&ty.label))
                        .unwrap_or(Type::Unknown)
                })
                .collect::<Vec<_>>();

            if self.functions.contains_key(&function.label) {
                self.error(
                    function.span,
                    "E2001",
                    format!("duplicate function `{}`", function.label),
                );
                continue;
            }

            let sig = FunctionSig {
                name: function.label.clone(),
                params,
                return_type: return_type.clone(),
            };
            self.functions.insert(function.label.clone(), sig);
            self.trace.symbols.push(SymbolTrace {
                name: function.label.clone(),
                kind: SymbolKind::Function,
                ty: return_type,
                mutable: false,
                scope_id: self.current_scope_id(),
                span: function.span,
            });
        }
    }

    fn analyze_program(&mut self, root: &AstNode) {
        for function in root.children.iter().filter(|node| node.kind == "Function") {
            self.analyze_function(function);
        }
    }

    fn analyze_function(&mut self, function: &AstNode) {
        let return_type = self
            .functions
            .get(&function.label)
            .map(|sig| sig.return_type.clone())
            .unwrap_or(Type::Unknown);

        self.enter_scope(format!("fn {}", function.label));
        for param in function
            .children
            .iter()
            .filter(|child| child.kind == "Param")
        {
            let ty = param
                .children
                .first()
                .map(|node| parse_type(&node.label))
                .unwrap_or(Type::Unknown);
            self.declare_symbol(param, SymbolKind::Parameter, ty, false);
        }

        if let Some(body) = function.children.iter().find(|child| child.kind == "Block") {
            self.analyze_block(body, &return_type);
        }
        self.leave_scope();
    }

    fn analyze_block(&mut self, block: &AstNode, expected_return: &Type) {
        self.enter_scope("block");
        for stmt in &block.children {
            self.analyze_stmt(stmt, expected_return);
        }
        self.leave_scope();
    }

    fn analyze_stmt(&mut self, stmt: &AstNode, expected_return: &Type) {
        match stmt.kind.as_str() {
            "LetStmt" => self.analyze_let(stmt),
            "ReturnStmt" => self.analyze_return(stmt, expected_return),
            "IfStmt" => self.analyze_if(stmt, expected_return),
            "WhileStmt" | "LoopStmt" => self.analyze_loop_like(stmt, expected_return),
            "ExprStmt" => {
                if let Some(expr) = stmt.children.first() {
                    self.analyze_expr(expr);
                }
            }
            "BreakStmt" | "ContinueStmt" => {}
            _ => self.error(
                stmt.span,
                "E2002",
                format!("unknown statement kind `{}`", stmt.kind),
            ),
        }
    }

    fn analyze_let(&mut self, stmt: &AstNode) {
        let mutable = stmt.label.starts_with("mut ");
        let name = if mutable {
            stmt.label.trim_start_matches("mut ").to_string()
        } else {
            stmt.label.clone()
        };

        let annotation = stmt
            .children
            .iter()
            .find(|child| child.kind == "Type")
            .map(|node| parse_type(&node.label));
        let initializer = stmt.children.iter().find(|child| child.kind != "Type");
        let init_ty = initializer.map(|expr| self.analyze_expr(expr));
        let ty = annotation
            .clone()
            .or(init_ty.clone())
            .unwrap_or(Type::Unknown);

        if let (Some(annotation), Some(init_ty)) = (annotation, init_ty) {
            self.require_same_type(&annotation, &init_ty, stmt.span, "E2005");
        }

        let symbol_node = AstNode {
            label: name,
            ..stmt.clone()
        };
        self.declare_symbol(&symbol_node, SymbolKind::Variable, ty, mutable);
    }

    fn analyze_return(&mut self, stmt: &AstNode, expected_return: &Type) {
        let actual = stmt
            .children
            .first()
            .map(|expr| self.analyze_expr(expr))
            .unwrap_or(Type::Unit);
        self.require_same_type(expected_return, &actual, stmt.span, "E2006");
    }

    fn analyze_if(&mut self, stmt: &AstNode, expected_return: &Type) {
        if let Some(condition) = stmt.children.first() {
            let condition_ty = self.analyze_expr(condition);
            self.require_bool(&condition_ty, condition.span, "E2007");
        }
        for branch in stmt.children.iter().skip(1) {
            if branch.kind == "Block" {
                self.analyze_block(branch, expected_return);
            } else {
                self.analyze_stmt(branch, expected_return);
            }
        }
    }

    fn analyze_loop_like(&mut self, stmt: &AstNode, expected_return: &Type) {
        if stmt.kind == "WhileStmt" {
            if let Some(condition) = stmt.children.first() {
                let condition_ty = self.analyze_expr(condition);
                self.require_bool(&condition_ty, condition.span, "E2007");
            }
            if let Some(body) = stmt.children.get(1) {
                self.analyze_block(body, expected_return);
            }
        } else if let Some(body) = stmt.children.first() {
            self.analyze_block(body, expected_return);
        }
    }

    fn analyze_expr(&mut self, expr: &AstNode) -> Type {
        let ty = match expr.kind.as_str() {
            "IntLiteral" => Type::I32,
            "BoolLiteral" => Type::Bool,
            "CharLiteral" => Type::Char,
            "StringLiteral" => Type::Str,
            "IdentExpr" => self.lookup(&expr.label).unwrap_or_else(|| {
                self.error(
                    expr.span,
                    "E2003",
                    format!("undefined variable `{}`", expr.label),
                );
                Type::Unknown
            }),
            "ParenExpr" => expr
                .children
                .first()
                .map(|child| self.analyze_expr(child))
                .unwrap_or(Type::Unknown),
            "UnaryExpr" => self.analyze_unary(expr),
            "BinaryExpr" => self.analyze_binary(expr),
            "CallExpr" => self.analyze_call(expr),
            "ErrorExpr" => Type::Unknown,
            _ => {
                self.error(
                    expr.span,
                    "E2004",
                    format!("unknown expression kind `{}`", expr.kind),
                );
                Type::Unknown
            }
        };
        self.trace.expression_types.push(ExpressionType {
            node_id: expr.id,
            ty: ty.clone(),
            span: expr.span,
        });
        ty
    }

    fn analyze_unary(&mut self, expr: &AstNode) -> Type {
        let operand = expr
            .children
            .first()
            .map(|child| self.analyze_expr(child))
            .unwrap_or(Type::Unknown);
        match expr.label.as_str() {
            "-" => {
                self.require_same_type(&Type::I32, &operand, expr.span, "E2010");
                Type::I32
            }
            "!" => {
                self.require_bool(&operand, expr.span, "E2011");
                Type::Bool
            }
            _ => Type::Unknown,
        }
    }

    fn analyze_binary(&mut self, expr: &AstNode) -> Type {
        let lhs = expr
            .children
            .first()
            .map(|child| self.analyze_expr(child))
            .unwrap_or(Type::Unknown);
        let rhs = expr
            .children
            .get(1)
            .map(|child| self.analyze_expr(child))
            .unwrap_or(Type::Unknown);

        match expr.label.as_str() {
            "=" => {
                if let Some(target) = expr.children.first() {
                    self.check_assignment_target(target);
                }
                self.require_same_type(&lhs, &rhs, expr.span, "E2009");
                lhs
            }
            "+" | "-" | "*" | "/" | "%" => {
                self.require_same_type(&Type::I32, &lhs, expr.span, "E2010");
                self.require_same_type(&Type::I32, &rhs, expr.span, "E2010");
                Type::I32
            }
            "==" | "!=" => {
                self.require_same_type(&lhs, &rhs, expr.span, "E2012");
                Type::Bool
            }
            "<" | "<=" | ">" | ">=" => {
                self.require_same_type(&Type::I32, &lhs, expr.span, "E2010");
                self.require_same_type(&Type::I32, &rhs, expr.span, "E2010");
                Type::Bool
            }
            "&&" | "||" => {
                self.require_bool(&lhs, expr.span, "E2011");
                self.require_bool(&rhs, expr.span, "E2011");
                Type::Bool
            }
            _ => Type::Unknown,
        }
    }

    fn analyze_call(&mut self, expr: &AstNode) -> Type {
        let Some(callee) = expr.children.first() else {
            return Type::Unknown;
        };
        let name = &callee.label;
        let Some(sig) = self.functions.get(name).cloned() else {
            self.error(
                callee.span,
                "E2014",
                format!("undefined function `{}`", name),
            );
            for arg in expr.children.iter().skip(1) {
                self.analyze_expr(arg);
            }
            return Type::Unknown;
        };

        let args = expr.children.iter().skip(1).collect::<Vec<_>>();
        if args.len() != sig.params.len() {
            self.error(
                expr.span,
                "E2015",
                format!(
                    "function `{}` expects {} argument(s), got {}",
                    sig.name,
                    sig.params.len(),
                    args.len()
                ),
            );
        }

        for (index, arg) in args.iter().enumerate() {
            let arg_ty = self.analyze_expr(arg);
            if let Some(expected) = sig.params.get(index) {
                self.require_same_type(expected, &arg_ty, arg.span, "E2016");
            }
        }

        sig.return_type
    }

    fn check_assignment_target(&mut self, target: &AstNode) {
        if target.kind != "IdentExpr" {
            self.error(target.span, "E2008", "assignment target must be a variable");
            return;
        }
        match self.lookup_symbol(&target.label) {
            Some(symbol) if symbol.mutable => {}
            Some(_) => self.error(
                target.span,
                "E2008",
                format!("cannot assign to immutable variable `{}`", target.label),
            ),
            None => self.error(
                target.span,
                "E2003",
                format!("undefined variable `{}`", target.label),
            ),
        }
    }

    fn declare_symbol(&mut self, node: &AstNode, kind: SymbolKind, ty: Type, mutable: bool) {
        if self.current_scope_contains(&node.label) {
            self.error(
                node.span,
                "E2013",
                format!("duplicate declaration `{}`", node.label),
            );
            return;
        }

        let scope_id = self.current_scope_id();
        self.scope_stack
            .last_mut()
            .expect("semantic scope exists")
            .insert(
                node.label.clone(),
                SymbolInfo {
                    ty: ty.clone(),
                    mutable,
                },
            );
        self.trace.symbols.push(SymbolTrace {
            name: node.label.clone(),
            kind,
            ty,
            mutable,
            scope_id,
            span: node.span,
        });
    }

    fn lookup(&self, name: &str) -> Option<Type> {
        self.lookup_symbol(name).map(|symbol| symbol.ty)
    }

    fn lookup_symbol(&self, name: &str) -> Option<SymbolInfo> {
        for scope in self.scope_stack.iter().rev() {
            if let Some(symbol) = scope.get(name) {
                return Some(symbol.clone());
            }
        }
        None
    }

    fn current_scope_contains(&self, name: &str) -> bool {
        self.scope_stack
            .last()
            .is_some_and(|scope| scope.contains_key(name))
    }

    fn require_same_type(&mut self, expected: &Type, actual: &Type, span: Span, code: &str) {
        if matches!(expected, Type::Unknown) || matches!(actual, Type::Unknown) {
            return;
        }
        if expected != actual {
            self.error(
                span,
                code,
                format!(
                    "type mismatch: expected `{}`, got `{}`",
                    expected.as_str(),
                    actual.as_str()
                ),
            );
        }
    }

    fn require_bool(&mut self, actual: &Type, span: Span, code: &str) {
        self.require_same_type(&Type::Bool, actual, span, code);
    }

    fn enter_scope(&mut self, label: impl Into<String>) {
        let id = self.trace.scopes.len();
        let parent = self.scope_ids.last().copied();
        self.trace.scopes.push(ScopeTrace {
            id,
            parent,
            label: label.into(),
        });
        self.scope_stack.push(HashMap::new());
        self.scope_ids.push(id);
    }

    fn leave_scope(&mut self) {
        self.scope_stack.pop();
        self.scope_ids.pop();
    }

    fn current_scope_id(&self) -> usize {
        *self.scope_ids.last().expect("semantic scope exists")
    }

    fn error(&mut self, span: Span, code: impl Into<String>, message: impl Into<String>) {
        self.trace
            .diagnostics
            .push(Diagnostic::error(code, message, span));
    }
}

fn parse_type(label: &str) -> Type {
    match label {
        "i32" => Type::I32,
        "bool" => Type::Bool,
        "char" => Type::Char,
        "str" => Type::Str,
        "()" => Type::Unit,
        _ => Type::Unknown,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rt_lexer::Lexer;
    use rt_parser::parse_tokens;

    fn analyze_source(source: &str) -> SemanticTrace {
        let lexed = Lexer::new(source).lex();
        let parsed = parse_tokens(&lexed.tokens);
        analyze(&parsed.root)
    }

    #[test]
    fn records_symbols_for_basic_program() {
        let trace = analyze_source("fn main() -> i32 { let x: i32 = 42; return x; }");
        assert!(trace.diagnostics.is_empty());
        assert!(trace.symbols.iter().any(|symbol| symbol.name == "main"));
        assert!(trace.symbols.iter().any(|symbol| symbol.name == "x"));
    }

    #[test]
    fn reports_undefined_variable() {
        let trace = analyze_source("fn main() -> i32 { return x; }");
        assert_eq!(trace.diagnostics[0].code, "E2003");
    }

    #[test]
    fn reports_type_mismatch() {
        let trace = analyze_source("fn main() -> i32 { let x: bool = 1; return 0; }");
        assert_eq!(trace.diagnostics[0].code, "E2005");
    }

    #[test]
    fn checks_function_call_arguments() {
        let trace = analyze_source(
            "fn add(a: i32, b: i32) -> i32 { return a + b; } fn main() -> i32 { return add(1, true); }",
        );
        assert!(trace
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "E2016"));
    }
}
