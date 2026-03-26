// ============================================================
// LIAB Language — Interpreter
// ============================================================
//
// The interpreter is a **tree-walking evaluator**.  It
// traverses the AST and directly executes each node.
//
// In v0.2 we support:
//   • Variable scoping via Environment (nested scopes)
//   • Function definitions and calls
//   • Return statements (using a signal mechanism)
//   • If/else conditionals
//   • While loops
//   • Comparison operators
//   • Boolean values
//
// There is no garbage collector — Rust's ownership model
// handles memory automatically.
// ============================================================

use crate::ast::{BinOp, Expr, Stmt};
use crate::environment::Environment;
use crate::value::Value;

// ----------------------------------------------------------
// Signal — control-flow mechanism for `return`
// ----------------------------------------------------------
// When a `return` statement is executed inside a function,
// we need to unwind all the way back to the call site.
// Rather than using panics or exceptions, we use this
// enum to signal the interpreter.

/// The result of executing a statement.
enum Signal {
    /// Normal execution — continue to the next statement.
    None,
    /// A `return <value>` was executed — propagate up.
    Return(Value),
}

/// The interpreter state.
#[derive(Debug)]
pub struct Interpreter {
    /// The current variable environment (scope chain).
    env: Environment,
}

impl Interpreter {
    // ----------------------------------------------------------
    // Construction
    // ----------------------------------------------------------
    pub fn new() -> Self {
        Self {
            env: Environment::new(),
        }
    }

    // ----------------------------------------------------------
    // Public entry point
    // ----------------------------------------------------------

    /// Execute a list of statements (a complete program).
    pub fn run(&mut self, stmts: Vec<Stmt>) -> Result<(), String> {
        for stmt in stmts {
            // At top-level, return signals are errors (you can't
            // return outside of a function).
            if let Signal::Return(_) = self.execute_stmt(stmt)? {
                return Err("Cannot use 'return' outside of a function".into());
            }
        }
        Ok(())
    }

    // ----------------------------------------------------------
    // Statement execution
    // ----------------------------------------------------------

    fn execute_stmt(&mut self, stmt: Stmt) -> Result<Signal, String> {
        match stmt {
            // ---- let <name> = <expr>; ----
            Stmt::Let { name, value } => {
                let val = self.eval_expr(value)?;
                self.env.define(name, val);
                Ok(Signal::None)
            }

            // ---- <name> = <expr>; ----
            Stmt::Assignment { name, value } => {
                let val = self.eval_expr(value)?;
                self.env.set(&name, val)?;
                Ok(Signal::None)
            }

            // ---- print <expr>; ----
            Stmt::Print(expr) => {
                let val = self.eval_expr(expr)?;
                println!("{}", val);
                Ok(Signal::None)
            }

            // ---- <expr>; ----
            Stmt::Expression(expr) => {
                // Evaluate for side-effects; discard the result.
                self.eval_expr(expr)?;
                Ok(Signal::None)
            }

            // ---- fn <name>(<params>) { <body> } ----
            Stmt::FunctionDef { name, params, body } => {
                let func = Value::Function {
                    name: name.clone(),
                    params,
                    body,
                };
                self.env.define(name, func);
                Ok(Signal::None)
            }

            // ---- return <expr>; ----
            Stmt::Return(expr) => {
                let val = self.eval_expr(expr)?;
                Ok(Signal::Return(val))
            }

            // ---- if <cond> { ... } [else { ... }] ----
            Stmt::If { condition, then_body, else_body } => {
                let cond_val = self.eval_expr(condition)?;
                if Self::is_truthy(&cond_val) {
                    let signal = self.execute_block(then_body)?;
                    Ok(signal)
                } else if let Some(else_stmts) = else_body {
                    let signal = self.execute_block(else_stmts)?;
                    Ok(signal)
                } else {
                    Ok(Signal::None)
                }
            }

            // ---- while <cond> { ... } ----
            Stmt::While { condition, body } => {
                loop {
                    let cond_val = self.eval_expr(condition.clone())?;
                    if !Self::is_truthy(&cond_val) {
                        break;
                    }
                    let signal = self.execute_block(body.clone())?;
                    if let Signal::Return(_) = signal {
                        return Ok(signal);
                    }
                }
                Ok(Signal::None)
            }

            // ---- love <path>; ----
            Stmt::Love(_) => {
                Err("Interpreter does not support loading modules natively".into())
            }

            // ---- export <name>; ----
            Stmt::Export(_) => {
                Err("Interpreter does not support exporting variables".into())
            }

            // ---- { ... } ----
            Stmt::Block(stmts) => {
                self.execute_block(stmts)
            }
        }
    }

