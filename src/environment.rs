// ============================================================
// LIAB Language — Environment (v0.2)
// ============================================================
//
// The Environment implements **lexical scoping** — each scope
// has its own set of variable bindings plus an optional link
// to a parent (enclosing) scope.
//
// When a variable is looked up, we search the current scope
// first, then walk up the parent chain.  This gives us:
//
//   • Local variables in function bodies / blocks
//   • Access to outer (enclosing) variables
//   • Variable shadowing (inner scope can redefine a name)
//
// We use `Box<Environment>` for the parent link to avoid
// infinite-size types.  Rust's ownership model ensures memory
// is freed when a scope is dropped — no garbage collector.
// ============================================================

use std::collections::HashMap;
use crate::value::Value;

/// A single scope in the environment chain.
#[derive(Debug, Clone)]
pub struct Environment {
    /// Variables defined in this scope.
    values: HashMap<String, Value>,
    /// The enclosing (parent) scope, if any.
    parent: Option<Box<Environment>>,
}

impl Environment {
    // ----------------------------------------------------------
    // Construction
    // ----------------------------------------------------------

    /// Create a new top-level (global) environment.
    pub fn new() -> Self {
        Self {
            values: HashMap::new(),
            parent: None,
        }
    }

    /// Create a child environment that encloses `parent`.
    pub fn with_parent(parent: Environment) -> Self {
        Self {
            values: HashMap::new(),
            parent: Some(Box::new(parent)),
        }
    }

    // ----------------------------------------------------------
    // Variable operations
    // ----------------------------------------------------------

    /// Define a new variable in the **current** scope.
    /// If it already exists in this scope, it is overwritten.
    pub fn define(&mut self, name: String, value: Value) {
        self.values.insert(name, value);
    }

    /// Look up a variable by walking up the scope chain.
    ///
    /// Returns `None` if the variable is not defined in any
    /// enclosing scope.
    pub fn get(&self, name: &str) -> Option<&Value> {
        if let Some(val) = self.values.get(name) {
            Some(val)
        } else if let Some(ref parent) = self.parent {
            parent.get(name)
        } else {
            None
        }
    }

    /// Update an existing variable in the scope where it was
    /// originally defined.
    ///
    /// Returns an error if the variable does not exist.
    pub fn set(&mut self, name: &str, value: Value) -> Result<(), String> {
        if self.values.contains_key(name) {
            self.values.insert(name.to_string(), value);
            Ok(())
        } else if let Some(ref mut parent) = self.parent {
            parent.set(name, value)
        } else {
            Err(format!("Undefined variable: '{}'", name))
        }
    }

    /// Take the parent environment, consuming the current one.
    /// Used when exiting a scope to restore the enclosing state.
    pub fn into_parent(self) -> Option<Environment> {
        self.parent.map(|p| *p)
    }
}

// ===========================================================
// Unit tests
// ===========================================================
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn define_and_get() {
        let mut env = Environment::new();
        env.define("x".into(), Value::Number(42.0));
        assert_eq!(env.get("x"), Some(&Value::Number(42.0)));
    }

    #[test]
    fn undefined_variable() {
        let env = Environment::new();
        assert_eq!(env.get("y"), None);
    }

    #[test]
    fn child_scope_accesses_parent() {
        let mut global = Environment::new();
        global.define("x".into(), Value::Number(10.0));

        let child = Environment::with_parent(global);
        assert_eq!(child.get("x"), Some(&Value::Number(10.0)));
    }

    #[test]
    fn child_scope_shadows_parent() {
        let mut global = Environment::new();
        global.define("x".into(), Value::Number(10.0));

        let mut child = Environment::with_parent(global);
        child.define("x".into(), Value::Number(99.0));
        assert_eq!(child.get("x"), Some(&Value::Number(99.0)));
    }

    #[test]
    fn set_updates_correct_scope() {
        let mut global = Environment::new();
        global.define("x".into(), Value::Number(10.0));

        let mut child = Environment::with_parent(global);
        child.set("x", Value::Number(20.0)).unwrap();
        assert_eq!(child.get("x"), Some(&Value::Number(20.0)));
    }

    #[test]
    fn set_undefined_returns_error() {
        let mut env = Environment::new();
        let result = env.set("z", Value::Number(1.0));
        assert!(result.is_err());
    }

    #[test]
    fn into_parent_restores_scope() {
        let mut global = Environment::new();
        global.define("x".into(), Value::Number(10.0));

        let child = Environment::with_parent(global);
        let restored = child.into_parent().unwrap();
        assert_eq!(restored.get("x"), Some(&Value::Number(10.0)));
    }
}
