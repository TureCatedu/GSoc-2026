# API Reference

This document describes the Ruby DSL and the Rust FFI functions exposed to Ruby.

## Ruby DSL

The Ruby DSL is defined in `lib/scarpe_tui.rb` under the `Scarpe` module.

### Entry Point

```ruby
Scarpe.app(use_alternate = false, title: "Scarpe App", &block)
```

Creates an `App` instance and runs the main loop until `quit` is called. If `use_alternate` is `true`, the terminal's alternate screen buffer is used (to preserve the main screen). The block is evaluated in the context of the `App` instance.

### App Instance Methods

During DSL block evaluation, the following methods are available:

#### Containers

| Method | Description |
|--------|-------------|
| `stack(&block)` | Vertical stack. Children arranged top to bottom. |
| `flow(&block)`  | Horizontal flow with wrapping. |
| `border(stroke:, fill:, modifier:, &block)` | Container with box-drawing border; adds 1-cell padding. |
| `dock_bottom(&block)` | Container fixed to the bottom of the screen. |
| `scroll_area(max_height:, stroke:, fill:, modifier:, &block)` | Scrollable container with optional max height. |
| `append_to(parent_id, &block)` | Evaluates a block with the node stack set to `parent_id`, allowing dynamic addition to existing nodes. |

#### Leaf Nodes

| Method | Description | Returns |
|--------|-------------|---------|
| `para(text, stroke:, fill:, modifier:)` | Static text. | `TextNode` |
| `edit_line(initial_text, stroke:, fill:, modifier:, &block)` | Single-line input. Optional block called on Enter. | `EditLine` |
| `edit_box(initial_text, stroke:, fill:, modifier:, &block)` | Multi-line input. Optional block called on Enter (without Shift). | `EditBox` |
| `button(text, stroke:, fill:, modifier:, &block)` | Clickable button. Optional block called on click. | nil |
| `checkbox([text], stroke:, fill:, modifier:, &block)` | Checkbox. Optional block called on toggle. | `Checkbox` |

#### Other Application Methods

| Method | Description |
|--------|-------------|
| `quit` | Signals the main loop to exit. |
| `get_node_text(node_id)` | Returns the text of an `EditLine` or `EditBox` node. |
| `update_node_text(node_id, new_text)` | Updates the text of a node. |
| `get_checkbox_state(node_id)` | Returns `true`/`false` for a checkbox. |

### Nested Helper Classes

| Class       | Methods                       | Description                          |
|-------------|-------------------------------|--------------------------------------|
| `EditLine`  | `#text`, `#text=`             | Gets/sets the text of the edit line. |
| `EditBox`   | `#text`, `#text=`             | Gets/sets the text of the edit box.  |
| `Checkbox`  | `#checked?`                   | Returns whether the checkbox is checked. |
| `TextNode`  | `#text=`                      | Updates the text of a text node.     |

### Constants

**Colors** (`COLORS` hash, string to ANSI code):

```ruby
{
  "black" => 0, "red" => 9, "green" => 10, "yellow" => 11,
  "blue" => 12, "magenta" => 13, "cyan" => 14, "white" => 15,
  "gray" => 8, "dark_red" => 1, "dark_green" => 2, "dark_yellow" => 3,
  "dark_blue" => 4, "dark_magenta" => 5, "dark_cyan" => 6, "light_gray" => 7
}
```

**Modifiers** (`MODIFIERS` hash, string to code):

```ruby
{
  "bold" => 1, "underlined" => 2, "italic" => 3, "reverse" => 4
}
```

**Node Types** (`NODE_TYPES`):

```ruby
{
  root: 0, stack: 1, flow: 2, text: 3, edit_line: 4, button: 5,
  checkbox: 6, border: 7, edit_box: 8, dock_bottom: 9, scroll_area: 10
}
```

## Rust FFI Bindings

The `ScarpeTuiBackend` module in `ext/mylib.rb` declares the following functions:

