// ============================================================
// LIAB Language — Virtual Machine (v0.3)
// ============================================================
//
// The VM executes compiled bytecode using a stack-based model.
//
// Key data structures:
//   • **Value stack** (`Vec<Value>`) — operands and results
//   • **Call stack** (`Vec<CallFrame>`) — one frame per
//     function invocation, tracking IP and stack base
//   • **Globals** (`HashMap<String, Value>`) — global variables
//
// Execution loop:
//   1. Fetch the instruction at the current frame's IP
//   2. Decode (pattern match) and execute
//   3. Advance IP (or jump)
//   4. Repeat until the top-level frame returns
// ============================================================

use std::collections::HashMap;

use crate::compiler::{Chunk, Instruction};
use crate::value::Value;

// ----------------------------------------------------------
// CallFrame
// ----------------------------------------------------------
/// A `CallFrame` represents a single active function call.
///
/// The VM maintains a stack of these.  The bottom-most frame
/// is the top-level "script" frame.
#[derive(Debug)]
struct CallFrame {
    /// The name of the function (or "<script>").
    name: String,
    /// The bytecode being executed.
    chunk: Chunk,
    /// Instruction pointer — index of the next instruction.
    ip: usize,
    /// The index into the value stack where this frame's
    /// locals begin.  For functions, slot 0 through arity-1
    /// hold the parameters.
    stack_base: usize,
    /// Originating module environment tag
    module_path: Option<String>,
}

#[derive(Debug, Clone)]
pub enum ModuleState {
    Loading,
    Loaded(Value),
}

// ----------------------------------------------------------
// VM
// ----------------------------------------------------------
#[derive(Debug)]
pub struct VM {
    /// The value stack — all operands and intermediate results.
    stack: Vec<Value>,
    /// The call stack — one frame per active function call.
    frames: Vec<CallFrame>,
    /// Global variable storage.
    pub globals: HashMap<String, Value>,
    /// Module private memory storage.
    pub environments: HashMap<String, HashMap<String, Value>>,
    /// Explicit exposed exports
    pub module_exports: HashMap<String, HashMap<String, Value>>,
    /// Module cache storage.
    pub modules: HashMap<String, ModuleState>,
    /// Tracing flag
    pub trace: bool,
}

impl VM {
    pub fn new() -> Self {
        Self {
            stack: Vec::new(),
            frames: Vec::new(),
            globals: HashMap::new(),
            environments: HashMap::new(),
            module_exports: HashMap::new(),
            modules: HashMap::new(),
            trace: false,
        }
    }

    /// Register a native Rust function into a specific namespace.
    /// Example: `vm.register_function("rust", "call", 2, my_rust_fn)`
    #[allow(dead_code)]
    pub fn register_function(
        &mut self,
        namespace: &str,
        name: &str,
        arity: usize,
        func: fn(&mut VM, &[Value]) -> Result<Value, crate::error::LiabError>
    ) {
        if !self.globals.contains_key(namespace) {
            self.globals.insert(namespace.to_string(), Value::Namespace(HashMap::new()));
        }
        if let Some(Value::Namespace(map)) = self.globals.get_mut(namespace) {
            map.insert(name.to_string(), Value::NativeFunction {
                name: format!("{}.{}", namespace, name),
                arity,
                func,
            });
        }
    }

    /// Execute a compiled chunk (the top-level script).
    pub fn run(&mut self, chunk: Chunk) -> Result<(), crate::error::LiabError> {
        // Push the script frame.
        self.frames.push(CallFrame {
            name: "<script>".into(),
            chunk,
            ip: 0,
            stack_base: 0,
            module_path: None,
        });

        self.execute()
    }

