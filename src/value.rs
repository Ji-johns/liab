// ============================================================
// LIAB Language — Runtime Value
// ============================================================
//
// A `Value` represents the result of evaluating an expression
// at runtime.  We support numbers, booleans, and functions.
//
// Two function representations exist:
//   • `Function`         — stores AST (used by tree-walk interpreter)
//   • `CompiledFunction` — stores bytecode (used by the VM)
//
// Keeping values in their own module makes it easy to extend
// later without touching the interpreter logic.
// ============================================================

use std::collections::HashMap;
use std::fmt;
use crate::ast::Stmt;

/// A runtime value produced by the interpreter.
#[derive(Debug, Clone, PartialEq)]
#[allow(unpredictable_function_pointer_comparisons)]
pub enum Value {
    /// A 64-bit floating-point number.
    Number(f64),

    /// A boolean value: `true` or `false`. (v0.2)
    Boolean(bool),

    /// A string value. (v0.4)
    String(String),

    /// A native Rust function capable of operating on VM values. (v0.5)
    #[allow(dead_code)]
    NativeFunction {
        name: String,
        arity: usize,
        func: fn(&mut crate::vm::VM, &[Value]) -> Result<Value, crate::error::LiabError>,
    },

    /// A function value, created by `fn` definitions. (v0.2)
    ///
    /// Functions are first-class values — they can be stored
    /// in variables and passed around.
    Function {
        name: String,
        params: Vec<String>,
        body: Vec<Stmt>,
    },

    /// A compiled function (bytecode). (v0.3)
    CompiledFunction {
        name: String,
        arity: usize,
        chunk: crate::compiler::Chunk,
        module_path: Option<String>,
    },

    /// A namespace containing multiple named values. (v0.5)
    Namespace(HashMap<String, Value>),
}

// ----------------------------------------------------------
// Display — controls what `print` outputs
// ----------------------------------------------------------
impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            // If the number has no fractional part, display it
            // as an integer (e.g. `42` instead of `42.0`).
            Value::Number(n) => {
                if n.fract() == 0.0 {
                    write!(f, "{}", *n as i64)
                } else {
                    write!(f, "{}", n)
                }
            }
            Value::Boolean(b) => write!(f, "{}", b),
            Value::String(s) => write!(f, "\"{}\"", s),
            Value::NativeFunction { name, .. } => write!(f, "<native fn {}>", name),
            Value::Function { name, .. } => write!(f, "<fn {}>", name),
            Value::CompiledFunction { name, .. } => write!(f, "<fn {}>", name),
            Value::Namespace(_) => write!(f, "<namespace>"),
        }
    }
}
