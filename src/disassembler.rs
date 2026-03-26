// ============================================================
// LIAB Language — Bytecode Disassembler (v0.4)
// ============================================================
//
// The disassembler converts a compiled `Chunk` back into a
// human-readable listing.  This is invaluable for debugging
// the compiler and understanding what bytecode your LIAB
// program produces.
//
// Output format:
//
//   == <label> ==
//   0000  Constant        0    ; 42
//   0001  Add
//   0002  Print
//   ...
//
// For `CompiledFunction` constants, the function's chunk is
// recursively disassembled inline.
// ============================================================

use std::fmt::Write;

use crate::compiler::{Chunk, Instruction};
use crate::value::Value;

/// Disassemble a chunk into a human-readable string.
///
/// `label` is the name shown in the header (e.g. `"<script>"`
/// or the function name).
pub fn disassemble_chunk(chunk: &Chunk, label: &str) -> String {
    let mut out = String::new();
    writeln!(out, "== {} ==", label).unwrap();

    for (i, instr) in chunk.code.iter().enumerate() {
        write!(out, "{:04}  ", i).unwrap();
        match instr {
            // ---- Constants ----
            Instruction::Constant(idx) => {
                let val = &chunk.constants[*idx];
                writeln!(out, "{:<16} {:>4}    ; {}", "Constant", idx, val).unwrap();
            }

            // ---- Arithmetic ----
            Instruction::Add => writeln!(out, "Add").unwrap(),
            Instruction::Sub => writeln!(out, "Sub").unwrap(),
            Instruction::Mul => writeln!(out, "Mul").unwrap(),
            Instruction::Div => writeln!(out, "Div").unwrap(),

            // ---- Comparison ----
            Instruction::Eq  => writeln!(out, "Eq").unwrap(),
            Instruction::Neq => writeln!(out, "Neq").unwrap(),
            Instruction::Gt  => writeln!(out, "Gt").unwrap(),
            Instruction::Lt  => writeln!(out, "Lt").unwrap(),
            Instruction::Gte => writeln!(out, "Gte").unwrap(),
            Instruction::Lte => writeln!(out, "Lte").unwrap(),

            // ---- Stack ----
            Instruction::Pop   => writeln!(out, "Pop").unwrap(),
            Instruction::Print => writeln!(out, "Print").unwrap(),

            // ---- Globals ----
            Instruction::DefineGlobal(idx) => {
                let name = &chunk.constants[*idx];
                writeln!(out, "{:<16} {:>4}    ; '{}'", "DefineGlobal", idx, name).unwrap();
            }
            Instruction::GetGlobal(idx) => {
                let name = &chunk.constants[*idx];
                writeln!(out, "{:<16} {:>4}    ; '{}'", "GetGlobal", idx, name).unwrap();
            }
            Instruction::SetGlobal(idx) => {
                let name = &chunk.constants[*idx];
                writeln!(out, "{:<16} {:>4}    ; '{}'", "SetGlobal", idx, name).unwrap();
            }
            Instruction::GetProperty(idx) => {
                let name = &chunk.constants[*idx];
                writeln!(out, "{:<16} {:>4}    ; '{}'", "GetProperty", idx, name).unwrap();
            }

            Instruction::GetIndex => writeln!(out, "GetIndex").unwrap(),

            // ---- Operations ----
            Instruction::GetLocal(slot) => {
                writeln!(out, "{:<16} {}", "GetLocal", slot).unwrap();
            }
            Instruction::SetLocal(slot) => {
                writeln!(out, "{:<16} {}", "SetLocal", slot).unwrap();
            }

            // ---- Control flow ----
            Instruction::Jump(target) => {
                writeln!(out, "{:<16} -> {:04}", "Jump", target).unwrap();
            }
            Instruction::JumpIfFalse(target) => {
                writeln!(out, "{:<16} -> {:04}", "JumpIfFalse", target).unwrap();
            }

            // ---- Functions ----
            Instruction::Call(arity) => {
                writeln!(out, "{:<16} ({})", "Call", arity).unwrap();
            }
            Instruction::Return => writeln!(out, "Return").unwrap(),

            // ---- Modules ----
            Instruction::Love(idx) => {
                let path = &chunk.constants[*idx];
                writeln!(out, "{:<16} {:>4}    ; '{}'", "Love", idx, path).unwrap();
            }
            Instruction::Export(idx) => {
                let name = &chunk.constants[*idx];
                writeln!(out, "{:<16} {:>4}    ; '{}'", "Export", idx, name).unwrap();
            }
        }
    }

    // Recursively disassemble any compiled functions in the
    // constant pool.
    for val in &chunk.constants {
        if let Value::CompiledFunction { name, chunk: fn_chunk, .. } = val {
            writeln!(out).unwrap();
            out.push_str(&disassemble_chunk(fn_chunk, &format!("fn {}", name)));
        }
    }

    out
}

