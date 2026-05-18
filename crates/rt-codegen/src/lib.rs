//! LLVM IR text generation for the first executable teaching subset.

use std::collections::HashMap;

use rt_ast::AstNode;

pub fn generate_llvm_ir(root: &AstNode) -> String {
    Codegen::new(root).generate_module()
}

#[derive(Clone, Debug)]
struct FunctionSig {
    return_type: LlvmType,
}

#[derive(Clone, Debug)]
struct Local {
    ptr: String,
    ty: LlvmType,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct Value {
    repr: String,
    ty: LlvmType,
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum LlvmType {
    I32,
    I1,
    I8,
    Ptr,
    Void,
}

impl LlvmType {
    fn as_str(&self) -> &'static str {
        match self {
            LlvmType::I32 => "i32",
            LlvmType::I1 => "i1",
            LlvmType::I8 => "i8",
            LlvmType::Ptr => "ptr",
            LlvmType::Void => "void",
        }
    }
}

struct Codegen<'a> {
    root: &'a AstNode,
    functions: HashMap<String, FunctionSig>,
}

struct FunctionCodegen<'a> {
    function: &'a AstNode,
    signatures: &'a HashMap<String, FunctionSig>,
    instructions: Vec<String>,
    locals: HashMap<String, Local>,
    temp_index: usize,
    terminated: bool,
}

impl<'a> Codegen<'a> {
    fn new(root: &'a AstNode) -> Self {
        let mut functions = HashMap::new();
        for function in root.children.iter().filter(|node| node.kind == "Function") {
            let return_type = function
                .children
                .iter()
                .find(|child| child.kind == "ReturnType")
                .map(|node| llvm_type(&node.label))
                .unwrap_or(LlvmType::Void);
            functions.insert(function.label.clone(), FunctionSig { return_type });
        }
        Self { root, functions }
    }

    fn generate_module(&self) -> String {
        let mut out = String::new();
        for function in self
            .root
            .children
            .iter()
            .filter(|node| node.kind == "Function")
        {
            out.push_str(&FunctionCodegen::new(function, &self.functions).generate());
            out.push('\n');
        }
        out
    }
}

impl<'a> FunctionCodegen<'a> {
    fn new(function: &'a AstNode, signatures: &'a HashMap<String, FunctionSig>) -> Self {
        Self {
            function,
            signatures,
            instructions: Vec::new(),
            locals: HashMap::new(),
            temp_index: 0,
            terminated: false,
        }
    }

    fn generate(mut self) -> String {
        let return_type = self
            .function
            .children
            .iter()
            .find(|child| child.kind == "ReturnType")
            .map(|node| llvm_type(&node.label))
            .unwrap_or(LlvmType::Void);
        let params = self
            .function
            .children
            .iter()
            .filter(|child| child.kind == "Param")
            .map(|param| {
                let ty = param
                    .children
                    .first()
                    .map(|node| llvm_type(&node.label))
                    .unwrap_or(LlvmType::I32);
                (param.label.clone(), ty)
            })
            .collect::<Vec<_>>();

        for (name, ty) in &params {
            let ptr = self.next_named(&format!("{}.addr", name));
            self.emit(format!("  {} = alloca {}", ptr, ty.as_str()));
            self.emit(format!(
                "  store {} %{}, ptr {}",
                ty.as_str(),
                sanitize(name),
                ptr
            ));
            self.locals.insert(
                name.clone(),
                Local {
                    ptr,
                    ty: ty.clone(),
                },
            );
        }

        if let Some(body) = self
            .function
            .children
            .iter()
            .find(|child| child.kind == "Block")
        {
            self.gen_block(body);
        }

        if !self.terminated {
            match return_type {
                LlvmType::Void => self.emit("  ret void"),
                LlvmType::I1 => self.emit("  ret i1 0"),
                LlvmType::I8 => self.emit("  ret i8 0"),
                LlvmType::Ptr => self.emit("  ret ptr null"),
                LlvmType::I32 => self.emit("  ret i32 0"),
            }
        }

        let params_text = params
            .iter()
            .map(|(name, ty)| format!("{} %{}", ty.as_str(), sanitize(name)))
            .collect::<Vec<_>>()
            .join(", ");

        format!(
            "define {} @{}({}) {{\nentry:\n{}\n}}\n",
            return_type.as_str(),
            sanitize(&self.function.label),
            params_text,
            self.instructions.join("\n")
        )
    }

