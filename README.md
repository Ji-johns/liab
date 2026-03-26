# 🔥 LIAB (Love Is A Bitch)

> A deterministic, embeddable scripting language built in Rust, powered by a custom bytecode virtual machine.

---

## 🧠 Overview

**LIAB** is a programming language designed to explore how modern scripting runtimes work under the hood.

It is built entirely from scratch in Rust and implements a complete execution pipeline:

* Lexing → Parsing → AST → Bytecode Compilation → Virtual Machine Execution

Unlike many high-level languages, LIAB focuses on **explicit execution**, **predictability**, and **minimal runtime complexity**.

---

## 🚀 Why LIAB?

Most scripting languages hide execution details behind layers of abstraction.

LIAB takes a different approach:

* **Deterministic execution** → No hidden behavior
* **Explicit syntax** → No implicit semicolon insertion or magic
* **Minimal runtime** → No garbage collector, built on Rust ownership
* **Full control** → You can understand every step from source to execution

This makes LIAB ideal for:

* Learning how programming languages work
* Experimenting with runtime design
* Building embedded scripting systems

---

## ✨ Features

### ⚙️ Language Core

* Variables and assignments
* Arithmetic and comparison operators
* Control flow (`if`, `while`)
* Functions with recursion
* Block scoping

### 🧩 Runtime System

* Stack-based Virtual Machine
* Bytecode instruction set
* Call stack & execution frames
* Structured error handling

### 📦 Module System

* `love "module"` import mechanism
* Explicit `export` declarations
* Cycle detection (safe module loading)
* Namespace-based access

### 🔗 Native Integration

* Call Rust functions directly from LIAB
* Type-safe boundary enforcement
* Controlled execution environment

---

## 🧪 Example

```liab
love "math";

fn factorial(n) {
    if n == 0 {
        return 1;
    }
    return n * factorial(n - 1);
}

print factorial(5);
```

Output:

```
120
```

---

## 🏗️ Architecture

LIAB follows a traditional language execution pipeline:

```
Source Code (.liab)
        ↓
      Lexer
        ↓
      Parser
        ↓
       AST
        ↓
Bytecode Compiler
        ↓
  Virtual Machine
        ↓
    Execution
```

### 🔹 Bytecode VM

LIAB compiles source code into a compact instruction set and executes it on a stack-based virtual machine.

This provides:

* Faster execution than tree-walk interpreters
* Better control over runtime behavior
* A clear separation between compilation and execution

---

## ⚙️ Installation

Clone the repository:

```bash
git clone https://github.com/Ji-johns/liab.git
cd liab
```

Build the project:

```bash
cargo build --release
```

(Optional) Install globally:

```bash
sudo cp target/release/liab /usr/local/bin/
```

---

## ▶️ Usage

Run a LIAB script:

```bash
liab main.liab
```

Or using Cargo:

```bash
cargo run -- main.liab
```

---

## 📁 Project Structure

```
src/
  lexer.rs         → Tokenization
  parser.rs        → Syntax analysis
  ast.rs           → Abstract Syntax Tree
  compiler.rs      → Bytecode generation
  vm.rs            → Virtual Machine

examples/
  hello.liab
  factorial.liab
  modules.liab

docs/
  language.md
  architecture.md
```

---

## 🧠 Design Philosophy

LIAB is built on a few core principles:

* **Explicit over implicit**
* **Control over convenience**
* **Understandability over abstraction**
* **Small, composable runtime pieces**

Every feature added to the language must justify its complexity.

---

## 🚧 Roadmap

* [ ] Modulus operator (`%`)
* [ ] REPL (interactive shell)
* [ ] Debugger / tracing improvements
* [ ] Module caching & resolution improvements
* [ ] Standard library
* [ ] WASM compilation target

---

## 🤝 Contributing

Contributions are welcome.

If you’re interested in language design, compilers, or virtual machines, feel free to open issues or submit pull requests.

---

## 📄 License

MIT License

---

## ⭐ Final Note

LIAB is not just a language — it’s an exploration of how programming languages are built.

If you find it interesting, consider giving it a ⭐ on GitHub.