/// Format a single instruction (one line, no newline).
/// Used by the VM's trace mode.
pub fn disassemble_instruction(chunk: &Chunk, offset: usize) -> String {
    let instr = &chunk.code[offset];
    let mut out = format!("{:04}  ", offset);
    match instr {
        Instruction::Constant(idx) => {
            let val = &chunk.constants[*idx];
            write!(out, "{:<16} {:>4}    ; {}", "Constant", idx, val).unwrap();
        }
        Instruction::Add => write!(out, "Add").unwrap(),
        Instruction::Sub => write!(out, "Sub").unwrap(),
        Instruction::Mul => write!(out, "Mul").unwrap(),
        Instruction::Div => write!(out, "Div").unwrap(),
        Instruction::Eq  => write!(out, "Eq").unwrap(),
        Instruction::Neq => write!(out, "Neq").unwrap(),
        Instruction::Gt  => write!(out, "Gt").unwrap(),
        Instruction::Lt  => write!(out, "Lt").unwrap(),
        Instruction::Gte => write!(out, "Gte").unwrap(),
        Instruction::Lte => write!(out, "Lte").unwrap(),
        Instruction::Pop   => write!(out, "Pop").unwrap(),
        Instruction::Print => write!(out, "Print").unwrap(),
        Instruction::DefineGlobal(idx) => {
            let name = &chunk.constants[*idx];
            write!(out, "{:<16} {:>4}    ; '{}'", "DefineGlobal", idx, name).unwrap();
        }
        Instruction::GetGlobal(idx) => {
            let name = &chunk.constants[*idx];
            write!(out, "{:<16} {:>4}    ; '{}'", "GetGlobal", idx, name).unwrap();
        }
        Instruction::SetGlobal(idx) => {
            let name = &chunk.constants[*idx];
            write!(out, "{:<16} {:>4}    ; '{}'", "SetGlobal", idx, name).unwrap();
        }
        Instruction::GetProperty(idx) => {
            let name = &chunk.constants[*idx];
            write!(out, "{:<16} {:>4}    ; '{}'", "GetProperty", idx, name).unwrap();
        }
        Instruction::GetIndex => {
            write!(out, "GetIndex").unwrap();
        }
        Instruction::GetLocal(slot) => {
            write!(out, "{:<16} {}", "GetLocal", slot).unwrap();
        }
        Instruction::SetLocal(slot) => {
            write!(out, "{:<16} {}", "SetLocal", slot).unwrap();
        }
        Instruction::Jump(target) => {
            write!(out, "{:<16} -> {:04}", "Jump", target).unwrap();
        }
        Instruction::JumpIfFalse(target) => {
            write!(out, "{:<16} -> {:04}", "JumpIfFalse", target).unwrap();
        }
        Instruction::Call(arity) => {
            write!(out, "{:<16} ({})", "Call", arity).unwrap();
        }
        Instruction::Return => write!(out, "Return").unwrap(),
        Instruction::Love(idx) => {
            let path = &chunk.constants[*idx];
            write!(out, "{:<16} {:>4}    ; '{}'", "Love", idx, path).unwrap();
        }
        Instruction::Export(idx) => {
            let name = &chunk.constants[*idx];
            write!(out, "{:<16} {:>4}    ; '{}'", "Export", idx, name).unwrap();
        }
    }
    out
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

    fn compile(source: &str) -> Chunk {
        let tokens = lex(source).unwrap();
        let stmts = parse(tokens).unwrap();
        Compiler::new().compile(stmts).unwrap()
    }

    #[test]
    fn test_disassemble_simple() {
        let chunk = compile("let x = 42; print x;");
        let output = disassemble_chunk(&chunk, "<script>");
        assert!(output.contains("== <script> =="));
        assert!(output.contains("Constant"));
        assert!(output.contains("42"));
        assert!(output.contains("DefineGlobal"));
        assert!(output.contains("Print"));
        assert!(output.contains("Return"));
    }

    #[test]
    fn test_disassemble_function() {
        let chunk = compile("fn add(a, b) { return a + b; }");
        let output = disassemble_chunk(&chunk, "<script>");
        // Should have the script-level listing AND a nested
        // "== fn add ==" section.
        assert!(output.contains("== fn add =="));
        assert!(output.contains("GetLocal"));
        assert!(output.contains("Add"));
    }

    #[test]
    fn test_disassemble_jumps() {
        let chunk = compile("if true { print 1; }");
        let output = disassemble_chunk(&chunk, "<script>");
        assert!(output.contains("JumpIfFalse"));
    }

    #[test]
    fn test_disassemble_instruction_single() {
        let chunk = compile("42;");
        let line = disassemble_instruction(&chunk, 0);
        assert!(line.contains("Constant"));
        assert!(line.contains("42"));
    }
}
