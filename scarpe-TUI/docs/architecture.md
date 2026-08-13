# Scarpe TUI Architecture

This document explains the overall design and data flow of the Scarpe TUI project.

## Overview

Scarpe TUI is a terminal-based UI toolkit with a Ruby DSL and a Rust rendering backend. The main components are:

- **Ruby DSL** (`lib/scarpe_tui.rb`): provides a high-level API for building UI (stacks, flows, borders, edit boxes, buttons, etc.).
- **FFI bindings** (`ext/mylib.rb`): connects the Ruby DSL to the Rust core via Ruby's `ffi` gem.
- **Rust core** (`rust_core/`): handles terminal rendering, layout, input processing, and the virtual DOM using the `crossterm` crate.
- **CLI bundle** (`build_cli.rb`): packs the Ruby source and the compiled Rust library into a single executable named `scarpe`.

The project supports both development (running Ruby files directly) and a bundled distribution (a single executable that extracts the Rust library on first run).

## High-Level Data Flow

1. **Application starts** (`examples/app.rb` or the bundled executable).
2. Ruby code calls `Scarpe.app` with a DSL block.
3. The DSL block creates a virtual DOM of nodes (via `stack`, `flow`, `button`, `edit_line`, etc.).
4. Each DSL method calls an FFI function in `ScarpeTuiBackend` to create or modify nodes in the Rust core.
5. The Rust core stores nodes in a `Slab<Node>`, computes layouts, renders to a terminal buffer, and polls for keyboard/mouse events.
6. User input (key presses, mouse clicks, scrolling) is handled by the Rust core and translated into events that the Ruby layer processes.
7. Ruby callbacks (e.g., button clicks, edit line submissions) are triggered after the Rust event loop signals a click/submit.

## Component Details

### 1. Ruby DSL (`lib/scarpe_tui.rb`)

- Defines the `Scarpe` module and `App` class.
- The `App` class owns the UI state, manages the node stack (for building the hierarchy), and stores callbacks.
- DSL methods (`stack`, `flow`, `border`, `para`, `edit_line`, `edit_box`, `button`, `checkbox`, `dock_bottom`, `scroll_area`) create nodes in the Rust core and link them to the current parent.
- The `run_loop` method repeatedly polls the Rust core for events, dispatches button clicks/submissions, and triggers rendering.

#### Node stack

When the DSL block is evaluated, the `App` keeps a stack of node IDs. Each container method pushes its own ID onto the stack, evaluates the nested block, and then pops. This ensures that newly created nodes are attached to the correct parent.

#### Callbacks

Callbacks are stored in `@callbacks` hash, keyed by the node ID. When a button is clicked (or an `edit_line`/`edit_box` submits via Enter), the Rust core returns the node ID, and the Ruby `handle_click!` method finds and executes the associated block.

### 2. FFI Bindings (`ext/mylib.rb`)

- The `ScarpeTuiBackend` module extends `FFI::Library`.
- It loads the shared library (`librust_core.dylib`) and declares function signatures with `attach_function`.
- All functions use C ABI (`extern "C"`) and operate on a pointer to the Rust context (`ScarpeTuiContext`).

### 3. Rust Core (`rust_core/`)

The Rust core is a `cdylib` crate that exposes C-compatible functions. Its main modules are:

- **`lib.rs`**: defines the node types, layout struct, style struct, the `ScarpeTuiContext`, and the double-buffered terminal `Buffer`.
- **`context.rs`**: implements terminal initialization, layout computation, rendering, mouse handling, and drawing routines.
- **`ffi.rs`**: contains all `#[no_mangle] pub extern "C"` functions that Ruby calls through FFI.

#### Virtual DOM

Nodes are stored in a `Slab<Node>`. Each `Node` has an `id`, a `NodeType`, a `Vec<NodeId>` of children, layout data, and style information.

`NodeType` variants:

| Rust variant            | Numeric code | Description                              |
|-------------------------|--------------|------------------------------------------|
| `Root`                  | 0            | Root container                           |
| `Stack`                 | 1            | Vertical layout container                |
| `Flow`                  | 2            | Horizontal/wrapping layout container     |
| `Text(String)`          | 3            | Static text                              |
| `EditLine(String)`      | 4            | Single-line editable field               |
| `Button(String)`        | 5            | Clickable button                         |
| `Checkbox(bool)`        | 6            | Checkbox with boolean state              |
| `Border`                | 7            | Container with a box-drawing border      |
| `EditBox(String)`       | 8            | Multi-line editable area                 |
| `DockBottom`            | 9            | Container pinned to the bottom of screen |
| `ScrollArea{...}`       | 10           | Scrollable container with max height     |

#### Layout system

Layout computation is recursive. The root is laid out at position `(0,0)` with the terminal width. Child containers are laid out according to their type:

- `Stack`: children placed vertically, each below the previous.
- `Flow`: children placed horizontally, wrapping to a new row when needed.
- `Border`: adds 1 cell of padding on all sides.
- `DockBottom`: positions children at the bottom of the terminal, regardless of their place in the tree.
- `ScrollArea`: computes content height, clamps scroll offset, and provides a clipping rectangle for children.
- `Text`, `Button`, `EditLine`, `EditBox`: compute dimensions based on text length and wrapping.

#### Rendering

Rendering uses a double buffer (`current_buffer` and `next_buffer`). Each frame:

1. Layouts are recomputed if needed.
2. The next buffer is cleared.
3. All nodes are drawn recursively into the next buffer, respecting clipping regions for scroll areas.
4. The two buffers are compared cell-by-cell; only changed cells are written to the terminal.
5. Buffers are swapped.

#### Input handling

Crossterm events are read in a separate thread and sent over an `mpsc` channel. The `scarpe_tui_poll_events` function drains the channel and processes:

- Keyboard: navigation (arrows, page up/down, tab), text input, Enter submissions, Esc/Ctrl+C to quit.
- Mouse: click on buttons/checkboxes/edit fields, scroll wheel for scroll areas.
- Resize: marks redraw.

The function returns:
- `0` for normal
- `1` for quit
- `2` for a submission/button click

### 4. The Bundled Executable (`build_cli.rb`)

- Reads the release build of the Rust library (`rust_core/target/release/librust_core.dylib`).
- Encodes the library as Base64 and embeds it in the generated Ruby script.
- Strips `require_relative` statements and replaces library path logic so the final script is self-contained.
- The generated `scarpe` executable is placed in the project root.
- When run, it extracts the embedded library to `~/.scarpe_ai/librust_core.dylib` (using size checks to avoid re-extracting) and then runs the application.

## Error Handling

The Rust core returns status codes:

| Code | Constant                | Meaning                              |
|------|-------------------------|--------------------------------------|
| 0    | `STATUS_OK`             | Success                              |
| -1   | `STATUS_ERR_NULL_PTR`   | Null pointer passed to Rust function |
| -2   | `STATUS_ERR_PANIC`      | Rust code panicked                   |
| -3   | `STATUS_ERR_IO`         | Terminal I/O error                   |
| -4   | `STATUS_ERR_INVALID_ID` | Invalid node ID or node type         |

The Ruby `App#handle_rust_status!` translates these into appropriate Ruby exceptions unless the code is non-negative.

## Configuration and Persistence

- Configuration is stored in `~/.scarpe_ai_config.json` and read at startup.
- Chat history is stored in `~/.scarpe_ai_history.json`.
- The AI app uses these files to maintain state across runs.
- The setup UI is shown if configuration is missing or `--setup` is passed.