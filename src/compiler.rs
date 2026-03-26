// ============================================================
// LIAB Language — Bytecode Compiler (v0.3)
// ============================================================
//
// The compiler walks the AST and produces a flat sequence of
// bytecode instructions stored in a `Chunk`.  This is the
// bridge between the parser (which produces trees) and the
// VM (which executes linear instructions).
//
// Key concepts:
//   • `Instruction` — one operation the VM can perform
//   • `Chunk`       — a sequence of instructions + constant pool
//   • `Compiler`    — walks the AST, emits instructions
//
// The compiler uses a **patch** strategy for jumps: it emits
// a placeholder jump, records where it is, then overwrites
// the target offset once the destination is known.
// ============================================================

use crate::ast::{BinOp, Expr, Stmt};
use crate::value::Value;

// ----------------------------------------------------------
// Instruction enum
// ----------------------------------------------------------
//
// Each variant maps directly to a VM operation.  The VM
// executes instructions sequentially unless a jump changes
// the instruction pointer.
//
// Stack effects are documented for each instruction:
//   `[before] → [after]`

/// A single bytecode instruction.
#[derive(Debug, Clone, PartialEq)]
pub enum Instruction {
    // ---- Constants & Literals ----
    /// Push a value from the constant pool onto the stack.
    /// `[] → [constants[index]]`
    Constant(usize),

    // ---- Arithmetic ----
    /// Pop two values, push their sum.
    /// `[a, b] → [a + b]`
    Add,
    /// `[a, b] → [a - b]`
    Sub,
    /// `[a, b] → [a * b]`
    Mul,
    /// `[a, b] → [a / b]`
    Div,

    // ---- Comparison ----
    /// `[a, b] → [a == b]`
    Eq,
    /// `[a, b] → [a != b]`
    Neq,
    /// `[a, b] → [a > b]`
    Gt,
    /// `[a, b] → [a < b]`
    Lt,
    /// `[a, b] → [a >= b]`
    Gte,
    /// `[a, b] → [a <= b]`
    Lte,

    // ---- Stack manipulation ----
    /// Pop and discard top of stack.
    /// `[val] → []`
    Pop,
    /// Print and pop top of stack.
    /// `[val] → []`
    Print,

    // ---- Global variables ----
    /// Define a new global variable. Pops the value.
    /// `[val] → []`
    DefineGlobal(usize),
    /// Push the value of a global variable.
    /// `[] → [val]`
    GetGlobal(usize),
    /// Pop a value and store it in an existing global.
    /// `[val] → []`
    SetGlobal(usize),

    /// Get a property (v0.5)
    GetProperty(usize),

    /// Get dynamic index (v0.6)
    GetIndex,

    // ---- Local variables (function params) ----
    /// Push a local variable by stack slot index.
    /// The slot is relative to the current call frame's base.
    /// `[] → [val]`
    GetLocal(usize),
    /// Set a local variable by stack slot index.
    /// `[val] → []`
    SetLocal(usize),

    // ---- Control flow ----
    /// Unconditional jump to an absolute instruction index.
    Jump(usize),
    /// Pop the top of stack; jump if the value is falsy.
    /// `[cond] → []`
    JumpIfFalse(usize),

    // ---- Functions ----
    /// Call a function with `arg_count` arguments.
    /// The function value is on the stack below the arguments.
    /// `[fn, arg0, arg1, ...] → [return_value]`
    Call(usize),
    /// Return from the current function.
    /// Pops the return value from the top of stack.
    Return,
    
    // ---- Module System ----
    Love(usize),
    Export(usize),
}

// ----------------------------------------------------------
// Chunk — a compiled unit of bytecode
// ----------------------------------------------------------
/// A `Chunk` holds the compiled bytecode for a single
/// function (or the top-level script).
#[derive(Debug, Clone, PartialEq)]
pub struct Chunk {
    /// The bytecode instructions.
    pub code: Vec<Instruction>,
    /// The constant pool — literals used by `Constant(idx)`.
    pub constants: Vec<Value>,
}

impl Chunk {
    pub fn new() -> Self {
        Self {
            code: Vec::new(),
            constants: Vec::new(),
        }
    }

