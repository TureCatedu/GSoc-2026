# Scarpe AI

![Ruby](https://img.shields.io/badge/Ruby-CC342D?style=for-the-badge&logo=ruby&logoColor=white)
![Rust](https://img.shields.io/badge/Rust-000000?style=for-the-badge&logo=rust&logoColor=white)
![GSoC 2026](https://img.shields.io/badge/GSoC-2026-FABB19?style=for-the-badge)

A terminal-based AI assistant with a retro text user interface, built with **Ruby** and **Rust**.

Integrates multiple LLM providers (OpenAI, Anthropic, Gemini, OpenRouter) into a single CLI with a responsive TUI. It can read files, create or modify files, and execute shell commands — with an optional consent gate.

---

## ✨ Features

- **Multi-provider AI** – OpenAI, Anthropic (Claude), Google Gemini, OpenRouter
- **Rich TUI** – scrollable chat history with a visible scrollbar, buttons, checkboxes, bordered panels, and a customisable theme
- **Tool use** – AI can read local files, write new files, and run shell commands
- **Consent controls** – optionally require confirmation before file writes or bash execution
- **Streaming** – token-by-token responses in real time
- **Persistent history** – conversation saved to `~/.scarpe_ai_history.json`
- **File injection** – `/read <path>` to feed a file or entire directory to the AI
- **Single portable binary** – build a self-contained executable with `build_cli.rb`

---

## 🚀 Quick start

### Prerequisites

- **Ruby** ≥ 3.0
- **Rust** toolchain (if building from source)

### 1. Clone & build

```bash
git clone https://github.com/yourusername/scarpe_ai.git
cd scarpe_ai

# Build the Rust core
cd rust_core && cargo build --release && cd ..

# Run the setup wizard
ruby examples/app.rb --setup
```

The wizard asks for your provider, API key, model, theme colour, and consent preferences. Consent options use native checkboxes and are persisted as booleans. Settings are saved to `~/.scarpe_ai_config.json`.

### 2. Run

```bash
ruby examples/app.rb
```

### 3. Install globally (optional)

Build the self-contained script and move it into your `PATH`:

```bash
ruby build_cli.rb            # creates ./scarpe
sudo cp scarpe /usr/local/bin/   # or ~/.local/bin if it is in your PATH
```

Now you can launch Scarpe AI from any directory:

```bash
scarpe --setup   # first-run configuration
scarpe            # start chatting
```

---

## ⌨️ In-app commands & shortcuts

| Command | Effect |
|---------|--------|
| `/read <path>` | Inject a file or directory into the conversation |
| `/clear` | Reset the conversation history |
| `/setup` | Re-run the setup wizard |

| Key | Action |
|-----|--------|
| `Esc` / `Ctrl+C` | Quit |
| `Tab` | Switch focus between input fields |
| `↑` `↓` `Page Up` `Page Down` | Scroll the active view |
| `Enter` | Submit (single-line) |
| `Shift+Enter` | New line (in the multi-line edit box) |
| Mouse wheel | Scroll the active scroll area |

---

## 🧱 Architecture

```
┌──────────────────────────────────────────────────┐
│  examples/app.rb        ← AI chat application    │
│  lib/scarpe_tui.rb      ← Shoes‑like TUI         │
│  ext/mylib.rb           ← FFI bindings to Rust   │
│  rust_core/             ← Terminal engine        │
│    ├── lib.rs           ← data structures        │
│    ├── ffi.rs           ← C ABI exports          │
│    └── context.rs       ← layout + rendering     │
│  build_cli.rb           ← single‑executable pack │
└──────────────────────────────────────────────────┘
```

### Rust core (`rust_core/`)

A terminal rendering engine built on [`crossterm`](https://crates.io/crates/crossterm).
It holds a virtual DOM (stack, flow, text, buttons, checkboxes, borders, scroll areas, …), computes layout, clips scroll-area children to their viewport, draws the scrollbar above the clipped region, and **diff‑renders** only changed cells.
All functions are exposed through a **C ABI** (`scarpe_tui_*`) so they can be called from any language via FFI. The scrolling command is exported as `scarpe_tui_scroll_to`, supporting movement to the start or end of the active scrollable view.

### Scarpe TUI (`lib/scarpe_tui.rb`)

A Ruby DSL inspired by [Shoes](https://shoesrb.com/). Build interfaces declaratively:

```ruby
Scarpe.app(true, title: "My App") do
  stack do
    para "Hello, world!", stroke: "white"
    button "Click me" do
      puts "clicked!"
    end
  end
end
```

**Widgets:** `stack`, `flow`, `border`, `scroll_area`, `dock_bottom`, `para`, `button`, `edit_line`, `edit_box`, `checkbox`.

Scrollable views expose programmatic controls for callbacks and actions:

```ruby
scroll_to_start
scroll_to_end
```

These commands move the active scroll area to the beginning or end. Scroll areas clamp their offset to the available content and render a vertical track and thumb when the content exceeds the viewport.

### CLI builder (`build_cli.rb`)

Generates a single Ruby script (`scarpe`) that:

1. Base64‑encodes the compiled `librust_core.dylib`
2. Inlines all Ruby sources
3. Extracts the dylib into `~/.scarpe_ai/` on first run

Just run `ruby build_cli.rb` and distribute the resulting `scarpe` file.

---

## ⚙️ Configuration

All settings live in `~/.scarpe_ai_config.json`:

```json
{
  "provider": "openai",
  "api_key": "sk-...",
  "model": "gpt-4o",
  "theme_color": "cyan",
  "require_file_consent": true,
  "require_bash_consent": true
}
```

### Supported providers

| Provider | Example model | API key source |
|----------|---------------|----------------|
| `openai` | `gpt-4o` | OpenAI |
| `anthropic` | `claude-sonnet-4-20250514` | Anthropic |
| `gemini` | `gemini-2.5-flash` | Google AI Studio |
| `openrouter` | `openai/gpt-4o` | OpenRouter |

---

## 🐞 Error handling

Crashes are logged to `~/.scarpe_ai/error.log`.
The Ruby layer raises typed exceptions for FFI failures:

- `RustPanicError`
- `RustIOError`
- `RustNullPointerError`
- `RustInvalidIdError`

---

## 📁 Project structure

```
.
├── build_cli.rb              # Standalone executable packer
├── examples/
│   └── app.rb                # Main AI chat application
├── ext/
│   └── mylib.rb              # FFI bindings to Rust
├── header/
│   └── scarpe_tui_header.c   # C header for Rust lib
├── lib/
│   └── scarpe_tui.rb         # Declarative TUI framework
├── rust_core/
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs
│       ├── ffi.rs
│       └── context.rs
├── Gemfile
├── Gemfile.lock
└── README.md
```

---

## 📜 License

This work is licensed under <a href="https://creativecommons.org/licenses/by-nc-nd/4.0/">CC BY-NC-ND 4.0</a><img src="https://mirrors.creativecommons.org/presskit/icons/cc.svg" alt="" style="max-width: 1em;max-height:1em;margin-left: .2em;"><img src="https://mirrors.creativecommons.org/presskit/icons/by.svg" alt="" style="max-width: 1em;max-height:1em;margin-left: .2em;"><img src="https://mirrors.creativecommons.org/presskit/icons/nc.svg" alt="" style="max-width: 1em;max-height:1em;margin-left: .2em;"><img src="https://mirrors.creativecommons.org/presskit/icons/nd.svg" alt="" style="max-width: 1em;max-height:1em;margin-left: .2em;">

You are free to share the work as long as you give appropriate credit, do not use it for commercial purposes, and do not make modifications.

Full license: https://creativecommons.org/licenses/by-nc-nd/4.0/
