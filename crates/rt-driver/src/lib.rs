//! Compile pipeline orchestration and stage rendering.

use rt_ast::AstNode;
use rt_common::{escape_json, Diagnostic, SourceFile, Span};
use rt_lexer::{Lexer, Token};
use rt_parser::parse_tokens;
use rt_semantic::{analyze, ExpressionType, ScopeTrace, SemanticTrace, SymbolTrace};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EmitStage {
    Tokens,
    Ast,
    Semantic,
    LlvmIr,
    TeachingIr,
    All,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OutputFormat {
    Text,
    Json,
}

#[derive(Clone, Debug)]
pub struct CompileOptions {
    pub emit: EmitStage,
    pub format: OutputFormat,
}

impl Default for CompileOptions {
    fn default() -> Self {
        Self {
            emit: EmitStage::Tokens,
            format: OutputFormat::Text,
        }
    }
}

#[derive(Clone, Debug)]
pub struct CompileTrace {
    pub source_name: String,
    pub tokens: Vec<Token>,
    pub diagnostics: Vec<Diagnostic>,
}

pub fn compile_source(source: SourceFile, options: CompileOptions) -> String {
    let lexed = Lexer::new(&source.text).lex();
    let parsed = parse_tokens(&lexed.tokens);
    let semantic = analyze(&parsed.root);
    let llvm_ir = rt_codegen::generate_llvm_ir(&parsed.root);
    let teaching_ir = rt_ir_view::project_from_llvm(&llvm_ir);

    let mut diagnostics = Vec::new();
    diagnostics.extend(lexed.diagnostics.clone());
    diagnostics.extend(parsed.diagnostics);
    diagnostics.extend(semantic.diagnostics.clone());

    let trace = CompileTrace {
        source_name: source.name,
        tokens: lexed.tokens,
        diagnostics,
    };

    match options.format {
        OutputFormat::Text => match options.emit {
            EmitStage::Tokens => render_tokens_text(&trace.tokens),
            EmitStage::Ast => format!("{:#?}\n", parsed.root),
            EmitStage::Semantic => format!("{:#?}\n", semantic),
            EmitStage::LlvmIr => llvm_ir,
            EmitStage::TeachingIr => teaching_ir.text,
            EmitStage::All => {
                format!(
                    "Tokens:\n{}\nAST:\n{:#?}\nSemantic:\n{:#?}\nLLVM IR:\n{}\nTeaching IR:\n{}\n",
                    render_tokens_text(&trace.tokens),
                    parsed.root,
                    semantic,
                    llvm_ir,
                    teaching_ir.text
                )
            }
        },
        OutputFormat::Json => render_json(&trace, &parsed.root, &semantic, options.emit),
    }
}

fn render_tokens_text(tokens: &[Token]) -> String {
    let mut out = String::new();
    for token in tokens {
        out.push_str(&format!(
            "{:<12} {:<12} line {}, column {}\n",
            token.kind.as_str(),
            token.lexeme,
            token.span.line,
            token.span.column
        ));
    }
    out
}

fn render_json(
    trace: &CompileTrace,
    ast: &AstNode,
    semantic: &SemanticTrace,
    emit: EmitStage,
) -> String {
    let include_tokens = matches!(emit, EmitStage::Tokens | EmitStage::All);
    let include_ast = matches!(emit, EmitStage::Ast | EmitStage::All);
    let include_semantic = matches!(emit, EmitStage::Semantic | EmitStage::All);
    let tokens = if include_tokens {
        render_tokens_json(&trace.tokens)
    } else {
        "[]".to_string()
    };
    let parser = if include_ast {
        format!("{{ \"ast\": {} }}", render_ast_json(ast))
    } else {
        "{}".to_string()
    };
    let semantic_stage = if include_semantic {
        render_semantic_json(semantic)
    } else {
        "{}".to_string()
    };
    format!(
        "{{\n  \"version\": \"0.1.0\",\n  \"source_name\": \"{}\",\n  \"stages\": {{\n    \"lexer\": {{ \"tokens\": {} }},\n    \"parser\": {},\n    \"semantic\": {},\n    \"llvm_ir\": {{}},\n    \"teaching_ir\": {{}}\n  }},\n  \"diagnostics\": {}\n}}\n",
        escape_json(&trace.source_name),
        tokens,
        parser,
        semantic_stage,
        render_diagnostics_json(&trace.diagnostics)
    )
}

fn render_semantic_json(semantic: &SemanticTrace) -> String {
    format!(
        "{{ \"scopes\": {}, \"symbols\": {}, \"expression_types\": {} }}",
        render_scopes_json(&semantic.scopes),
        render_symbols_json(&semantic.symbols),
        render_expression_types_json(&semantic.expression_types)
    )
}

fn render_scopes_json(scopes: &[ScopeTrace]) -> String {
    let parts = scopes
        .iter()
        .map(|scope| {
            let parent = scope
                .parent
                .map(|id| id.to_string())
                .unwrap_or_else(|| "null".to_string());
            format!(
                "{{ \"id\": {}, \"parent\": {}, \"label\": \"{}\" }}",
                scope.id,
                parent,
                escape_json(&scope.label)
            )
        })
        .collect::<Vec<_>>();
    format!("[{}]", parts.join(", "))
}

fn render_symbols_json(symbols: &[SymbolTrace]) -> String {
    let parts = symbols
        .iter()
        .map(|symbol| {
            format!(
                "{{ \"name\": \"{}\", \"kind\": \"{}\", \"type\": \"{}\", \"mutable\": {}, \"scope_id\": {}, \"span\": {} }}",
                escape_json(&symbol.name),
                symbol.kind.as_str(),
                symbol.ty.as_str(),
                symbol.mutable,
                symbol.scope_id,
                render_span_json(symbol.span)
            )
        })
        .collect::<Vec<_>>();
    format!("[{}]", parts.join(", "))
}

fn render_expression_types_json(expression_types: &[ExpressionType]) -> String {
    let parts = expression_types
        .iter()
        .map(|expr| {
            format!(
                "{{ \"node_id\": {}, \"type\": \"{}\", \"span\": {} }}",
                expr.node_id,
                expr.ty.as_str(),
                render_span_json(expr.span)
            )
        })
        .collect::<Vec<_>>();
    format!("[{}]", parts.join(", "))
}

fn render_ast_json(node: &AstNode) -> String {
    let children = node
        .children
        .iter()
        .map(render_ast_json)
        .collect::<Vec<_>>()
        .join(", ");
    format!(
        "{{ \"id\": {}, \"kind\": \"{}\", \"label\": \"{}\", \"span\": {}, \"children\": [{}] }}",
        node.id,
        escape_json(&node.kind),
        escape_json(&node.label),
        render_span_json(node.span),
        children
    )
}

fn render_tokens_json(tokens: &[Token]) -> String {
    let mut parts = Vec::with_capacity(tokens.len());
    for token in tokens {
        parts.push(format!(
            "{{ \"kind\": \"{}\", \"lexeme\": \"{}\", \"span\": {} }}",
            token.kind.as_str(),
            escape_json(&token.lexeme),
            render_span_json(token.span)
        ));
    }
    format!("[{}]", parts.join(", "))
}

fn render_diagnostics_json(diagnostics: &[Diagnostic]) -> String {
    let mut parts = Vec::with_capacity(diagnostics.len());
    for diagnostic in diagnostics {
        let span = diagnostic
            .span
            .map(render_span_json)
            .unwrap_or_else(|| "null".to_string());
        parts.push(format!(
            "{{ \"level\": \"{}\", \"code\": \"{}\", \"message\": \"{}\", \"span\": {} }}",
            diagnostic.level.as_str(),
            escape_json(&diagnostic.code),
            escape_json(&diagnostic.message),
            span
        ));
    }
    format!("[{}]", parts.join(", "))
}

fn render_span_json(span: Span) -> String {
    format!(
        "{{ \"start\": {}, \"end\": {}, \"line\": {}, \"column\": {} }}",
        span.start, span.end, span.line, span.column
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn emits_tokens_as_json() {
        let source = SourceFile::new("basic.rs", "fn main() { return 42; }");
        let output = compile_source(
            source,
            CompileOptions {
                emit: EmitStage::Tokens,
                format: OutputFormat::Json,
            },
        );
        assert!(output.contains("\"lexer\""));
        assert!(output.contains("\"Fn\""));
    }
}