    /// Append an instruction and return its index in `code`.
    pub fn emit(&mut self, instr: Instruction) -> usize {
        let idx = self.code.len();
        self.code.push(instr);
        idx
    }

    /// Add a constant to the pool, returning its index.
    pub fn add_constant(&mut self, value: Value) -> usize {
        self.constants.push(value);
        self.constants.len() - 1
    }

    /// Emit a placeholder jump and return its index for patching.
    pub fn emit_jump(&mut self, instr: Instruction) -> usize {
        self.emit(instr)
    }

    /// Patch a previously emitted jump to target the current
    /// instruction index.
    pub fn patch_jump(&mut self, jump_idx: usize) {
        let target = self.code.len();
        match &mut self.code[jump_idx] {
            Instruction::Jump(ref mut addr) => *addr = target,
            Instruction::JumpIfFalse(ref mut addr) => *addr = target,
            _ => panic!("Tried to patch a non-jump instruction"),
        }
    }

    /// Return the current instruction count (useful for
    /// loop start addresses).
    pub fn current_offset(&self) -> usize {
        self.code.len()
    }
}

impl Default for Chunk {
    fn default() -> Self {
        Self::new()
    }
}

// ----------------------------------------------------------
// Compiler
// ----------------------------------------------------------
/// Compiles an AST into bytecode.
///
/// The compiler processes global code into a "script" chunk.
/// Each function definition produces its own chunk, stored
/// as a `Value::CompiledFunction` in the constant pool.
pub struct Compiler {
    /// The chunk currently being compiled.
    chunk: Chunk,
    /// Originating module path scoping
    module_path: Option<String>,
}

impl Compiler {
    pub fn new() -> Self {
        Self {
            chunk: Chunk::new(),
            module_path: None,
        }
    }

    pub fn with_module(path: Option<String>) -> Self {
        Self {
            chunk: Chunk::new(),
            module_path: path,
        }
    }

    /// Compile a full program (list of statements) into a Chunk.
    pub fn compile(mut self, stmts: Vec<Stmt>) -> Result<Chunk, crate::error::LiabError> {
        for stmt in stmts {
            self.compile_stmt(stmt)?;
        }
        // Implicit return at end of script
        let idx = self.chunk.add_constant(Value::Number(0.0));
        self.chunk.emit(Instruction::Constant(idx));
        self.chunk.emit(Instruction::Return);
        Ok(self.chunk)
    }

    // ----------------------------------------------------------
    // Statement compilation
    // ----------------------------------------------------------