```ruby
attach_function :scarpe_tui_init, [:bool], :pointer
attach_function :scarpe_tui_free_context, [:pointer], :void
attach_function :scarpe_tui_render, [:pointer], :int
attach_function :scarpe_tui_create_node, [:pointer, :int, :string], :int
attach_function :scarpe_tui_append_child, [:pointer, :int, :int], :int
attach_function :scarpe_tui_poll_events, [:pointer], :int
attach_function :scarpe_tui_get_text, [:pointer, :int], :pointer
attach_function :scarpe_tui_free_string, [:uint64], :void
attach_function :scarpe_tui_get_clicked_button, [:pointer], :int
attach_function :scarpe_tui_get_checkbox_state, [:pointer, :int], :int
attach_function :scarpe_tui_set_style, [:pointer, :int, :int, :int, :int], :int
attach_function :scarpe_tui_update_text, [:pointer, :int, :string], :int
```

### Function Descriptions

| Function | Parameters | Return | Description |
|----------|------------|--------|-------------|
| `scarpe_tui_init` | `use_alternate: bool` | pointer | Initializes the TUI context, returns pointer to `ScarpeTuiContext` or null on failure. |
| `scarpe_tui_free_context` | `ctx_ptr: pointer` | void | Frees the context and restores terminal state. |
| `scarpe_tui_render` | `ctx_ptr` | int status | Renders the current virtual DOM to the terminal using double buffering. |
| `scarpe_tui_create_node` | `ctx_ptr`, `node_type_code: int`, `text_ptr: string/null` | int id or error | Creates a new node in the virtual DOM. Returns node ID or status error. |
| `scarpe_tui_append_child` | `ctx_ptr`, `parent_id: int`, `child_id: int` | int status | Links a child node to a parent. |
| `scarpe_tui_poll_events` | `ctx_ptr` | int code | Processes pending terminal events. Returns `0` (normal), `1` (quit), `2` (submit/click). |
| `scarpe_tui_get_text` | `ctx_ptr`, `node_id: int` | pointer to C string | Returns a heap-allocated C string containing the node text. Caller must free with `scarpe_tui_free_string`. |
| `scarpe_tui_free_string` | `str_ptr: uint64` | void | Frees a C string allocated by the Rust core. |
| `scarpe_tui_get_clicked_button` | `ctx_ptr` | int id | Returns the ID of the last clicked button or `-1`. |
| `scarpe_tui_get_checkbox_state` | `ctx_ptr`, `node_id` | int | Returns `1` if checked, `0` if unchecked, `-1` if invalid. |
| `scarpe_tui_set_style` | `ctx_ptr`, `node_id`, `fg: int`, `bg: int`, `modifier: int` | int status | Sets foreground color, background color, and modifier for a node. |
| `scarpe_tui_update_text` | `ctx_ptr`, `node_id`, `new_text: string` | int status | Updates the text of a `Text`, `EditLine`, `EditBox`, or `Button` node. |

### Status Codes

| Code | Constant | Description |
|------|----------|-------------|
| 0    | `STATUS_OK` | Success |
| -1   | `STATUS_ERR_NULL_PTR` | Null pointer argument |
| -2   | `STATUS_ERR_PANIC` | Rust panicked |
| -3   | `STATUS_ERR_IO` | Terminal I/O error |
| -4   | `STATUS_ERR_INVALID_ID` | Invalid node ID or operation |

## Layout Notes

- The root node fills the terminal width and grows vertically.
- `DockBottom` repositions its children at the bottom of the terminal during layout, independent of their position in the tree.
- `ScrollArea` clips its children and supports vertical scrolling. The `max_height=0` special value means it should fill the remaining vertical space between its current position and the bottom (minus dock height).
- Text wrapping is based on character count; wide characters may not be perfectly handled.

## Event Handling Flow

1. The Rust event thread continuously reads `crossterm` events and sends them over an MPSC channel.
2. `scarpe_tui_poll_events` drains the channel and processes events.
3. Keyboard input updates the focused node (if any) and may set a submission flag or quit flag.
4. Mouse clicks are translated to focus changes, checkbox toggles, or button clicks.
5. The function returns a code indicating whether the Ruby layer should handle a click/submit.
6. The Ruby `run_loop` calls `poll_events`, then `render`, and repeats until quit.