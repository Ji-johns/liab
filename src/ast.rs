// ============================================================
// LIAB Language — AST
// ============================================================
//
// The Abstract Syntax Tree (AST) is the *structured*
// representation of your program.  While the lexer gives us a
// flat list of tokens, the AST captures the **hierarchical**
// relationships — for example, that `1 + 2 * 3` means
// "add 1 to (2 times 3)", not "(1 plus 2) times 3".
//
// We define two main types:
//   • `Expr`  — an expression that produces a value
//   • `Stmt`  — a statement that performs an action
//
// Each is a Rust enum, which means the compiler guarantees
// that every match on an AST node handles all cases.
// ============================================================

// ----------------------------------------------------------
// Binary operators
// ----------------------------------------------------------
/// Arithmetic and comparison operators.
#[derive(Debug, Clone, PartialEq)]
pub enum BinOp {
    // Arithmetic
    Add, // +
    Sub, // -
    Mul, // *
    Div, // /

    // Comparison (v0.2)
    Eq,  // ==
    Neq, // !=
    Gt,  // >
    Lt,  // <
    Gte, // >=
    Lte, // <=
}

// ----------------------------------------------------------
// Expressions
// ----------------------------------------------------------
/// An expression is anything that evaluates to a value.
///
/// `Box<Expr>` is used for recursive variants so the enum
/// has a known size at compile time (Rust requires this).
#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    /// A numeric literal, e.g. `42` or `3.14`.
    Number(f64),

    /// A boolean literal: `true` or `false`. (v0.2)
    Boolean(bool),

    /// A string literal. (v0.4)
    String(String),

    /// A variable reference, e.g. `x`.
    Identifier(String),

    /// A property access, e.g. `rust.call`. (v0.5)
    Member {
        object: Box<Expr>,
        property: String,
    },

    /// An indexed access, e.g. `arr[0]` or `map["key"]`. (v0.6)
    Index {
        object: Box<Expr>,
        index: Box<Expr>,
    },

    /// A binary operation, e.g. `a + b` or `x > 10`.
    ///
    /// `left` and `right` are boxed because an expression can
    /// contain other expressions (it's recursive).
    BinaryOp {
        left: Box<Expr>,
        op: BinOp,
        right: Box<Expr>,
    },

    /// A function call, e.g. `add(2, 3)` or `rust.call()`. (v0.5)
    FunctionCall {
        callee: Box<Expr>,
        args: Vec<Expr>,
    },
}

// ----------------------------------------------------------
// Statements
// ----------------------------------------------------------
/// A statement performs an action but does not itself
/// produce a value that can be used in further expressions.
#[derive(Debug, Clone, PartialEq)]
pub enum Stmt {
    /// Variable declaration: `let x = <expr>;`
    Let {
        name: String,
        value: Expr,
    },

    /// Variable reassignment: `x = <expr>;` (v0.2)
    Assignment {
        name: String,
        value: Expr,
    },

    /// Print statement: `print <expr>;`
    Print(Expr),

    /// A bare expression used as a statement: `<expr>;`
    Expression(Expr),

    /// Function definition: `fn name(params) { body }` (v0.2)
    FunctionDef {
        name: String,
        params: Vec<String>,
        body: Vec<Stmt>,
    },
    /// load module: `love "math";`
    Love(String),

    /// explicitly export a global variable: `export foo;`
    Export(String),

    /// Return statement: `return <expr>;` (v0.2)
    Return(Expr),

    /// Conditional: `if <expr> { ... } else { ... }` (v0.2)
    If {
        condition: Expr,
        then_body: Vec<Stmt>,
        else_body: Option<Vec<Stmt>>,
    },

    /// Loop: `while <expr> { ... }` (v0.2)
    While {
        condition: Expr,
        body: Vec<Stmt>,
    },

    /// A scoped block: `{ ... }` (v0.2)
    Block(Vec<Stmt>),
}