    fn compile_stmt(&mut self, stmt: Stmt) -> Result<(), crate::error::LiabError> {
        match stmt {
            // ---- let <name> = <expr>; ----
            Stmt::Let { name, value } => {
                self.compile_expr(value)?;
                let idx = self.chunk.add_constant(Value::String(name));
                self.chunk.emit(Instruction::DefineGlobal(idx));
            }

            // ---- <name> = <expr>; ----
            Stmt::Assignment { name, value } => {
                self.compile_expr(value)?;
                let idx = self.chunk.add_constant(Value::String(name));
                self.chunk.emit(Instruction::SetGlobal(idx));
            }

            // ---- print <expr>; ----
            Stmt::Print(expr) => {
                self.compile_expr(expr)?;
                self.chunk.emit(Instruction::Print);
            }

            // ---- <expr>; ----
            Stmt::Expression(expr) => {
                self.compile_expr(expr)?;
                self.chunk.emit(Instruction::Pop);
            }

            // ---- fn <name>(<params>) { <body> } ----
            Stmt::FunctionDef { name, params, body } => {
                // Compile the function body into its own chunk.
                let func_chunk = self.compile_function(&params, body)?;
                let func_val = Value::CompiledFunction {
                    name: name.clone(),
                    arity: params.len(),
                    chunk: func_chunk,
                    module_path: self.module_path.clone(),
                };
                let idx = self.chunk.add_constant(func_val);
                self.chunk.emit(Instruction::Constant(idx));
                let name_idx = self.chunk.add_constant(Value::String(name));
                self.chunk.emit(Instruction::DefineGlobal(name_idx));
            }

            // ---- return <expr>; ----
            Stmt::Return(expr) => {
                self.compile_expr(expr)?;
                self.chunk.emit(Instruction::Return);
            }

            // ---- if <cond> { <then> } [else { <else> }] ----
            //
            // Bytecode layout:
            //   [condition code]
            //   JumpIfFalse → else_start (or end)
            //   [then body]
            //   Jump → end
            //   [else body]       ← else_start
            //   ...               ← end
            Stmt::If { condition, then_body, else_body } => {
                self.compile_expr(condition)?;

                // Emit conditional jump (placeholder target).
                let jump_to_else = self.chunk.emit_jump(
                    Instruction::JumpIfFalse(0xFFFF)
                );

                // Compile then-body.
                for stmt in then_body {
                    self.compile_stmt(stmt)?;
                }

                if let Some(else_stmts) = else_body {
                    // Jump over the else-body at end of then-body.
                    let jump_over_else = self.chunk.emit_jump(
                        Instruction::Jump(0xFFFF)
                    );

                    // Patch: JumpIfFalse lands here (else start).
                    self.chunk.patch_jump(jump_to_else);

                    // Compile else-body.
                    for stmt in else_stmts {
                        self.compile_stmt(stmt)?;
                    }

                    // Patch: Jump lands here (end).
                    self.chunk.patch_jump(jump_over_else);
                } else {
                    // No else — just patch the conditional jump.
                    self.chunk.patch_jump(jump_to_else);
                }
            }

            // ---- while <cond> { <body> } ----
            //
            // Bytecode layout:
            //   loop_start:
            //     [condition code]
            //     JumpIfFalse → loop_end
            //     [body]
            //     Jump → loop_start
            //   loop_end:
            Stmt::While { condition, body } => {
                let loop_start = self.chunk.current_offset();

                // Compile condition.
                self.compile_expr(condition)?;

                // Emit conditional jump out of the loop.
                let exit_jump = self.chunk.emit_jump(
                    Instruction::JumpIfFalse(0xFFFF)
                );

                // Compile body.
                for stmt in body {
                    self.compile_stmt(stmt)?;
                }

                // Jump back to loop start.
                self.chunk.emit(Instruction::Jump(loop_start));

                // Patch: JumpIfFalse lands here (loop end).
                self.chunk.patch_jump(exit_jump);
            }

            // ---- { <stmts> } ----
            Stmt::Block(stmts) => {
                for stmt in stmts {
                    self.compile_stmt(stmt)?;
                }
            }

            // ---- love "script"; ----
            Stmt::Love(path) => {
                let idx = self.chunk.add_constant(Value::String(path));
                self.chunk.emit(Instruction::Love(idx));
            }

            // ---- export a; ----
            Stmt::Export(name) => {
                let idx = self.chunk.add_constant(Value::String(name));
                self.chunk.emit(Instruction::Export(idx));
            }
        }

        Ok(())
    }

    // ----------------------------------------------------------
    // Expression compilation
    // ----------------------------------------------------------