    // ----------------------------------------------------------
    // Main execution loop
    // ----------------------------------------------------------
    fn execute(&mut self) -> Result<(), crate::error::LiabError> {
        loop {
            // Fetch the current instruction.
            let frame = self.frames.last()
                .ok_or_else(|| self.runtime_error("No active call frame"))?;

            // If IP is past the end, we're done.
            if frame.ip >= frame.chunk.code.len() {
                return Ok(());
            }

            let instr = frame.chunk.code[frame.ip].clone();

            if self.trace {
                print!("          ");
                for slot in &self.stack {
                    print!("[ {} ]", slot);
                }
                println!();
                println!("{}", crate::disassembler::disassemble_instruction(&frame.chunk, frame.ip));
            }

            // Advance IP before executing (so jumps can override).
            self.frames.last_mut().unwrap().ip += 1;

            match instr {
                // ---- Constants ----
                Instruction::Constant(idx) => {
                    let frame = self.frames.last().unwrap();
                    let val = frame.chunk.constants[idx].clone();
                    self.stack.push(val);
                }

                // ---- Arithmetic ----
                Instruction::Add => self.binary_arith(|a, b| a + b)?,
                Instruction::Sub => self.binary_arith(|a, b| a - b)?,
                Instruction::Mul => self.binary_arith(|a, b| a * b)?,
                Instruction::Div => {
                    let b = self.pop_number()?;
                    let a = self.pop_number()?;
                    if b == 0.0 {
                        return Err(self.runtime_error("Runtime error: division by zero"));
                    }
                    self.stack.push(Value::Number(a / b));
                }

                // ---- Comparison ----
                Instruction::Eq  => self.binary_cmp(|a, b| a == b)?,
                Instruction::Neq => self.binary_cmp(|a, b| a != b)?,
                Instruction::Gt  => self.binary_cmp(|a, b| a > b)?,
                Instruction::Lt  => self.binary_cmp(|a, b| a < b)?,
                Instruction::Gte => self.binary_cmp(|a, b| a >= b)?,
                Instruction::Lte => self.binary_cmp(|a, b| a <= b)?,

                // ---- Stack manipulation ----
                Instruction::Pop => {
                    self.stack.pop()
                        .ok_or_else(|| self.runtime_error("Stack underflow on Pop"))?;
                }

                Instruction::Print => {
                    let val = self.stack.pop()
                        .ok_or_else(|| self.runtime_error("Stack underflow on Print"))?;
                    println!("{}", val);
                }

                // ---- Global variables ----
                Instruction::DefineGlobal(idx) => {
                    let name = match &self.frames.last().unwrap().chunk.constants[idx] {
                        Value::String(s) => s.clone(),
                        _ => unreachable!("DefineGlobal arg must be a string"),
                    };
                    let val = self.stack.pop()
                        .ok_or_else(|| self.runtime_error("Stack underflow on DefineGlobal"))?;
                    
                    if let Some(path) = self.frames.last().unwrap().module_path.clone() {
                        self.environments.get_mut(&path).unwrap().insert(name, val);
                    } else {
                        self.globals.insert(name, val);
                    }
                }

                Instruction::GetGlobal(idx) => {
                    let name = match &self.frames.last().unwrap().chunk.constants[idx] {
                        Value::String(s) => s.clone(),
                        _ => unreachable!("GetGlobal arg must be a string"),
                    };
                    
                    let val = if let Some(path) = self.frames.last().unwrap().module_path.clone() {
                        self.environments.get(&path).and_then(|e| e.get(&name)).cloned()
                    } else {
                        self.globals.get(&name).cloned()
                    };
                    
                    let val = val.ok_or_else(|| self.runtime_error(format!("Undefined variable: '{}'", name)))?;
                    self.stack.push(val);
                }

                Instruction::SetGlobal(idx) => {
                    let name = match &self.frames.last().unwrap().chunk.constants[idx] {
                        Value::String(s) => s.clone(),
                        _ => unreachable!("SetGlobal arg must be a string"),
                    };
                    let val = self.stack.pop()
                        .ok_or_else(|| self.runtime_error("Stack underflow on SetGlobal"))?;
                    
                    if let Some(path) = self.frames.last().unwrap().module_path.clone() {
                        let env = self.environments.get_mut(&path).unwrap();
                        if !env.contains_key(&name) {
                            return Err(self.runtime_error(format!("Undefined variable in module: '{}'", name)));
                        }
                        env.insert(name, val);
                    } else {
                        if !self.globals.contains_key(&name) {
                            return Err(self.runtime_error(format!("Undefined variable: '{}'", name)));
                        }
                        self.globals.insert(name, val);
                    }
                }
                // ---- Properties ----
                Instruction::GetProperty(idx) => {
                    let property = match &self.frames.last().unwrap().chunk.constants[idx] {
                        Value::String(s) => s.clone(),
                        _ => unreachable!("GetProperty arg must be a string"),
                    };

                    let object = self.stack.pop()
                        .ok_or_else(|| self.runtime_error("Stack underflow on GetProperty"))?;

                    match object {
                        Value::Namespace(exports) => {
                            let val = exports.get(&property)
                                .cloned()
                                .ok_or_else(|| self.runtime_error(format!("Property '{}' not found in namespace", property)))?;
                            self.stack.push(val);
                        }
                        other => {
                            return Err(self.runtime_error(format!("Cannot access property '{}' on {:?}", property, other)));
                        }
                    }
                }

                Instruction::GetIndex => {
                    let index = self.stack.pop()
                        .ok_or_else(|| self.runtime_error("Stack underflow on GetIndex"))?;
                    let object = self.stack.pop()
                        .ok_or_else(|| self.runtime_error("Stack underflow on GetIndex"))?;

                    match object {
                        Value::Namespace(exports) => {
                            if let Value::String(key) = index {
                                let val = exports.get(&key)
                                    .cloned()
                                    .ok_or_else(|| self.runtime_error(format!("Property '{}' not found in namespace", key)))?;
                                self.stack.push(val);
                            } else {
                                return Err(self.runtime_error("Namespace index must be a string"));
                            }
                        }
                        other => {
                            return Err(self.runtime_error(format!("Cannot index into '{}'", other)));
                        }
                    }
                }

                // ---- Operations ----
                // ---- Local variables ----
                Instruction::GetLocal(slot) => {
                    let base = self.frames.last().unwrap().stack_base;
                    let val = self.stack[base + slot].clone();
                    self.stack.push(val);
                }

                Instruction::SetLocal(slot) => {
                    let val = self.stack.pop()
                        .ok_or_else(|| self.runtime_error("Stack underflow on SetLocal"))?;
                    let base = self.frames.last().unwrap().stack_base;
                    self.stack[base + slot] = val;
                }

                // ---- Control flow ----
                Instruction::Jump(target) => {
                    self.frames.last_mut().unwrap().ip = target;
                }

                Instruction::JumpIfFalse(target) => {
                    let val = self.stack.pop()
                        .ok_or_else(|| self.runtime_error("Stack underflow on JumpIfFalse"))?;
                    if !Self::is_truthy(&val) {
                        self.frames.last_mut().unwrap().ip = target;
                    }
                }

                // ---- Function calls ----
                Instruction::Call(arg_count) => {
                    self.call_callable(arg_count)?;
                }

                Instruction::Return => {
                    // Pop the return value.
                    let return_val = self.stack.pop()
                        .ok_or_else(|| self.runtime_error("Stack underflow on Return"))?;

                    // Pop the call frame.
                    let frame = self.frames.pop()
                        .ok_or_else(|| self.runtime_error("No frame to return from"))?;

                    // If no frames left, we're done (top-level return).
                    if self.frames.is_empty() {
                        return Ok(());
                    }

                    // Discard any locals left on the stack by this
                    // frame (truncate back to the frame's base).
                    self.stack.truncate(frame.stack_base);
                    
                    if frame.name == "<script>" {
                        if let Some(path) = frame.module_path {
                            // Yield module evaluation
                            let exports = self.module_exports.remove(&path).unwrap_or_default();
                            let namespace = Value::Namespace(exports);
                            self.modules.insert(path.clone(), ModuleState::Loaded(namespace.clone()));
                            
                            let name = std::path::Path::new(&path).file_stem().unwrap().to_str().unwrap().to_string();
                            
                            if let Some(parent_path) = self.frames.last().unwrap().module_path.clone() {
                                self.environments.get_mut(&parent_path).unwrap().insert(name, namespace);
                            } else {
                                self.globals.insert(name, namespace);
                            }
                        }
                    }

                    // Push the return value for the caller.
                    self.stack.push(return_val);
                }
                // ---- Module System ----
                Instruction::Love(idx) => {
                    let path = match &self.frames.last().unwrap().chunk.constants[idx] {
                        Value::String(s) => s.clone(),
                        _ => unreachable!("Love arg must be a string"),
                    };

                    // Lazy load checking:
                    if let Some(state) = self.modules.get(&path) {
                        match state {
                            ModuleState::Loading => return Err(self.runtime_error(format!("Cyclic dependency detected: '{}'", path))),
                            ModuleState::Loaded(namespace) => {
                                let name = std::path::Path::new(&path).file_stem().unwrap().to_str().unwrap().to_string();
                                if let Some(parent_path) = self.frames.last().unwrap().module_path.clone() {
                                    self.environments.get_mut(&parent_path).unwrap().insert(name, namespace.clone());
                                } else {
                                    self.globals.insert(name, namespace.clone());
                                }
                                continue;
                            }
                        }
                    }

                    self.modules.insert(path.clone(), ModuleState::Loading);
                    self.environments.insert(path.clone(), HashMap::new());
                    self.module_exports.insert(path.clone(), HashMap::new());

                    // Otherwise, load from file system
                    let source = std::fs::read_to_string(format!("{}.liab", path))
                        .map_err(|e| self.runtime_error(format!("Failed to load module '{}': {}", path, e)))?;

                    // Compile natively passing unified tracing
                    let tokens = crate::lexer::lex(&source).map_err(|e| self.runtime_error(format!("Module lex error: {}", e)))?;
                    let stmts = crate::parser::parse(tokens).map_err(|e| self.runtime_error(format!("Module parse error: {}", e)))?;
                    let chunk = crate::compiler::Compiler::with_module(Some(path.clone())).compile(stmts).map_err(|e| self.runtime_error(format!("Module compile error: {}", e)))?;
                    
                    self.frames.push(CallFrame {
                        name: "<script>".into(),
                        chunk,
                        ip: 0,
                        stack_base: self.stack.len(),
                        module_path: Some(path),
                    });
                }

                Instruction::Export(idx) => {
                    let name = match &self.frames.last().unwrap().chunk.constants[idx] {
                        Value::String(s) => s.clone(),
                        _ => unreachable!("Export arg must be a string"),
                    };
                    
                    let frame = self.frames.last().unwrap();
                    let module_path = frame.module_path.clone()
                        .ok_or_else(|| self.runtime_error("Cannot export from top-level script, only modules"))?;
                        
                    let val = self.environments.get(&module_path).and_then(|e| e.get(&name)).cloned()
                        .ok_or_else(|| self.runtime_error(format!("Undefined variable to export: '{}'", name)))?;
                        
                    self.module_exports.get_mut(&module_path).unwrap().insert(name, val);
                }
            }
        }
    }