    fn gen_block(&mut self, block: &AstNode) {
        for stmt in &block.children {
            if self.terminated {
                break;
            }
            self.gen_stmt(stmt);
        }
    }

    fn gen_stmt(&mut self, stmt: &AstNode) {
        match stmt.kind.as_str() {
            "LetStmt" => self.gen_let(stmt),
            "ReturnStmt" => self.gen_return(stmt),
            "ExprStmt" => {
                if let Some(expr) = stmt.children.first() {
                    self.gen_expr(expr);
                }
            }
            _ => {
                self.emit(format!("  ; unsupported statement {}", stmt.kind));
            }
        }
    }

    fn gen_let(&mut self, stmt: &AstNode) {
        let name = stmt.label.trim_start_matches("mut ").to_string();
        let annotated_ty = stmt
            .children
            .iter()
            .find(|child| child.kind == "Type")
            .map(|node| llvm_type(&node.label));
        let initializer = stmt.children.iter().find(|child| child.kind != "Type");
        let value = initializer
            .map(|expr| self.gen_expr(expr))
            .unwrap_or_else(|| zero_value(annotated_ty.clone().unwrap_or(LlvmType::I32)));
        let ty = annotated_ty.unwrap_or_else(|| value.ty.clone());
        let ptr = self.next_named(&format!("{}.addr", name));
        self.emit(format!("  {} = alloca {}", ptr, ty.as_str()));
        let casted = self.cast_if_needed(value, &ty);
        self.emit(format!(
            "  store {} {}, ptr {}",
            ty.as_str(),
            casted.repr,
            ptr
        ));
        self.locals.insert(name, Local { ptr, ty });
    }

    fn gen_return(&mut self, stmt: &AstNode) {
        if let Some(expr) = stmt.children.first() {
            let value = self.gen_expr(expr);
            self.emit(format!("  ret {} {}", value.ty.as_str(), value.repr));
        } else {
            self.emit("  ret void");
        }
        self.terminated = true;
    }

    fn gen_expr(&mut self, expr: &AstNode) -> Value {
        match expr.kind.as_str() {
            "IntLiteral" => Value {
                repr: expr.label.clone(),
                ty: LlvmType::I32,
            },
            "BoolLiteral" => Value {
                repr: if expr.label == "true" { "1" } else { "0" }.to_string(),
                ty: LlvmType::I1,
            },
            "CharLiteral" => Value {
                repr: char_literal_to_i8(&expr.label).to_string(),
                ty: LlvmType::I8,
            },
            "StringLiteral" => Value {
                repr: "null".to_string(),
                ty: LlvmType::Ptr,
            },
            "IdentExpr" => self.gen_ident(expr),
            "ParenExpr" => expr
                .children
                .first()
                .map(|child| self.gen_expr(child))
                .unwrap_or_else(|| zero_value(LlvmType::I32)),
            "UnaryExpr" => self.gen_unary(expr),
            "BinaryExpr" => self.gen_binary(expr),
            "CallExpr" => self.gen_call(expr),
            _ => zero_value(LlvmType::I32),
        }
    }

    fn gen_ident(&mut self, expr: &AstNode) -> Value {
        let Some(local) = self.locals.get(&expr.label).cloned() else {
            return zero_value(LlvmType::I32);
        };
        let temp = self.next_temp();
        self.emit(format!(
            "  {} = load {}, ptr {}",
            temp,
            local.ty.as_str(),
            local.ptr
        ));
        Value {
            repr: temp,
            ty: local.ty,
        }
    }