    fn compile_expr(&mut self, expr: Expr) -> Result<(), crate::error::LiabError> {
        match expr {
            // ---- literal number ----
            Expr::Number(n) => {
                let idx = self.chunk.add_constant(Value::Number(n));
                self.chunk.emit(Instruction::Constant(idx));
            }

            // ---- literal boolean ----
            Expr::Boolean(b) => {
                let idx = self.chunk.add_constant(Value::Boolean(b));
                self.chunk.emit(Instruction::Constant(idx));
            }

            // ---- literal string ----
            Expr::String(s) => {
                let idx = self.chunk.add_constant(Value::String(s));
                self.chunk.emit(Instruction::Constant(idx));
            }

            // ---- variable reference ----
            Expr::Identifier(name) => {
                let idx = self.chunk.add_constant(Value::String(name));
                self.chunk.emit(Instruction::GetGlobal(idx));
            }

            // ---- member access ----
            Expr::Member { object, property } => {
                self.compile_expr(*object)?;
                let idx = self.chunk.add_constant(Value::String(property));
                self.chunk.emit(Instruction::GetProperty(idx));
            }

            Expr::Index { object, index } => {
                self.compile_expr(*object)?;
                self.compile_expr(*index)?;
                self.chunk.emit(Instruction::GetIndex);
            }

            // ---- binary operation ----
            Expr::BinaryOp { left, op, right } => {
                self.compile_expr(*left)?;
                self.compile_expr(*right)?;
                let instr = match op {
                    BinOp::Add => Instruction::Add,
                    BinOp::Sub => Instruction::Sub,
                    BinOp::Mul => Instruction::Mul,
                    BinOp::Div => Instruction::Div,
                    BinOp::Eq  => Instruction::Eq,
                    BinOp::Neq => Instruction::Neq,
                    BinOp::Gt  => Instruction::Gt,
                    BinOp::Lt  => Instruction::Lt,
                    BinOp::Gte => Instruction::Gte,
                    BinOp::Lte => Instruction::Lte,
                };
                self.chunk.emit(instr);
            }

            // ---- function call ----
            Expr::FunctionCall { callee, args } => {
                // Determine the function value
                self.compile_expr(*callee)?;

                // Compile each argument, pushing them onto the stack
                for arg in args.iter() {
                    self.compile_expr(arg.clone())?;
                }

                // Call instruction with arity
                self.chunk.emit(Instruction::Call(args.len()));
            }
        }

        Ok(())
    }

    // ----------------------------------------------------------
    // Function compilation
    // ----------------------------------------------------------

    /// Compile a function body into its own Chunk.
    ///
    /// Parameters become local variables at slots 0..n on the
    /// VM's value stack (relative to the call frame's base).
    /// Inside the function body, identifiers matching parameter
    /// names are compiled as `GetLocal(slot)` / `SetLocal(slot)`.
    fn compile_function(
        &mut self,
        params: &[String],
        body: Vec<Stmt>,
    ) -> Result<Chunk, crate::error::LiabError> {
        let mut func_compiler = FunctionCompiler::new(params, self.module_path.clone());
        for stmt in body {
            func_compiler.compile_stmt(stmt)?;
        }
        // Implicit return 0 if no explicit return.
        let idx = func_compiler.chunk.add_constant(Value::Number(0.0));
        func_compiler.chunk.emit(Instruction::Constant(idx));
        func_compiler.chunk.emit(Instruction::Return);
        Ok(func_compiler.chunk)
    }
}

// ----------------------------------------------------------
// FunctionCompiler — compiles inside a function scope
// ----------------------------------------------------------
// This is a separate struct because function bodies need to
// resolve parameter names as local slots rather than globals.
struct FunctionCompiler {
    chunk: Chunk,
    /// Maps parameter names to stack slot indices.
    locals: Vec<String>,
    module_path: Option<String>,
}

impl FunctionCompiler {
    fn new(params: &[String], module_path: Option<String>) -> Self {
        Self {
            chunk: Chunk::new(),
            locals: params.to_vec(),
            module_path,
        }
    }

    /// Look up a local variable by name.
    fn resolve_local(&self, name: &str) -> Option<usize> {
        self.locals.iter().position(|n| n == name)
    }