    /// Execute a list of statements in a new child scope.
    fn execute_block(&mut self, stmts: Vec<Stmt>) -> Result<Signal, String> {
        // Enter a new scope by replacing env with a child.
        let parent = std::mem::replace(&mut self.env, Environment::new());
        self.env = Environment::with_parent(parent);

        let mut result = Signal::None;
        for stmt in stmts {
            result = self.execute_stmt(stmt)?;
            if let Signal::Return(_) = result {
                break;
            }
        }

        // Exit the scope — restore the parent.
        let child = std::mem::replace(&mut self.env, Environment::new());
        self.env = child.into_parent()
            .expect("Block scope should have a parent");

        Ok(result)
    }

    // ----------------------------------------------------------
    // Expression evaluation
    // ----------------------------------------------------------

    fn eval_expr(&mut self, expr: Expr) -> Result<Value, String> {
        match expr {
            // ---- literal number ----
            Expr::Number(n) => Ok(Value::Number(n)),

            // ---- literal boolean ----
            Expr::Boolean(b) => Ok(Value::Boolean(b)),
            Expr::String(s) => Ok(Value::String(s.clone())),

            // ---- variable lookup ----
            Expr::Identifier(name) => {
                self.env
                    .get(&name)
                    .cloned()
                    .ok_or_else(|| format!("Undefined variable: '{}'", name))
            }

            // ---- member access ----
            Expr::Member { .. } => {
                Err("Interpreter does not support member access".into())
            }

            Expr::Index { .. } => {
                Err("Interpreter does not support index access".into())
            }

            // ---- binary operation ----
            Expr::BinaryOp { left, op, right } => {
                let left_val = self.eval_expr(*left)?;
                let right_val = self.eval_expr(*right)?;

                match op {
                    // Arithmetic — both sides must be numbers
                    BinOp::Add | BinOp::Sub | BinOp::Mul | BinOp::Div => {
                        let l = Self::extract_number(&left_val)?;
                        let r = Self::extract_number(&right_val)?;
                        let result = match op {
                            BinOp::Add => l + r,
                            BinOp::Sub => l - r,
                            BinOp::Mul => l * r,
                            BinOp::Div => {
                                if r == 0.0 {
                                    return Err("Runtime error: division by zero".into());
                                }
                                l / r
                            }
                            _ => unreachable!(),
                        };
                        Ok(Value::Number(result))
                    }

                    // Comparison — both sides must be numbers,
                    // result is a boolean.
                    BinOp::Eq | BinOp::Neq | BinOp::Gt | BinOp::Lt
                    | BinOp::Gte | BinOp::Lte => {
                        let l = Self::extract_number(&left_val)?;
                        let r = Self::extract_number(&right_val)?;
                        let result = match op {
                            BinOp::Eq  => l == r,
                            BinOp::Neq => l != r,
                            BinOp::Gt  => l > r,
                            BinOp::Lt  => l < r,
                            BinOp::Gte => l >= r,
                            BinOp::Lte => l <= r,
                            _ => unreachable!(),
                        };
                        Ok(Value::Boolean(result))
                    }
                }
            }

            // ---- function calls ----
            Expr::FunctionCall { callee, args } => {
                let callee_val = self.eval_expr(*callee)?;

                // Evaluate arguments
                let mut eval_args = Vec::new();
                for arg in args {
                    eval_args.push(self.eval_expr(arg)?);
                }

                match callee_val {
                    Value::Function { name, params, body } => {
                        if eval_args.len() != params.len() {
                            return Err(format!(
                                "Function '{}' expects {} arguments, got {}",
                                name, params.len(), eval_args.len()
                            ));
                        }

                        // Create a new child scope for the function execution
                        let parent = std::mem::replace(&mut self.env, crate::environment::Environment::new());
                        self.env = crate::environment::Environment::with_parent(parent);

                        for (param_name, arg_val) in params.iter().zip(eval_args) {
                            self.env.define(param_name.clone(), arg_val);
                        }

                        // Execute body
                        let mut returned_val = None;
                        for stmt in &body {
                            match self.execute_stmt(stmt.clone()) {
                                Ok(crate::interpreter::Signal::None) => continue,
                                Ok(crate::interpreter::Signal::Return(val)) => {
                                    returned_val = Some(val);
                                    break;
                                }
                                Err(e) => {
                                    let child = std::mem::replace(&mut self.env, crate::environment::Environment::new());
                                    if let Some(p) = child.into_parent() { self.env = p; }
                                    return Err(e);
                                }
                            }
                        }

                        let child = std::mem::replace(&mut self.env, crate::environment::Environment::new());
                        if let Some(p) = child.into_parent() { self.env = p; }

                        Ok(returned_val.unwrap_or(Value::Number(0.0)))
                    }
                    Value::NativeFunction { .. } => {
                        Err("Interpreter does not support calling VM NativeFunctions".into())
                    }
                    other => Err(format!("Cannot call '{:?}' — it is not a function", other)),
                }
            }
        }
    }