    fn gen_unary(&mut self, expr: &AstNode) -> Value {
        let value = expr
            .children
            .first()
            .map(|child| self.gen_expr(child))
            .unwrap_or_else(|| zero_value(LlvmType::I32));
        match expr.label.as_str() {
            "-" => {
                let temp = self.next_temp();
                self.emit(format!("  {} = sub i32 0, {}", temp, value.repr));
                Value {
                    repr: temp,
                    ty: LlvmType::I32,
                }
            }
            "!" => {
                let temp = self.next_temp();
                self.emit(format!("  {} = xor i1 {}, true", temp, value.repr));
                Value {
                    repr: temp,
                    ty: LlvmType::I1,
                }
            }
            _ => value,
        }
    }

    fn gen_binary(&mut self, expr: &AstNode) -> Value {
        if expr.label == "=" {
            return self.gen_assignment(expr);
        }

        let lhs = expr
            .children
            .first()
            .map(|child| self.gen_expr(child))
            .unwrap_or_else(|| zero_value(LlvmType::I32));
        let rhs = expr
            .children
            .get(1)
            .map(|child| self.gen_expr(child))
            .unwrap_or_else(|| zero_value(lhs.ty.clone()));

        match expr.label.as_str() {
            "+" | "-" | "*" | "/" | "%" => self.gen_i32_binary(expr.label.as_str(), lhs, rhs),
            "==" | "!=" | "<" | "<=" | ">" | ">=" => {
                self.gen_i32_compare(expr.label.as_str(), lhs, rhs)
            }
            "&&" | "||" => self.gen_bool_binary(expr.label.as_str(), lhs, rhs),
            _ => lhs,
        }
    }

    fn gen_assignment(&mut self, expr: &AstNode) -> Value {
        let Some(target) = expr.children.first() else {
            return zero_value(LlvmType::I32);
        };
        let Some(rhs) = expr.children.get(1) else {
            return zero_value(LlvmType::I32);
        };
        let value = self.gen_expr(rhs);
        if target.kind != "IdentExpr" {
            return value;
        }
        if let Some(local) = self.locals.get(&target.label).cloned() {
            let casted = self.cast_if_needed(value, &local.ty);
            self.emit(format!(
                "  store {} {}, ptr {}",
                local.ty.as_str(),
                casted.repr,
                local.ptr
            ));
            casted
        } else {
            value
        }
    }

    fn gen_i32_binary(&mut self, op: &str, lhs: Value, rhs: Value) -> Value {
        let opcode = match op {
            "+" => "add",
            "-" => "sub",
            "*" => "mul",
            "/" => "sdiv",
            "%" => "srem",
            _ => "add",
        };
        let temp = self.next_temp();
        self.emit(format!(
            "  {} = {} i32 {}, {}",
            temp, opcode, lhs.repr, rhs.repr
        ));
        Value {
            repr: temp,
            ty: LlvmType::I32,
        }
    }

    fn gen_i32_compare(&mut self, op: &str, lhs: Value, rhs: Value) -> Value {
        let predicate = match op {
            "==" => "eq",
            "!=" => "ne",
            "<" => "slt",
            "<=" => "sle",
            ">" => "sgt",
            ">=" => "sge",
            _ => "eq",
        };
        let temp = self.next_temp();
        self.emit(format!(
            "  {} = icmp {} i32 {}, {}",
            temp, predicate, lhs.repr, rhs.repr
        ));
        Value {
            repr: temp,
            ty: LlvmType::I1,
        }
    }

    fn gen_bool_binary(&mut self, op: &str, lhs: Value, rhs: Value) -> Value {
        let opcode = if op == "&&" { "and" } else { "or" };
        let temp = self.next_temp();
        self.emit(format!(
            "  {} = {} i1 {}, {}",
            temp, opcode, lhs.repr, rhs.repr
        ));
        Value {
            repr: temp,
            ty: LlvmType::I1,
        }
    }

