// ============================================================
// LIAB Language — Parser
// ============================================================
//
// The parser transforms the flat token stream from the lexer
// into a tree of AST nodes that capture the program's
// structure and operator precedence.
//
// We use a **recursive-descent** parser — one of the simplest
// and most intuitive parsing strategies:
//
//   parse_program        → list of statements
//     parse_statement     → fn, if, while, return, let, print,
//                           assignment, or expression stmt
//       parse_expression  → delegates to parse_comparison
//         parse_comparison → handles ==, !=, >, <, >=, <=
//           parse_addition → handles + and -
//             parse_term   → handles * and /
//               parse_factor → parens, atoms, booleans,
//                              function calls, unary minus
//
// This nesting naturally encodes precedence:
//   comparisons bind looser than arithmetic.
// ============================================================

use crate::ast::{BinOp, Expr, Stmt};
use crate::lexer::Token;

/// The parser state — wraps the token list and a cursor.
pub struct Parser {
    tokens: Vec<Token>,
    /// Current position in the token list.
    pos: usize,
}

impl Parser {
    // ----------------------------------------------------------
    // Construction
    // ----------------------------------------------------------
    pub fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, pos: 0 }
    }

    // ----------------------------------------------------------
    // Helpers
    // ----------------------------------------------------------

    /// Look at the current token without consuming it.
    fn peek(&self) -> &Token {
        self.tokens.get(self.pos).unwrap_or(&Token::Eof)
    }

    /// Look at the token `offset` positions ahead (0 = current).
    fn peek_at(&self, offset: usize) -> &Token {
        self.tokens.get(self.pos + offset).unwrap_or(&Token::Eof)
    }

    /// Consume the current token and advance the cursor.
    fn advance(&mut self) -> Token {
        let tok = self.tokens.get(self.pos).cloned().unwrap_or(Token::Eof);
        self.pos += 1;
        tok
    }

    /// Consume the current token only if it matches `expected`.
    /// Returns an error message otherwise.
    fn expect(&mut self, expected: &Token) -> Result<(), crate::error::LiabError> {
        let tok = self.advance();
        if &tok == expected {
            Ok(())
        } else {
            Err(crate::error::LiabError::SyntaxError(format!("Expected {:?}, found {:?}", expected, tok)))
        }
    }

    /// Check if the current token matches `expected` without consuming.
    fn check(&self, expected: &Token) -> bool {
        self.peek() == expected
    }

    /// Consume the current token exclusively mapping exact syntax error overrides if missing.
    fn expect_semicolon(&mut self) -> Result<(), crate::error::LiabError> {
        let tok = self.advance();
        if matches!(tok, Token::Semicolon) {
            Ok(())
        } else {
            Err(crate::error::LiabError::SyntaxError("Expected ';' after statement".to_string()))
        }
    }

    // ----------------------------------------------------------
    // Public entry point
    // ----------------------------------------------------------

    /// Parse the entire token stream into a list of statements.
    pub fn parse(&mut self) -> Result<Vec<Stmt>, crate::error::LiabError> {
        let mut stmts = Vec::new();

        // Keep parsing statements until we reach the end-of-file
        // token.
        while *self.peek() != Token::Eof {
            stmts.push(self.parse_statement()?);
        }

        Ok(stmts)
    }

    // ----------------------------------------------------------
    // Statement parsing
    // ----------------------------------------------------------

    fn parse_statement(&mut self) -> Result<Stmt, crate::error::LiabError> {
        match self.peek().clone() {
            // ---- fn <name>(<params>) { <body> } ----
            Token::Fn => self.parse_function_def(),

            // ---- if <expr> { <body> } [else { <body> }] ----
            Token::If => self.parse_if(),

            // ---- while <expr> { <body> } ----
            Token::While => self.parse_while(),

            // ---- return <expr>; ----
            Token::Return => self.parse_return(),

            // ---- let <name> = <expr>; ----
            Token::Let => {
                self.advance(); // consume `let`

                // The next token must be an identifier.
                let name = match self.advance() {
                    Token::Identifier(n) => n,
                    other => return Err(crate::error::LiabError::SyntaxError(format!(
                        "Expected variable name after 'let', found {:?}", other
                    ))),
                };

                self.expect(&Token::Equals)?;
                let value = self.parse_expression()?;
                self.expect_semicolon()?;

                Ok(Stmt::Let { name, value })
            }

            // ---- print <expr>; ----
            Token::Print => {
                self.advance(); // consume `print`
                let expr = self.parse_expression()?;
                self.expect_semicolon()?;
                Ok(Stmt::Print(expr))
            }

            // ---- love "path"; ----
            Token::Love => self.parse_love_statement(),

            // ---- export ident; ----
            Token::Export => self.parse_export_statement(),

            // ---- { <stmts> } ----
            Token::LeftBrace => {
                let body = self.parse_block()?;
                Ok(Stmt::Block(body))
            }

            // ---- assignment or expression statement ----
            // If we see `<identifier> =` (but NOT `==`), it's an
            // assignment.  Otherwise it's an expression statement.
            Token::Identifier(_) => {
                // Peek ahead: is the *next* token `=` (but not `==`)?
                if *self.peek_at(1) == Token::Equals {
                    self.parse_assignment()
                } else {
                    let expr = self.parse_expression()?;
                    self.expect_semicolon()?;
                    Ok(Stmt::Expression(expr))
                }
            }

            // ---- expression statement: <expr>; ----
            _ => {
                let expr = self.parse_expression()?;
                self.expect_semicolon()?;
                Ok(Stmt::Expression(expr))
            }
        }
    }

    // ----------------------------------------------------------
    // Compound statement parsers
    // ----------------------------------------------------------

    /// Parse: `fn <name>(<params>) { <body> }`
    fn parse_function_def(&mut self) -> Result<Stmt, crate::error::LiabError> {
        self.advance(); // consume `fn`

        let name = match self.advance() {
            Token::Identifier(n) => n,
            other => return Err(crate::error::LiabError::SyntaxError(format!(
                "Expected function name after 'fn', found {:?}", other
            ))),
        };

        // Parse parameter list
        self.expect(&Token::LeftParen)?;
        let mut params = Vec::new();
        if !self.check(&Token::RightParen) {
            // First parameter
            match self.advance() {
                Token::Identifier(p) => params.push(p),
                other => return Err(crate::error::LiabError::SyntaxError(format!(
                    "Expected parameter name, found {:?}", other
                ))),
            }
            // Remaining parameters
            while self.check(&Token::Comma) {
                self.advance(); // consume `,`
                match self.advance() {
                    Token::Identifier(p) => params.push(p),
                    other => return Err(crate::error::LiabError::SyntaxError(format!(
                        "Expected parameter name after ',', found {:?}", other
                    ))),
                }
            }
        }
        self.expect(&Token::RightParen)?;

        // Parse body
        let body = self.parse_block()?;

        Ok(Stmt::FunctionDef { name, params, body })
    }

    /// Parse: `if <expr> { <body> } [else { <body> }]`
    fn parse_if(&mut self) -> Result<Stmt, crate::error::LiabError> {
        self.advance(); // consume `if`

        let condition = self.parse_expression()?;
        let then_body = self.parse_block()?;

        let else_body = if self.check(&Token::Else) {
            self.advance(); // consume `else`
            // Support `else if` by treating `if` as the only
            // statement in the else block.
            if self.check(&Token::If) {
                let else_if = self.parse_if()?;
                Some(vec![else_if])
            } else {
                Some(self.parse_block()?)
            }
        } else {
            None
        };

        Ok(Stmt::If { condition, then_body, else_body })
    }

    /// Parse: `while <expr> { <body> }`
    fn parse_while(&mut self) -> Result<Stmt, crate::error::LiabError> {
        self.advance(); // consume `while`

        let condition = self.parse_expression()?;
        let body = self.parse_block()?;

        Ok(Stmt::While { condition, body })
    }

    /// Parse: `return <expr>;`
    fn parse_return(&mut self) -> Result<Stmt, crate::error::LiabError> {
        self.advance(); // consume `return`

        let expr = self.parse_expression()?;
        self.expect_semicolon()?;

        Ok(Stmt::Return(expr))
    }

    /// Parse: `<name> = <expr>;`
    fn parse_assignment(&mut self) -> Result<Stmt, crate::error::LiabError> {
        let name = match self.advance() {
            Token::Identifier(n) => n,
            other => return Err(crate::error::LiabError::SyntaxError(format!(
                "Expected variable name for assignment, found {:?}", other
            ))),
        };

        self.expect(&Token::Equals)?;
        let value = self.parse_expression()?;
        self.expect_semicolon()?;

        Ok(Stmt::Assignment { name, value })
    }

    /// Parse a block: `{ <stmt>* }`
    fn parse_block(&mut self) -> Result<Vec<Stmt>, crate::error::LiabError> {
        self.expect(&Token::LeftBrace)?;

        let mut stmts = Vec::new();
        while !self.check(&Token::RightBrace) && !self.check(&Token::Eof) {
            stmts.push(self.parse_statement()?);
        }

        self.expect(&Token::RightBrace)?;
        Ok(stmts)
    }

    /// Parse: `love "path";`
    fn parse_love_statement(&mut self) -> Result<Stmt, crate::error::LiabError> {
        self.expect(&Token::Love)?;
        let path = match self.advance() {
            Token::String(s) => s,
            other => return Err(crate::error::LiabError::SyntaxError(format!("Expected module string path after 'love', found {:?}", other))),
        };
        self.expect_semicolon()?;
        Ok(Stmt::Love(path))
    }

    fn parse_export_statement(&mut self) -> Result<Stmt, crate::error::LiabError> {
        self.expect(&Token::Export)?;
        let name = match self.advance() {
            Token::Identifier(s) => s,
            other => return Err(crate::error::LiabError::SyntaxError(format!("Expected identifier after 'export', found {:?}", other))),
        };
        self.expect_semicolon()?;
        Ok(Stmt::Export(name))
    }

    // ----------------------------------------------------------
    // Expression parsing (precedence climbing)
    // ----------------------------------------------------------

    /// Entry point for expression parsing — delegates to the
    /// lowest-precedence level.
    fn parse_expression(&mut self) -> Result<Expr, crate::error::LiabError> {
        self.parse_comparison()
    }

    /// Lowest precedence: comparison operators.
    fn parse_comparison(&mut self) -> Result<Expr, crate::error::LiabError> {
        let mut left = self.parse_addition()?;

        loop {
            let op = match self.peek() {
                Token::EqualEqual   => BinOp::Eq,
                Token::BangEqual    => BinOp::Neq,
                Token::Greater      => BinOp::Gt,
                Token::Less         => BinOp::Lt,
                Token::GreaterEqual => BinOp::Gte,
                Token::LessEqual    => BinOp::Lte,
                _ => break,
            };
            self.advance(); // consume the operator
            let right = self.parse_addition()?;
            left = Expr::BinaryOp {
                left: Box::new(left),
                op,
                right: Box::new(right),
            };
        }

        Ok(left)
    }

    /// Addition and subtraction.
    fn parse_addition(&mut self) -> Result<Expr, crate::error::LiabError> {
        let mut left = self.parse_term()?;

        loop {
            let op = match self.peek() {
                Token::Plus  => BinOp::Add,
                Token::Minus => BinOp::Sub,
                _ => break,
            };
            self.advance(); // consume the operator
            let right = self.parse_term()?;
            left = Expr::BinaryOp {
                left: Box::new(left),
                op,
                right: Box::new(right),
            };
        }

        Ok(left)
    }

    /// Multiplication and division.
    fn parse_term(&mut self) -> Result<Expr, crate::error::LiabError> {
        let mut left = self.parse_factor()?;

        loop {
            let op = match self.peek() {
                Token::Star  => BinOp::Mul,
                Token::Slash => BinOp::Div,
                _ => break,
            };
            self.advance();
            let right = self.parse_factor()?;
            left = Expr::BinaryOp {
                left: Box::new(left),
                op,
                right: Box::new(right),
            };
        }

        Ok(left)
    }

    /// Highest precedence: parenthesised groups, literals,
    /// identifiers, booleans, and function calls.
    fn parse_factor(&mut self) -> Result<Expr, crate::error::LiabError> {
        match self.peek().clone() {
            // ---- ( <expr> ) ----
            Token::LeftParen => {
                self.advance(); // consume `(`
                let expr = self.parse_expression()?;
                self.expect(&Token::RightParen)?;
                Ok(expr)
            }

            // ---- numeric literal ----
            Token::Number(n) => {
                self.advance();
                Ok(Expr::Number(n))
            }

            // ---- string literal ----
            Token::String(ref s) => {
                let s_clone = s.clone();
                self.advance();
                Ok(Expr::String(s_clone))
            }

            // ---- boolean literals ----
            Token::True => {
                self.advance();
                Ok(Expr::Boolean(true))
            }
            Token::False => {
                self.advance();
                Ok(Expr::Boolean(false))
            }

            // ---- identifier, member access, or function call ----
            Token::Identifier(name) => {
                self.advance();
                let mut expr = Expr::Identifier(name);

                // Allow chained `.property` or `(args)`
                loop {
                    if self.check(&Token::Dot) {
                        self.advance(); // consume `.`
                        let property = match self.advance() {
                            Token::Identifier(n) => n,
                            other => return Err(crate::error::LiabError::SyntaxError(format!("Expected property name after '.', found {:?}", other))),
                        };
                        expr = Expr::Member {
                            object: Box::new(expr),
                            property,
                        };
                    } else if self.check(&Token::LeftBracket) {
                        self.advance(); // consume '['
                        let index = self.parse_expression()?;
                        self.expect(&Token::RightBracket)?;
                        expr = Expr::Index {
                            object: Box::new(expr),
                            index: Box::new(index),
                        };
                    } else if self.check(&Token::LeftParen) {
                        self.advance(); // consume `(`
                        let mut args = Vec::new();

                        if !self.check(&Token::RightParen) {
                            // First argument
                            args.push(self.parse_expression()?);
                            // Remaining arguments
                            while self.check(&Token::Comma) {
                                self.advance(); // consume `,`
                                args.push(self.parse_expression()?);
                            }
                        }

                        self.expect(&Token::RightParen)?;
                        expr = Expr::FunctionCall {
                            callee: Box::new(expr),
                            args,
                        };
                    } else {
                        break;
                    }
                }

                Ok(expr)
            }

            // ---- unary minus (negative numbers) ----
            Token::Minus => {
                self.advance(); // consume `-`
                let expr = self.parse_factor()?;
                // Represent `-x` as `0 - x`
                Ok(Expr::BinaryOp {
                    left: Box::new(Expr::Number(0.0)),
                    op: BinOp::Sub,
                    right: Box::new(expr),
                })
            }

            other => Err(crate::error::LiabError::SyntaxError(format!("Unexpected token: {:?}", other))),
        }
    }
}