    // ----------------------------------------------------------
    // Helpers
    // ----------------------------------------------------------

    /// Extract an `f64` from a Value, or return an error.
    fn extract_number(val: &Value) -> Result<f64, String> {
        match val {
            Value::Number(n) => Ok(*n),
            other => Err(format!("Expected a number, got {}", other)),
        }
    }

    /// Determine the truthiness of a value.
    ///
    /// - `Boolean(false)` is falsy
    /// - `Number(0.0)` is falsy
    /// - Everything else is truthy
    fn is_truthy(val: &Value) -> bool {
        match val {
            Value::Boolean(b) => *b,
            Value::Number(n) => *n != 0.0,
            Value::String(s) => !s.is_empty(),
            Value::Function { .. } | Value::CompiledFunction { .. } | Value::NativeFunction { .. } | Value::Namespace(_) => true,
        }
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

    /// Helper: run a LIAB program string through the full pipeline
    /// and return the interpreter (so we can inspect variable state).
    fn run_program(source: &str) -> Result<Interpreter, String> {
        let tokens = lex(source).map_err(|e| e.to_string())?;
        let stmts = parse(tokens).map_err(|e| e.to_string())?;
        let mut interp = Interpreter::new();
        interp.run(stmts)?;
        Ok(interp)
    }

    // ---- v0.1 tests (preserved) ----

    #[test]
    fn variable_declaration_and_lookup() {
        let interp = run_program("let x = 42;").unwrap();
        assert_eq!(interp.env.get("x"), Some(&Value::Number(42.0)));
    }

    #[test]
    fn arithmetic_evaluation() {
        let interp = run_program("let result = 2 + 3 * 4;").unwrap();
        // 2 + (3 * 4) = 14
        assert_eq!(interp.env.get("result"), Some(&Value::Number(14.0)));
    }

    #[test]
    fn division_by_zero() {
        let result = run_program("let x = 1 / 0;");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("division by zero"));
    }

    #[test]
    fn undefined_variable() {
        let result = run_program("print y;");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Undefined variable"));
    }

