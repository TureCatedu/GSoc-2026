# Scarpe-TUI (Proof of Concept) 👞🖥️

![Ruby](https://img.shields.io/badge/Ruby-CC342D?style=for-the-badge&logo=ruby&logoColor=white)
![Rust](https://img.shields.io/badge/Rust-000000?style=for-the-badge&logo=rust&logoColor=white)
![GSoC 2026](https://img.shields.io/badge/GSoC-2026-FABB19?style=for-the-badge)

A high-performance, memory-safe Text User Interface (TUI) backend for [Scarpe](https://github.com/scarpe-team/scarpe) (the Ruby Shoes revival). 

This repository serves as a **Proof of Concept (PoC)** for my Google Summer of Code (GSoC) 2026 proposal. It demonstrates the feasibility of bridging a declarative Ruby DSL with a blazing-fast Rust terminal rendering engine via C-FFI.

## ✨ Features Currently Implemented

* **Shoes-like Declarative DSL:** Build UIs using familiar concepts (`stack`, `flow`, `para`, `button`, `edit_line`).
* **Rust Rendering Engine:** Powered by `ratatui` and `crossterm` for 60fps terminal rendering.
* **Differential Rendering:** Utilizes double-buffering (via `Clear` widgets) to ensure zero flickering or text overlapping during state updates.
* **Full Mouse & Keyboard Support:** Click buttons, focus input fields, and scroll through overflowing content natively.
* **Memory-Safe FFI Boundary:** Dynamic string allocation and deallocation across the C-ABI to prevent Ruby memory leaks.
* **Crash Prevention (Clipping):** Intelligent viewport bounding-box calculations to prevent `out-of-bounds` segmentation faults when resizing or scrolling.

## 🏗️ Architecture

The project is structured in three main layers to ensure maximum decoupling:

1. **The Ruby Layer (`lib/scarpe_tui.rb`):** The frontend DSL that developers use. It builds an internal representation of the UI and holds the callback blocks.
2. **The FFI Bridge:** A standard C-ABI layer. This acts as a universal protocol, paving the way for a generic Low-Level TUI library consumable by any language.
3. **The Rust Core (`rust_core/`):** A stateful, thread-safe engine that manages the terminal's raw mode, calculates layouts, and polls non-blocking input events.

## 🚀 Getting Started

### Prerequisites
* **Ruby** (v3.0+)
* **Rust & Cargo** (Latest stable)
* **Bundler** (`gem install bundler`)

### Installation & Running the Showcase

1. **Clone the repository:**

   git clone [https://github.com/TureCatedu/scarpe-tui-poc.git](https://github.com/TureCatedu/scarpe-tui-poc.git)
   cd scarpe-tui-poc

2. **Build the Rust Core engine::**
    cd rust_core
    cargo build
    cd ..

3. **Install Ruby dependencies (FFI):**

    bundle install

4. **Run the Interactive Showcase:**

    bundle exec ruby examples/app.rb
