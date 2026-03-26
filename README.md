# 🔥 LIAB (Love Is A Bitch)

> A deterministic scripting language powered by a custom bytecode virtual machine written in Rust.

---

## 🚀 Demo

```bash
liab examples/factorial.liab
```

```
120
```

---

## ✨ Features

* ⚙️ Custom Bytecode VM
* 🧠 Lexer + Parser + AST
* 📦 Module system (`love`)
* 🔗 Native Rust function integration
* 🔁 Recursion & control flow
* 🧩 Namespaces & member access
* ❌ No garbage collection (Rust ownership)

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

---

## 🏗️ Architecture

```
Source Code
   ↓
Lexer → Parser → AST
   ↓
Bytecode Compiler
   ↓
Virtual Machine
   ↓
Execution
```

---

## ⚙️ Installation

```bash
git clone https://github.com/Ji-johns/liab.git
cd liab
cargo build --release
```

---

## ▶️ Usage

```bash
./target/release/liab main.liab
```

or (after installing globally):

```bash
liab main.liab
```

---

## 📁 Project Structure

```
src/            → core language implementation
examples/       → runnable programs
docs/           → documentation
```

---

## 🗺️ Roadmap

* [ ] Modulus operator (%)
* [ ] REPL
* [ ] Debugger
* [ ] Package manager
* [ ] WASM target

---

## 🤝 Contributing

Contributions are welcome.

---

## 📄 License

MIT License