    // ----------------------------------------------------------
    // Helpers
    // ----------------------------------------------------------

    /// Dynamically dispatch a function call.
    fn call_callable(&mut self, arg_count: usize) -> Result<(), crate::error::LiabError> {
        let fn_pos = self.stack.len() - arg_count - 1;
        let fn_val = self.stack[fn_pos].clone();

        match fn_val {
            Value::CompiledFunction { name, arity, chunk, module_path } => {
                if arg_count != arity {
                    return Err(self.runtime_error(format!(
                        "Function '{}' expects {} arguments, got {}",
                        name, arity, arg_count
                    )));
                }
                let stack_base = fn_pos + 1;
                self.stack.remove(fn_pos);
                let stack_base = stack_base - 1;

                self.frames.push(CallFrame {
                    name, chunk, ip: 0, stack_base, module_path: module_path.clone()
                });
                Ok(())
            }
            Value::NativeFunction { name: fn_name, arity, func } => {
                if arg_count != arity {
                    return Err(self.runtime_error(format!(
                        "Native function '{}' expects {} arguments, got {}",
                        fn_name, arity, arg_count
                    )));
                }
                
                let stack_base = fn_pos + 1;
                let mut args = Vec::new();
                for i in stack_base..self.stack.len() {
                    args.push(self.stack[i].clone());
                }
                
                let result = func(self, &args)?;
                self.stack.truncate(fn_pos);
                self.stack.push(result);
                Ok(())
            }
            other => {
                Err(self.runtime_error(format!(
                    "Cannot call '{}' — it is not a function",
                    other
                )))
            }
        }
    }

