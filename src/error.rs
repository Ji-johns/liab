// ============================================================
// LIAB Language — Error Types (v0.4)
// ============================================================
//
// Structured error types that carry context about *where*
// and *why* something failed.  This replaces the raw
// `String` errors used in v0.3.
//
//   • `LiabError::CompileError`  — problem during compilation
//   • `LiabError::RuntimeError`  — problem during VM execution
//
// Both implement `Display` for human-readable messages and
// `Error` for idiomatic Rust error handling.
// ============================================================

use std::fmt;

/// A structured error produced by the LIAB compiler or VM.
#[derive(Debug, Clone)]
#[allow(dead_code)]
#[allow(clippy::enum_variant_names)]
pub enum LiabError {
    /// A syntax error produced by the lexer or parser.
    SyntaxError(String),

    /// An error produced during bytecode compilation.
    CompileError {
        message: String,
        line: usize,
        column: usize,
    },

    /// An error encountered during VM execution.
    ///
    /// Carries the instruction pointer and function name so
    /// the user can trace exactly where the failure occurred.
    RuntimeError {
        message: String,
        /// Instruction pointer at the time of the error.
        ip: usize,
        /// Name of the function being executed (or "<script>").
        frame_name: String,
    },
}

impl fmt::Display for LiabError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LiabError::SyntaxError(msg) => {
                write!(f, "Syntax Error: {}", msg)
            }
            LiabError::CompileError { message, line, column } => {
                write!(f, "Compile Error at [{}:{}]: {}", line, column, message)
            }
            LiabError::RuntimeError { message, ip, frame_name } => {
                write!(f, "Runtime Error in '{}' at IP {:04}: {}", frame_name, ip, message)
            }
        }
    }
}

impl std::error::Error for LiabError {}

// ===========================================================
// Unit tests
// ===========================================================
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compile_error_display() {
        let err = LiabError::CompileError {
            message: "Variable not found".to_string(),
            line: 1,
            column: 5,
        };
        assert_eq!(
            err.to_string(),
            "Compile Error at [1:5]: Variable not found"
        );
    }

    #[test]
    fn test_runtime_error_display() {
        let err = LiabError::RuntimeError {
            message: "division by zero".into(),
            ip: 5,
            frame_name: "<script>".into(),
        };
        assert_eq!(
            err.to_string(),
            "Runtime Error in '<script>' at IP 0005: division by zero"
        );
    }

    #[test]
    fn test_runtime_error_in_function() {
        let err = LiabError::RuntimeError {
            message: "stack underflow".into(),
            ip: 2,
            frame_name: "factorial".into(),
        };
        assert!(err.to_string().contains("factorial"));
        assert!(err.to_string().contains("IP 0002"));
    }
}
