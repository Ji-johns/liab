// ============================================================
// LIAB Language — Lexer
// ============================================================
//
// The lexer (also called a "tokeniser" or "scanner") is the very
// first stage of the language pipeline.  It takes raw source text
// and converts it into a flat list of **tokens** — the smallest
// meaningful units of the language.
//
// For example, the source line:
//
//     let x = 10 + 3.14;
//
// becomes the token stream:
//
//     [Let, Identifier("x"), Equals, Number(10.0), Plus, Number(3.14), Semicolon, Eof]
//
// ============================================================

// ----------------------------------------------------------
// Token enum
// ----------------------------------------------------------
//
// Every distinct "word type" the language can contain is
// represented by a variant of this enum.  Rust enums are
// perfect here because:
//   • They are algebraic data types — each variant can carry
//     its own payload (e.g. `Number(f64)`).
//   • Pattern matching (`match`) lets us exhaustively handle
//     every variant, so the compiler catches missing cases.

/// A single token produced by the lexer.
#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    // ---- Literals ----
    /// A numeric literal.  We store both integers and floats as
    /// `f64` for simplicity.
    Number(f64),

    /// A string literal.
    String(String),

    /// A user-defined name such as a variable (`x`, `result`).
    Identifier(String),

    // ---- Arithmetic Operators ----
    Plus,      // +
    Minus,     // -
    Star,      // *
    Slash,     // /

    // ---- Comparison Operators (v0.2) ----
    /// `==` — equality comparison
    EqualEqual,
    /// `!=` — inequality comparison
    BangEqual,
    /// `>` — greater than
    Greater,
    /// `<` — less than
    Less,
    /// `>=` — greater than or equal
    GreaterEqual,
    /// `<=` — less than or equal
    LessEqual,

    // ---- Punctuation ----
    Equals,     // =  (assignment)
    LeftParen,  // (
    RightParen, // )
    LeftBrace,  // {  (v0.2 — block delimiter)
    RightBrace, // }  (v0.2 — block delimiter)
    Semicolon,  // ;
    Comma,      // ,  (v0.2 — parameter separator)
    Bang,       // !  (v0.2 — logical NOT / prefix)
    Dot,        // .  (v0.5 — member access)
    LeftBracket,  // [  (v0.6 - indexing access)
    RightBracket, // ]  (v0.6 - indexing access)

    // ---- Keywords ----
    /// `let` — variable declaration
    Let,
    /// `print` — write values to stdout
    Print,
    /// `fn` — function definition (v0.2)
    Fn,
    /// `return` — return from a function (v0.2)
    Return,
    /// `if` — conditional branch (v0.2)
    If,
    /// `else` — alternative branch (v0.2)
    Else,
    /// `while` — loop (v0.2)
    While,
    /// `true` — boolean literal (v0.2)
    True,
    /// `false` — boolean literal (v0.2)
    False,
    
    /// `love` — module inclusion (v0.6)
    Love,
    /// `export` — export a value from a module (v0.6)
    Export,

    // ---- Special ----
    /// Signals the end of input.  The parser uses this to know
    /// when to stop.
    Eof,
}

// ----------------------------------------------------------
// Lexer implementation
// ----------------------------------------------------------
//
// `lex()` is a pure function: it takes a `&str` and returns
// either a `Vec<Token>` on success or a `String` error message.
//
// Internally it walks through the characters one at a time,
// deciding what kind of token each sequence represents.

use crate::error::LiabError;