    /// Generate a structured runtime error using current IP.
    fn runtime_error(&self, message: impl Into<String>) -> crate::error::LiabError {
        let (ip, frame_name) = if let Some(frame) = self.frames.last() {
            // we use ip - 1 because ip has already advanced
            let err_ip = if frame.ip > 0 { frame.ip - 1 } else { 0 };
            (err_ip, frame.name.clone())
        } else {
            (0, "<missing_frame>".into())
        };
        crate::error::LiabError::RuntimeError {
            message: message.into(),
            ip,
            frame_name,
        }
    }

    /// Pop a number from the stack, or error.
    fn pop_number(&mut self) -> Result<f64, crate::error::LiabError> {
        let val = self.stack.pop()
            .ok_or_else(|| self.runtime_error("Stack underflow"))?;
        match val {
            Value::Number(n) => Ok(n),
            other => Err(self.runtime_error(format!("Operand must be a number, got {:?}", other))),
        }
    }

    /// Binary arithmetic: pop two numbers, apply op, push result.
    fn binary_arith(&mut self, op: fn(f64, f64) -> f64) -> Result<(), crate::error::LiabError> {
        let b = self.pop_number()?;
        let a = self.pop_number()?;
        self.stack.push(Value::Number(op(a, b)));
        Ok(())
    }

