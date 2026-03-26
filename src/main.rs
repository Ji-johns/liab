// ============================================================
// LIAB Language — CLI Entry Point
// ============================================================
//
// This is the main binary that ties everything together.
//
// Usage:
//     cargo run -- <file.liab>
//
// Pipeline (v0.3):
//     1. Read the source file
//     2. Lex it into tokens
//     3. Parse tokens into an AST
//     4. Compile AST into bytecode
//     5. Execute bytecode in the VM
//
// Errors at any stage are printed to stderr and the process
// exits with code 1.
// ============================================================

mod ast;
mod compiler;
mod disassembler;
#[allow(dead_code)]
mod environment;      // kept for reference (v0.2 tree-walk)
mod error;
#[allow(dead_code)]
mod interpreter;      // kept for reference (v0.2 tree-walk)
mod lexer;
mod parser;
mod value;
mod vm;

use std::env;
use std::fs;
use std::process;

fn main() {
    // ---- Parse CLI arguments ----
    let mut args: Vec<String> = env::args().collect();
    args.remove(0); // Remove program name

    let mut disassemble = false;
    let mut trace = false;
    let mut filename = String::new();

    for arg in args {
        match arg.as_str() {
            "--disassemble" => disassemble = true,
            "--trace" => trace = true,
            _ if arg.starts_with("--") => {
                eprintln!("Unknown option: {}", arg);
                process::exit(1);
            }
            _ => {
                if filename.is_empty() {
                    filename = arg;
                } else {
                    eprintln!("Usage: liab [--disassemble] [--trace] <file.liab>");
                    process::exit(1);
                }
            }
        }
    }

    if filename.is_empty() {
        eprintln!("Usage: liab [--disassemble] [--trace] <file.liab>");
        process::exit(1);
    }

    // ---- Read source file ----
    let source = match fs::read_to_string(&filename) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Error reading '{}': {}", filename, e);
            process::exit(1);
        }
    };

    // ---- Run the pipeline ----
    if let Err(e) = run(&source, disassemble, trace) {
        eprintln!("Error: {}", e);
        process::exit(1);
    }
}

/// Run the full LIAB pipeline: lex → parse → compile → VM run.
fn run(source: &str, disassemble: bool, trace: bool) -> Result<(), crate::error::LiabError> {
    // Step 1: Lexing
    let tokens = lexer::lex(source)?;

    // Step 2: Parsing
    let stmts = parser::parse(tokens)?;

    // Step 3: Compile to bytecode
    let compiler = compiler::Compiler::new();
    let chunk = compiler.compile(stmts)?;

    if disassemble {
        println!("{}", disassembler::disassemble_chunk(&chunk, "<script>"));
    }

    // Step 4: Execute in the VM
    let mut vm = vm::VM::new();
    vm.trace = trace;
    vm.run(chunk)?;

    Ok(())
}