/// Tokenise the given source string.
///
/// # Errors
/// Returns a human-readable error message if the input contains
/// a character that does not belong to the language.
pub fn lex(source: &str) -> Result<Vec<Token>, LiabError> {
    let mut tokens: Vec<Token> = Vec::new();
    // We work with a peekable iterator over *characters* so we
    // can look one character ahead without consuming it.
    let mut chars = source.chars().peekable();

    while let Some(&ch) = chars.peek() {
        match ch {
            // ---- Whitespace (skip) ----
            // Spaces, tabs, newlines carry no meaning in LIAB.
            ' ' | '\t' | '\n' | '\r' => {
                chars.next();
            }

            // ---- Single-character tokens ----
            '+' => { chars.next(); tokens.push(Token::Plus); }
            '-' => { chars.next(); tokens.push(Token::Minus); }
            '*' => { chars.next(); tokens.push(Token::Star); }
            '.' => { chars.next(); tokens.push(Token::Dot); }
            '/' => {
                chars.next();
                // Support single-line comments starting with //
                if chars.peek() == Some(&'/') {
                    // Consume the rest of the line
                    while let Some(&c) = chars.peek() {
                        if c == '\n' { break; }
                        chars.next();
                    }
                } else {
                    tokens.push(Token::Slash);
                }
            }

            // ---- Multi-character operators ----
            // `=` can be assignment `=` or equality `==`.
            '=' => {
                chars.next();
                if chars.peek() == Some(&'=') {
                    chars.next();
                    tokens.push(Token::EqualEqual);
                } else {
                    tokens.push(Token::Equals);
                }
            }
            // `!` can be logical NOT `!` or inequality `!=`.
            '!' => {
                chars.next();
                if chars.peek() == Some(&'=') {
                    chars.next();
                    tokens.push(Token::BangEqual);
                } else {
                    tokens.push(Token::Bang);
                }
            }
            // `>` can be `>` or `>=`.
            '>' => {
                chars.next();
                if chars.peek() == Some(&'=') {
                    chars.next();
                    tokens.push(Token::GreaterEqual);
                } else {
                    tokens.push(Token::Greater);
                }
            }
            // `<` can be `<` or `<=`.
            '<' => {
                chars.next();
                if chars.peek() == Some(&'=') {
                    chars.next();
                    tokens.push(Token::LessEqual);
                } else {
                    tokens.push(Token::Less);
                }
            }

            // ---- Remaining single-character tokens ----
            '(' => { chars.next(); tokens.push(Token::LeftParen); }
            ')' => { chars.next(); tokens.push(Token::RightParen); }
            '{' => { chars.next(); tokens.push(Token::LeftBrace); }
            '}' => { chars.next(); tokens.push(Token::RightBrace); }
            '[' => { chars.next(); tokens.push(Token::LeftBracket); }
            ']' => { chars.next(); tokens.push(Token::RightBracket); }
            ';' => { chars.next(); tokens.push(Token::Semicolon); }
            ',' => { chars.next(); tokens.push(Token::Comma); }

            // ---- Strings ----
            '"' => {
                chars.next(); // consume opening quote
                let mut s = String::new();
                while let Some(&c) = chars.peek() {
                    if c == '"' {
                        break;
                    }
                    s.push(c);
                    chars.next();
                }
                if chars.peek() == Some(&'"') {
                    chars.next(); // consume closing quote
                    tokens.push(Token::String(s));
                } else {
                    return Err(LiabError::SyntaxError("Unterminated string literal".into()));
                }
            }

            // ---- Numbers ----
            // If the character is a digit, consume the whole
            // number (including an optional decimal part).
            '0'..='9' => {
                let num = lex_number(&mut chars)?;
                tokens.push(Token::Number(num));
            }

            // ---- Identifiers & keywords ----
            // Identifiers start with a letter or underscore.
            'a'..='z' | 'A'..='Z' | '_' => {
                let word = lex_identifier(&mut chars);
                // Check if the word is a reserved keyword.
                let token = match word.as_str() {
                    "let"    => Token::Let,
                    "print"  => Token::Print,
                    "fn"     => Token::Fn,
                    "return" => Token::Return,
                    "love"   => Token::Love,
                    "export" => Token::Export,
                    "if"     => Token::If,
                    "else"   => Token::Else,
                    "while"  => Token::While,
                    "true"   => Token::True,
                    "false"  => Token::False,
                    _        => Token::Identifier(word),
                };
                tokens.push(token);
            }

            // ---- Unknown character ----
            _ => {
                let c = chars.next().unwrap();
                return Err(LiabError::SyntaxError(format!("Unknown character: '{}'", c)));
            }
        }
    }

    // Always append an Eof so the parser has a clean stop signal.
    tokens.push(Token::Eof);
    Ok(tokens)
}

// ----------------------------------------------------------
// Helper: lex a number (integer or float)
// ----------------------------------------------------------
fn lex_number(chars: &mut std::iter::Peekable<std::str::Chars>) -> Result<f64, LiabError> {
    let mut buf = String::new();

    // Consume digits before the decimal point.
    while let Some(&c) = chars.peek() {
        if c.is_ascii_digit() {
            buf.push(c);
            chars.next();
        } else {
            break;
        }
    }

    // Optional fractional part.
    if chars.peek() == Some(&'.') {
        buf.push('.');
        chars.next();

        // There must be at least one digit after the dot.
        let mut has_digit = false;
        while let Some(&c) = chars.peek() {
            if c.is_ascii_digit() {
                buf.push(c);
                chars.next();
                has_digit = true;
            } else {
                break;
            }
        }
        if !has_digit {
            return Err(LiabError::SyntaxError(format!("Invalid number literal: '{}'", buf)));
        }
    }

    buf.parse::<f64>()
        .map_err(|e| LiabError::SyntaxError(format!("Cannot parse number '{}': {}", buf, e)))
}

// ----------------------------------------------------------
// Helper: lex an identifier (letters, digits, underscores)
// ----------------------------------------------------------
fn lex_identifier(chars: &mut std::iter::Peekable<std::str::Chars>) -> String {
    let mut word = String::new();
    while let Some(&c) = chars.peek() {
        if c.is_alphanumeric() || c == '_' {
            word.push(c);
            chars.next();
        } else {
            break;
        }
    }
    word
}