    fn gen_call(&mut self, expr: &AstNode) -> Value {
        let Some(callee) = expr.children.first() else {
            return zero_value(LlvmType::I32);
        };
        let args = expr
            .children
            .iter()
            .skip(1)
            .map(|arg| self.gen_expr(arg))
            .collect::<Vec<_>>();
        let return_type = self
            .signatures
            .get(&callee.label)
            .map(|sig| sig.return_type.clone())
            .unwrap_or(LlvmType::I32);
        let args_text = args
            .iter()
            .map(|arg| format!("{} {}", arg.ty.as_str(), arg.repr))
            .collect::<Vec<_>>()
            .join(", ");

        if return_type == LlvmType::Void {
            self.emit(format!(
                "  call void @{}({})",
                sanitize(&callee.label),
                args_text
            ));
            zero_value(LlvmType::Void)
        } else {
            let temp = self.next_temp();
            self.emit(format!(
                "  {} = call {} @{}({})",
                temp,
                return_type.as_str(),
                sanitize(&callee.label),
                args_text
            ));
            Value {
                repr: temp,
                ty: return_type,
            }
        }
    }

    fn cast_if_needed(&mut self, value: Value, expected: &LlvmType) -> Value {
        if &value.ty == expected {
            return value;
        }
        match (&value.ty, expected) {
            (LlvmType::I1, LlvmType::I32) => {
                let temp = self.next_temp();
                self.emit(format!("  {} = zext i1 {} to i32", temp, value.repr));
                Value {
                    repr: temp,
                    ty: LlvmType::I32,
                }
            }
            (LlvmType::I32, LlvmType::I1) => {
                let temp = self.next_temp();
                self.emit(format!("  {} = icmp ne i32 {}, 0", temp, value.repr));
                Value {
                    repr: temp,
                    ty: LlvmType::I1,
                }
            }
            _ => value,
        }
    }

    fn emit(&mut self, instruction: impl Into<String>) {
        self.instructions.push(instruction.into());
    }

    fn next_temp(&mut self) -> String {
        let temp = format!("%t{}", self.temp_index);
        self.temp_index += 1;
        temp
    }

    fn next_named(&mut self, base: &str) -> String {
        let name = format!("%{}", sanitize(base));
        if !self.locals.values().any(|local| local.ptr == name) {
            return name;
        }
        self.next_temp()
    }
}

fn llvm_type(label: &str) -> LlvmType {
    match label {
        "i32" => LlvmType::I32,
        "bool" => LlvmType::I1,
        "char" => LlvmType::I8,
        "str" => LlvmType::Ptr,
        "()" => LlvmType::Void,
        _ => LlvmType::I32,
    }
}

fn zero_value(ty: LlvmType) -> Value {
    let repr = match ty {
        LlvmType::I1 | LlvmType::I8 | LlvmType::I32 => "0",
        LlvmType::Ptr => "null",
        LlvmType::Void => "",
    };
    Value {
        repr: repr.to_string(),
        ty,
    }
}

fn char_literal_to_i8(label: &str) -> u32 {
    let body = label.trim_matches('\'');
    match body {
        "\\n" => 10,
        "\\r" => 13,
        "\\t" => 9,
        "\\0" => 0,
        _ => body.chars().next().unwrap_or('\0') as u32,
    }
}

fn sanitize(name: &str) -> String {
    name.chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '_' || ch == '.' {
                ch
            } else {
                '_'
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use rt_lexer::Lexer;
    use rt_parser::parse_tokens;

    fn compile(source: &str) -> String {
        let lexed = Lexer::new(source).lex();
        let parsed = parse_tokens(&lexed.tokens);
        generate_llvm_ir(&parsed.root)
    }

    #[test]
    fn emits_return_literal() {
        let ir = compile("fn main() -> i32 { return 42; }");
        assert!(ir.contains("define i32 @main()"));
        assert!(ir.contains("ret i32 42"));
    }

    #[test]
    fn emits_let_and_return_identifier() {
        let ir = compile("fn main() -> i32 { let x: i32 = 40 + 2; return x; }");
        assert!(ir.contains("alloca i32"));
        assert!(ir.contains("add i32 40, 2"));
        assert!(ir.contains("load i32"));
    }
}