// ===========================================================
// Convenience function
// ===========================================================

/// Parse a token list into a program (list of statements).
pub fn parse(tokens: Vec<Token>) -> Result<Vec<Stmt>, crate::error::LiabError> {
    let mut parser = Parser::new(tokens);
    parser.parse()
}

// ===========================================================
// Unit tests
// ===========================================================
#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::lex;

    #[test]
    fn parse_let_statement() {
        let tokens = lex("let x = 42;").unwrap();
        let stmts = parse(tokens).unwrap();
        assert_eq!(stmts.len(), 1);
        match &stmts[0] {
            Stmt::Let { name, value } => {
                assert_eq!(name, "x");
                assert_eq!(*value, Expr::Number(42.0));
            }
            other => panic!("Expected Let statement, got {:?}", other),
        }
    }

    #[test]
    fn parse_print_expression() {
        let tokens = lex("print 1 + 2;").unwrap();
        let stmts = parse(tokens).unwrap();
        assert_eq!(stmts.len(), 1);
        assert!(matches!(&stmts[0], Stmt::Print(_)));
    }

    #[test]
    fn parse_operator_precedence() {
        // 1 + 2 * 3 should parse as 1 + (2 * 3), not (1 + 2) * 3
        let tokens = lex("1 + 2 * 3;").unwrap();
        let stmts = parse(tokens).unwrap();
        match &stmts[0] {
            Stmt::Expression(Expr::BinaryOp { op, .. }) => {
                assert_eq!(*op, BinOp::Add); // top-level is Add
            }
            other => panic!("Expected BinaryOp, got {:?}", other),
        }
    }

    #[test]
    fn parse_parenthesised_expression() {
        let tokens = lex("(1 + 2) * 3;").unwrap();
        let stmts = parse(tokens).unwrap();
        match &stmts[0] {
            Stmt::Expression(Expr::BinaryOp { op, .. }) => {
                assert_eq!(*op, BinOp::Mul); // top-level is Mul
            }
            other => panic!("Expected BinaryOp, got {:?}", other),
        }
    }

    // ---- v0.2 tests ----

    #[test]
    fn parse_function_def() {
        let tokens = lex("fn add(a, b) { return a + b; }").unwrap();
        let stmts = parse(tokens).unwrap();
        assert_eq!(stmts.len(), 1);
        match &stmts[0] {
            Stmt::FunctionDef { name, params, body } => {
                assert_eq!(name, "add");
                assert_eq!(params, &["a", "b"]);
                assert_eq!(body.len(), 1);
                assert!(matches!(&body[0], Stmt::Return(_)));
            }
            other => panic!("Expected FunctionDef, got {:?}", other),
        }
    }

    #[test]
    fn parse_function_call() {
        let tokens = lex("let r = add(2, 3);").unwrap();
        let stmts = parse(tokens).unwrap();
        match &stmts[0] {
            Stmt::Let { value: Expr::FunctionCall { callee, args }, .. } => {
                match callee.as_ref() {
                    Expr::Identifier(name) => assert_eq!(name, "add"),
                    _ => panic!("Expected Identifier callee"),
                }
                assert_eq!(args.len(), 2);
            }
            other => panic!("Expected function call in let, got {:?}", other),
        }
    }

    #[test]
    fn parse_if_statement() {
        let tokens = lex("if x > 10 { print x; }").unwrap();
        let stmts = parse(tokens).unwrap();
        assert_eq!(stmts.len(), 1);
        match &stmts[0] {
            Stmt::If { then_body, else_body, .. } => {
                assert_eq!(then_body.len(), 1);
                assert!(else_body.is_none());
            }
            other => panic!("Expected If, got {:?}", other),
        }
    }

    #[test]
    fn parse_if_else_statement() {
        let tokens = lex("if x > 10 { print x; } else { print 0; }").unwrap();
        let stmts = parse(tokens).unwrap();
        match &stmts[0] {
            Stmt::If { else_body: Some(body), .. } => {
                assert_eq!(body.len(), 1);
            }
            other => panic!("Expected If/Else, got {:?}", other),
        }
    }

    #[test]
    fn parse_while_loop() {
        let tokens = lex("while x < 10 { x = x + 1; }").unwrap();
        let stmts = parse(tokens).unwrap();
        assert_eq!(stmts.len(), 1);
        match &stmts[0] {
            Stmt::While { body, .. } => {
                assert_eq!(body.len(), 1);
                assert!(matches!(&body[0], Stmt::Assignment { .. }));
            }
            other => panic!("Expected While, got {:?}", other),
        }
    }

    #[test]
    fn parse_assignment() {
        let tokens = lex("x = 42;").unwrap();
        let stmts = parse(tokens).unwrap();
        match &stmts[0] {
            Stmt::Assignment { name, value } => {
                assert_eq!(name, "x");
                assert_eq!(*value, Expr::Number(42.0));
            }
            other => panic!("Expected Assignment, got {:?}", other),
        }
    }

    #[test]
    fn parse_boolean_literals() {
        let tokens = lex("let a = true; let b = false;").unwrap();
        let stmts = parse(tokens).unwrap();
        assert_eq!(stmts.len(), 2);
        match &stmts[0] {
            Stmt::Let { value: Expr::Boolean(true), .. } => {}
            other => panic!("Expected Boolean(true), got {:?}", other),
        }
        match &stmts[1] {
            Stmt::Let { value: Expr::Boolean(false), .. } => {}
            other => panic!("Expected Boolean(false), got {:?}", other),
        }
    }

    #[test]
    fn parse_comparison_expression() {
        let tokens = lex("1 + 2 > 3;").unwrap();
        let stmts = parse(tokens).unwrap();
        // Top-level should be Gt, with left being Add
        match &stmts[0] {
            Stmt::Expression(Expr::BinaryOp { op, left, .. }) => {
                assert_eq!(*op, BinOp::Gt);
                match left.as_ref() {
                    Expr::BinaryOp { op: inner_op, .. } => {
                        assert_eq!(*inner_op, BinOp::Add);
                    }
                    other => panic!("Expected inner Add, got {:?}", other),
                }
            }
            other => panic!("Expected BinaryOp, got {:?}", other),
        }
    }

    #[test]
    fn test_valid_explicit_semicolons() {
        let source = "
            let x = 10;
            print x;
        ";
        let tokens = lex(source).unwrap();
        let stmts = parse(tokens).unwrap();
        assert_eq!(stmts.len(), 2);
    }

    #[test]
    fn test_invalid_missing_semicolon_newline() {
        let source = "
            let x = 10
            print x
        ";
        let tokens = lex(source).unwrap();
        let result = parse(tokens);
        
        assert!(result.is_err());
        if let Err(crate::error::LiabError::SyntaxError(msg)) = result {
            assert_eq!(msg, "Expected ';' after statement");
        } else {
            panic!("Expected SyntaxError for missing semicolon");
        }
    }
    
    #[test]
    fn test_valid_control_flow_blocks_no_semicolon() {
        let source = "
            if x > 10 {
                print x;
            }
            let y = 5;
        ";
        let tokens = lex(source).unwrap();
        assert!(parse(tokens).is_ok());
    }
}