    fn compile_stmt(&mut self, stmt: Stmt) -> Result<(), crate::error::LiabError> {
        match stmt {
            Stmt::Let { name, value } => {
                self.compile_expr(value)?;
                // In a function, `let` creates a new local.
                let slot = self.locals.len();
                self.locals.push(name);
                // The value is already on the stack at the right slot.
                // We just need to track it — no instruction needed
                // because the value sits at stack[base + slot].
                // However, to keep things simple and correct, we
                // store it similarly to a global but using SetLocal.
                // Actually, the value was just pushed to the stack —
                // it's already in the correct slot position. We
                // don't emit any additional instruction.
                let _ = slot; // slot is implicitly stack position
            }

            Stmt::Assignment { name, value } => {
                self.compile_expr(value)?;
                if let Some(slot) = self.resolve_local(&name) {
                    self.chunk.emit(Instruction::SetLocal(slot));
                } else {
                    let idx = self.chunk.add_constant(Value::String(name));
                    self.chunk.emit(Instruction::SetGlobal(idx));
                }
            }

            Stmt::Print(expr) => {
                self.compile_expr(expr)?;
                self.chunk.emit(Instruction::Print);
            }

            Stmt::Expression(expr) => {
                self.compile_expr(expr)?;
                self.chunk.emit(Instruction::Pop);
            }

            Stmt::Return(expr) => {
                self.compile_expr(expr)?;
                self.chunk.emit(Instruction::Return);
            }

            Stmt::FunctionDef { name, params, body } => {
                // Nested function: compile into its own chunk.
                let mut nested = FunctionCompiler::new(&params, self.module_path.clone());
                for s in body {
                    nested.compile_stmt(s)?;
                }
                let idx = nested.chunk.add_constant(Value::Number(0.0));
                nested.chunk.emit(Instruction::Constant(idx));
                nested.chunk.emit(Instruction::Return);

                let func_val = Value::CompiledFunction {
                    name: name.clone(),
                    arity: params.len(),
                    chunk: nested.chunk,
                    module_path: self.module_path.clone(),
                };
                let const_idx = self.chunk.add_constant(func_val);
                self.chunk.emit(Instruction::Constant(const_idx));

                // Store as local or global
                let slot = self.locals.len();
                self.locals.push(name);
                let _ = slot;
            }

            Stmt::If { condition, then_body, else_body } => {
                self.compile_expr(condition)?;
                let jump_to_else = self.chunk.emit_jump(
                    Instruction::JumpIfFalse(0xFFFF)
                );

                for s in then_body {
                    self.compile_stmt(s)?;
                }

                if let Some(else_stmts) = else_body {
                    let jump_over = self.chunk.emit_jump(
                        Instruction::Jump(0xFFFF)
                    );
                    self.chunk.patch_jump(jump_to_else);
                    for s in else_stmts {
                        self.compile_stmt(s)?;
                    }
                    self.chunk.patch_jump(jump_over);
                } else {
                    self.chunk.patch_jump(jump_to_else);
                }
            }

            Stmt::While { condition, body } => {
                let loop_start = self.chunk.current_offset();
                self.compile_expr(condition)?;
                let exit_jump = self.chunk.emit_jump(
                    Instruction::JumpIfFalse(0xFFFF)
                );
                for s in body {
                    self.compile_stmt(s)?;
                }
                self.chunk.emit(Instruction::Jump(loop_start));
                self.chunk.patch_jump(exit_jump);
            }

            Stmt::Block(stmts) => {
                for s in stmts {
                    self.compile_stmt(s)?;
                }
            }

            Stmt::Love(path) => {
                let idx = self.chunk.add_constant(Value::String(path));
                self.chunk.emit(Instruction::Love(idx));
            }

            Stmt::Export(name) => {
                let idx = self.chunk.add_constant(Value::String(name));
                self.chunk.emit(Instruction::Export(idx));
            }
        }
        Ok(())
    }