// ===========================================================
// Unit tests
// ===========================================================
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lex_simple_assignment() {
        let tokens = lex("let x = 42;").unwrap();
        assert_eq!(
            tokens,
            vec![
                Token::Let,
                Token::Identifier("x".into()),
                Token::Equals,
                Token::Number(42.0),
                Token::Semicolon,
                Token::Eof,
            ]
        );
    }

    #[test]
    fn lex_float_number() {
        let tokens = lex("42.5").unwrap();
        assert_eq!(tokens, vec![Token::Number(42.5), Token::Eof]);
    }

    #[test]
    fn lex_string() {
        let tokens = lex("\"hello world\"").unwrap();
        assert_eq!(tokens, vec![Token::String("hello world".into()), Token::Eof]);
    }

    #[test]
    fn lex_dot() {
        let tokens = lex("rust.call").unwrap();
        assert_eq!(tokens, vec![
            Token::Identifier("rust".into()),
            Token::Dot,
            Token::Identifier("call".into()),
            Token::Eof
        ]);
    }

    #[test]
    fn lex_arithmetic_expression() {
        let tokens = lex("(1 + 2) * 3").unwrap();
        assert_eq!(
            tokens,
            vec![
                Token::LeftParen,
                Token::Number(1.0),
                Token::Plus,
                Token::Number(2.0),
                Token::RightParen,
                Token::Star,
                Token::Number(3.0),
                Token::Eof,
            ]
        );
    }

    #[test]
    fn lex_print_keyword() {
        let tokens = lex("print result;").unwrap();
        assert_eq!(
            tokens,
            vec![
                Token::Print,
                Token::Identifier("result".into()),
                Token::Semicolon,
                Token::Eof,
            ]
        );
    }

    #[test]
    fn lex_unknown_char() {
        let result = lex("let a = $;");
        assert!(result.is_err());
        if let Err(LiabError::SyntaxError(msg)) = result {
            assert!(msg.contains("Unknown character: '$'"));
        } else {
            panic!("Expected SyntaxError");
        }
    }

    #[test]
    fn lex_comment() {
        let tokens = lex("let x = 5; // this is a comment\nprint x;").unwrap();
        assert_eq!(
            tokens,
            vec![
                Token::Let,
                Token::Identifier("x".into()),
                Token::Equals,
                Token::Number(5.0),
                Token::Semicolon,
                Token::Print,
                Token::Identifier("x".into()),
                Token::Semicolon,
                Token::Eof,
            ]
        );
    }

    // ---- v0.2 tests ----

    #[test]
    fn lex_comparison_operators() {
        let tokens = lex("== != > < >= <=").unwrap();
        assert_eq!(
            tokens,
            vec![
                Token::EqualEqual,
                Token::BangEqual,
                Token::Greater,
                Token::Less,
                Token::GreaterEqual,
                Token::LessEqual,
                Token::Eof,
            ]
        );
    }

    #[test]
    fn lex_braces_and_comma() {
        let tokens = lex("{ a, b }").unwrap();
        assert_eq!(
            tokens,
            vec![
                Token::LeftBrace,
                Token::Identifier("a".into()),
                Token::Comma,
                Token::Identifier("b".into()),
                Token::RightBrace,
                Token::Eof,
            ]
        );
    }

    #[test]
    fn lex_keywords_v02() {
        let tokens = lex("fn return love export if else while true false").unwrap();
        assert_eq!(
            tokens,
            vec![
                Token::Fn,
                Token::Return,
                Token::Love,
                Token::Export,
                Token::If,
                Token::Else,
                Token::While,
                Token::True,
                Token::False,
                Token::Eof,
            ]
        );
    }

    #[test]
    fn lex_function_definition() {
        let tokens = lex("fn add(a, b) { return a + b; }").unwrap();
        assert_eq!(
            tokens,
            vec![
                Token::Fn,
                Token::Identifier("add".into()),
                Token::LeftParen,
                Token::Identifier("a".into()),
                Token::Comma,
                Token::Identifier("b".into()),
                Token::RightParen,
                Token::LeftBrace,
                Token::Return,
                Token::Identifier("a".into()),
                Token::Plus,
                Token::Identifier("b".into()),
                Token::Semicolon,
                Token::RightBrace,
                Token::Eof,
            ]
        );
    }

    #[test]
    fn lex_if_statement() {
        let tokens = lex("if x > 10 { print x; }").unwrap();
        assert_eq!(
            tokens,
            vec![
                Token::If,
                Token::Identifier("x".into()),
                Token::Greater,
                Token::Number(10.0),
                Token::LeftBrace,
                Token::Print,
                Token::Identifier("x".into()),
                Token::Semicolon,
                Token::RightBrace,
                Token::Eof,
            ]
        );
    }

    #[test]
    fn lex_equality_vs_assignment() {
        // Ensure `=` and `==` are correctly distinguished
        let tokens = lex("x = 5; x == 5;").unwrap();
        assert_eq!(
            tokens,
            vec![
                Token::Identifier("x".into()),
                Token::Equals,
                Token::Number(5.0),
                Token::Semicolon,
                Token::Identifier("x".into()),
                Token::EqualEqual,
                Token::Number(5.0),
                Token::Semicolon,
                Token::Eof,
            ]
        );
    }
}