    #[test]
    fn variable_reuse() {
        let interp = run_program(
            "let a = 10;\nlet b = 20;\nlet c = a + b;"
        ).unwrap();
        assert_eq!(interp.env.get("c"), Some(&Value::Number(30.0)));
    }

    // ---- v0.2 tests ----

    #[test]
    fn test_boolean_values() {
        let interp = run_program("let a = true; let b = false;").unwrap();
        assert_eq!(interp.env.get("a"), Some(&Value::Boolean(true)));
        assert_eq!(interp.env.get("b"), Some(&Value::Boolean(false)));
    }

    #[test]
    fn test_comparison_operators() {
        let interp = run_program(
            "let a = 5 > 3; let b = 5 < 3; let c = 5 == 5; let d = 5 != 3; let e = 5 >= 5; let f = 5 <= 4;"
        ).unwrap();
        assert_eq!(interp.env.get("a"), Some(&Value::Boolean(true)));
        assert_eq!(interp.env.get("b"), Some(&Value::Boolean(false)));
        assert_eq!(interp.env.get("c"), Some(&Value::Boolean(true)));
        assert_eq!(interp.env.get("d"), Some(&Value::Boolean(true)));
        assert_eq!(interp.env.get("e"), Some(&Value::Boolean(true)));
        assert_eq!(interp.env.get("f"), Some(&Value::Boolean(false)));
    }

    #[test]
    fn test_function_call() {
        let interp = run_program(
            "fn add(a, b) { return a + b; }\nlet result = add(2, 3);"
        ).unwrap();
        assert_eq!(interp.env.get("result"), Some(&Value::Number(5.0)));
    }

    #[test]
    fn test_function_no_return() {
        // A function without an explicit return should return 0
        let interp = run_program(
            "fn noop() { let x = 1; }\nlet r = noop();"
        ).unwrap();
        assert_eq!(interp.env.get("r"), Some(&Value::Number(0.0)));
    }

    #[test]
    fn test_function_wrong_arity() {
        let result = run_program(
            "fn add(a, b) { return a + b; }\nadd(1);"
        );
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("expects 2 arguments"));
    }

    #[test]
    fn test_if_true_branch() {
        let interp = run_program(
            "let x = 0;\nif true { x = 42; }"
        ).unwrap();
        assert_eq!(interp.env.get("x"), Some(&Value::Number(42.0)));
    }

    #[test]
    fn test_if_false_branch() {
        let interp = run_program(
            "let x = 0;\nif false { x = 1; } else { x = 99; }"
        ).unwrap();
        assert_eq!(interp.env.get("x"), Some(&Value::Number(99.0)));
    }

    #[test]
    fn test_while_loop() {
        let interp = run_program(
            "let x = 0;\nwhile x < 5 { x = x + 1; }"
        ).unwrap();
        assert_eq!(interp.env.get("x"), Some(&Value::Number(5.0)));
    }

    #[test]
    fn test_nested_scopes() {
        // Inner block should not leak its variable
        let interp = run_program(
            "let x = 10;\n{ let y = 20; }"
        ).unwrap();
        assert_eq!(interp.env.get("x"), Some(&Value::Number(10.0)));
        assert_eq!(interp.env.get("y"), None);
    }

    #[test]
    fn test_assignment() {
        let interp = run_program(
            "let x = 1;\nx = 42;"
        ).unwrap();
        assert_eq!(interp.env.get("x"), Some(&Value::Number(42.0)));
    }

    #[test]
    fn test_return_outside_function() {
        let result = run_program("return 42;");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("return"));
    }

    #[test]
    fn test_recursive_function() {
        let interp = run_program(
            "fn factorial(n) {\n\
                if n <= 1 { return 1; }\n\
                return n * factorial(n - 1);\n\
            }\n\
            let r = factorial(5);"
        ).unwrap();
        assert_eq!(interp.env.get("r"), Some(&Value::Number(120.0)));
    }
}