    fn compile_expr(&mut self, expr: Expr) -> Result<(), crate::error::LiabError> {
        match expr {
            Expr::Number(n) => {
                let idx = self.chunk.add_constant(Value::Number(n));
                self.chunk.emit(Instruction::Constant(idx));
            }
            Expr::Boolean(b) => {
                let idx = self.chunk.add_constant(Value::Boolean(b));
                self.chunk.emit(Instruction::Constant(idx));
            }
            Expr::String(s) => {
                let idx = self.chunk.add_constant(Value::String(s));
                self.chunk.emit(Instruction::Constant(idx));
            }
            Expr::Identifier(name) => {
                if let Some(slot) = self.resolve_local(&name) {
                    self.chunk.emit(Instruction::GetLocal(slot));
                } else {
                    let idx = self.chunk.add_constant(Value::String(name));
                    self.chunk.emit(Instruction::GetGlobal(idx));
                }
            }

            // ---- member access ----
            Expr::Member { object, property } => {
                self.compile_expr(*object)?;
                let idx = self.chunk.add_constant(Value::String(property));
                self.chunk.emit(Instruction::GetProperty(idx));
            }

            Expr::Index { object, index } => {
                self.compile_expr(*object)?;
                self.compile_expr(*index)?;
                self.chunk.emit(Instruction::GetIndex);
            }

            // ---- boolean/numeric ops ----
            Expr::BinaryOp { left, op, right } => {
                self.compile_expr(*left)?;
                self.compile_expr(*right)?;
                let instr = match op {
                    BinOp::Add => Instruction::Add,
                    BinOp::Sub => Instruction::Sub,
                    BinOp::Mul => Instruction::Mul,
                    BinOp::Div => Instruction::Div,
                    BinOp::Eq  => Instruction::Eq,
                    BinOp::Neq => Instruction::Neq,
                    BinOp::Gt  => Instruction::Gt,
                    BinOp::Lt  => Instruction::Lt,
                    BinOp::Gte => Instruction::Gte,
                    BinOp::Lte => Instruction::Lte,
                };
                self.chunk.emit(instr);
            }
            // ---- function call ----
            Expr::FunctionCall { callee, args } => {
                // Determine the function value
                self.compile_expr(*callee)?;

                // Compile each argument, pushing them onto the stack
                for arg in args.iter() {
                    self.compile_expr(arg.clone())?;
                }

                // Call instruction with arity
                self.chunk.emit(Instruction::Call(args.len()));
            }
        }
        Ok(())
    }
}

// ===========================================================
// Unit tests
// ===========================================================
#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::lex;
    use crate::parser::parse;

    /// Helper: compile a source string into a Chunk.
    fn compile_source(source: &str) -> Result<Chunk, crate::error::LiabError> {
        let tokens = lex(source)?;
        let stmts = parse(tokens)?;
        let compiler = Compiler::new();
        compiler.compile(stmts)
    }

    #[test]
    fn compile_constant() {
        let chunk = compile_source("42;").unwrap();
        // Should have: Constant(0), Pop, Constant(1), Return
        assert!(matches!(chunk.code[0], Instruction::Constant(0)));
        assert_eq!(chunk.constants[0], Value::Number(42.0));
        assert!(matches!(chunk.code[1], Instruction::Pop));
    }

    #[test]
    fn compile_arithmetic() {
        let chunk = compile_source("1 + 2;").unwrap();
        // Constant(0), Constant(1), Add, Pop, ...
        assert!(matches!(chunk.code[0], Instruction::Constant(0)));
        assert!(matches!(chunk.code[1], Instruction::Constant(1)));
        assert!(matches!(chunk.code[2], Instruction::Add));
        assert!(matches!(chunk.code[3], Instruction::Pop));
    }

    #[test]
    fn compile_let_and_print() {
        let chunk = compile_source("let x = 5; print x;").unwrap();
        // Constant(0), DefineGlobal("x"), GetGlobal("x"), Print, ...
        assert!(matches!(chunk.code[0], Instruction::Constant(0)));
        assert!(matches!(chunk.code[1], Instruction::DefineGlobal(_)));
        assert!(matches!(chunk.code[2], Instruction::GetGlobal(_)));
        assert!(matches!(chunk.code[3], Instruction::Print));
    }

    #[test]
    fn compile_if() {
        let chunk = compile_source("if true { print 1; }").unwrap();
        // Should contain a JumpIfFalse
        assert!(chunk.code.iter().any(|i| matches!(i, Instruction::JumpIfFalse(_))));
    }

    #[test]
    fn compile_while() {
        let chunk = compile_source("while false { print 1; }").unwrap();
        // Should contain both JumpIfFalse and Jump (loop back)
        assert!(chunk.code.iter().any(|i| matches!(i, Instruction::JumpIfFalse(_))));
        assert!(chunk.code.iter().any(|i| matches!(i, Instruction::Jump(_))));
    }

    #[test]
    fn compile_function() {
        let chunk = compile_source("fn add(a, b) { return a + b; }").unwrap();
        // Should have a CompiledFunction in the constant pool
        assert!(chunk.constants.iter().any(|v| matches!(v, Value::CompiledFunction { .. })));
    }
}