    /// Binary comparison: pop two numbers, apply predicate, push bool.
    fn binary_cmp(&mut self, op: fn(f64, f64) -> bool) -> Result<(), crate::error::LiabError> {
        let b = self.pop_number()?;
        let a = self.pop_number()?;
        self.stack.push(Value::Boolean(op(a, b)));
        Ok(())
    }

    /// Determine truthiness of a value.
    fn is_truthy(val: &Value) -> bool {
        match val {
            Value::Boolean(b) => *b,
            Value::Number(n) => *n != 0.0,
            _ => true,
        }
    }
}

impl Default for VM {
    fn default() -> Self {
        Self::new()
    }
}

// ===========================================================
// Unit tests
// ===========================================================
#[cfg(test)]
mod tests {
    use super::*;
    use crate::compiler::Compiler;
    use crate::lexer::lex;
    use crate::parser::parse;

    /// Helper: compile and run a LIAB program, return the VM
    /// so we can inspect globals.
    fn run_program(source: &str) -> Result<VM, crate::error::LiabError> {
        let tokens = lex(source).unwrap();
        let stmts = parse(tokens).unwrap();
        let compiler = Compiler::new();
        let chunk = compiler.compile(stmts).unwrap();
        let mut vm = VM::new();
        vm.run(chunk)?;
        Ok(vm)
    }

    // ---- Arithmetic ----

    #[test]
    fn test_arithmetic() {
        let vm = run_program("let result = 2 + 3 * 4;").unwrap();
        assert_eq!(vm.globals.get("result"), Some(&Value::Number(14.0)));
    }

    #[test]
    fn test_subtraction() {
        let vm = run_program("let r = 10 - 3;").unwrap();
        assert_eq!(vm.globals.get("r"), Some(&Value::Number(7.0)));
    }

    #[test]
    fn test_division() {
        let vm = run_program("let r = 10 / 4;").unwrap();
        assert_eq!(vm.globals.get("r"), Some(&Value::Number(2.5)));
    }

    #[test]
    fn test_division_by_zero() {
        let result = run_program("let r = 10 / 0;");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("division by zero"));
    }

    // ---- Comparisons ----

    #[test]
    fn test_comparisons() {
        let vm = run_program(
            "let a = 5 > 3; let b = 5 < 3; let c = 5 == 5;\
             let d = 5 != 3; let e = 5 >= 5; let f = 5 <= 4;"
        ).unwrap();
        assert_eq!(vm.globals.get("a"), Some(&Value::Boolean(true)));
        assert_eq!(vm.globals.get("b"), Some(&Value::Boolean(false)));
        assert_eq!(vm.globals.get("c"), Some(&Value::Boolean(true)));
        assert_eq!(vm.globals.get("d"), Some(&Value::Boolean(true)));
        assert_eq!(vm.globals.get("e"), Some(&Value::Boolean(true)));
        assert_eq!(vm.globals.get("f"), Some(&Value::Boolean(false)));
    }

    // ---- Variables ----

    #[test]
    fn test_let_and_get() {
        let vm = run_program("let x = 42;").unwrap();
        assert_eq!(vm.globals.get("x"), Some(&Value::Number(42.0)));
    }

    #[test]
    fn test_assignment() {
        let vm = run_program("let x = 1; x = 42;").unwrap();
        assert_eq!(vm.globals.get("x"), Some(&Value::Number(42.0)));
    }

    #[test]
    fn test_undefined_variable() {
        let result = run_program("print y;");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Undefined variable"));
    }

    // ---- Booleans ----

    #[test]
    fn test_booleans() {
        let vm = run_program("let a = true; let b = false;").unwrap();
        assert_eq!(vm.globals.get("a"), Some(&Value::Boolean(true)));
        assert_eq!(vm.globals.get("b"), Some(&Value::Boolean(false)));
    }

    // ---- If/Else ----

    #[test]
    fn test_if_true() {
        let vm = run_program("let x = 0; if true { x = 42; }").unwrap();
        assert_eq!(vm.globals.get("x"), Some(&Value::Number(42.0)));
    }

    #[test]
    fn test_if_false_else() {
        let vm = run_program(
            "let x = 0; if false { x = 1; } else { x = 99; }"
        ).unwrap();
        assert_eq!(vm.globals.get("x"), Some(&Value::Number(99.0)));
    }

    // ---- While ----

    #[test]
    fn test_while_loop() {
        let vm = run_program(
            "let x = 0; while x < 5 { x = x + 1; }"
        ).unwrap();
        assert_eq!(vm.globals.get("x"), Some(&Value::Number(5.0)));
    }

    // ---- Functions ----

    #[test]
    fn test_function_call() {
        let vm = run_program(
            "fn add(a, b) { return a + b; } let r = add(2, 3);"
        ).unwrap();
        assert_eq!(vm.globals.get("r"), Some(&Value::Number(5.0)));
    }

    #[test]
    fn test_function_wrong_arity() {
        let result = run_program(
            "fn add(a, b) { return a + b; } add(1);"
        );
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("expects 2 arguments"));
    }

    #[test]
    fn test_recursive_function() {
        let vm = run_program(
            "fn factorial(n) {\n\
                if n <= 1 { return 1; }\n\
                return n * factorial(n - 1);\n\
            }\n\
            let r = factorial(5);"
        ).unwrap();
        assert_eq!(vm.globals.get("r"), Some(&Value::Number(120.0)));
    }

    // ---- Native Rust Functions ----

    #[test]
    fn test_native_function_rust_call() {
        let tokens = lex("let r = rust.call(\"process_data\", 10);").unwrap();
        let stmts = parse(tokens).unwrap();
        let chunk = Compiler::new().compile(stmts).unwrap();
        
        let mut vm = VM::new();
        // Register the native dispatcher "rust.call" into namespace "rust", arity 2
        vm.register_function("rust", "call", 2, |vm, args| {
            if let (Value::String(name), Value::Number(val)) = (&args[0], &args[1]) {
                if name == "process_data" {
                    // Double the number as a mock process
                    return Ok(Value::Number(val * 2.0));
                }
            }
            Err(vm.runtime_error("Invalid arguments or unknown function"))
        });

        vm.run(chunk).unwrap();
        assert_eq!(vm.globals.get("r"), Some(&Value::Number(20.0)));
    }

    #[test]
    fn test_native_invalid_arity() {
        let tokens = lex("let r = rust.call(10);").unwrap();
        let stmts = parse(tokens).unwrap();
        let chunk = Compiler::new().compile(stmts).unwrap();
        let mut vm = VM::new();
        vm.register_function("rust", "call", 2, |_, _| Ok(Value::Number(0.0)));
        let err = vm.run(chunk).unwrap_err();
        if let crate::error::LiabError::RuntimeError { message, .. } = err {
            assert!(message.contains("expects 2 arguments, got 1"));
        } else {
            panic!("Expected RuntimeError");
        }
    }

    #[test]
    fn test_chained_namespace_access() {
        let tokens = lex("let r = math.geo.area();").unwrap();
        let stmts = parse(tokens).unwrap();
        let chunk = Compiler::new().compile(stmts).unwrap();
        let mut vm = VM::new();
        
        let mut geo_ns = std::collections::HashMap::new();
        geo_ns.insert("area".to_string(), Value::NativeFunction {
            name: "math.geo.area".to_string(),
            arity: 0,
            func: |_, _| Ok(Value::Number(99.9))
        });
        
        let mut math_ns = std::collections::HashMap::new();
        math_ns.insert("geo".to_string(), Value::Namespace(geo_ns));
        
        vm.globals.insert("math".to_string(), Value::Namespace(math_ns));
        
        vm.run(chunk).unwrap();
        assert_eq!(vm.globals.get("r"), Some(&Value::Number(99.9)));
    }

    #[test]
    fn test_call_non_function() {
        let tokens = lex("let a = 10; a();").unwrap();
        let stmts = parse(tokens).unwrap();
        let chunk = Compiler::new().compile(stmts).unwrap();
        let mut vm = VM::new();
        let err = vm.run(chunk).unwrap_err();
        if let crate::error::LiabError::RuntimeError { message, .. } = err {
            assert!(message.contains("Cannot call"), "Unexpected error: {}", message);
        } else {
            panic!("Expected RuntimeError");
        }
    }

    #[test]
    fn test_lazy_module_loading() {
        // Assume 'math.liab' is already written to the local disk test suite path.
        let tokens = lex("love \"math\"; let result = math.add(10, 5); let dynamic_pi = math[\"pi\"];").unwrap();
        let stmts = parse(tokens).unwrap();
        let chunk = Compiler::new().compile(stmts).unwrap();
        let mut vm = VM::new();
        vm.run(chunk).unwrap();

        // 10 + 5 from the math `add` module
        assert_eq!(vm.globals.get("result"), Some(&Value::Number(15.0)));
        assert_eq!(vm.globals.get("dynamic_pi"), Some(&Value::Number(3.14)));
        
        // Assert the math namespace caches 
        if let Some(Value::Namespace(exports)) = vm.globals.get("math") {
            assert!(exports.contains_key("pi"));
            assert!(exports.contains_key("add"));
        } else {
            panic!("Expected math module to parse locally inside namespace");
        }
    }
}
