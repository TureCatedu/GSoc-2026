# Scarpe AI · demo

> A terminal-based AI assistant with a retro TUI. This README shows a typical session.

## 1. Launch

```bash
scarpe
```

You see the main screen with a scrollable chat area, a theme-coloured header, and a docked input panel at the bottom.

```
┌────────────────────────────────────────────┐
│  SCARPE AI                                 │
│  Use /read <path> to inject files. /setup …│
│                                            │
│   USER   /read src/main.rb                 │
│          Command: Read src/main.rb         │
│                                            │
│   AI     I see you’ve shared main.rb.      │
│          What would you like to do?        │
│                                            │
│────────────────────────────────────────────│
│                                            │
│ > add error handling________ [Send] [Exit] │
└────────────────────────────────────────────┘
```

## 2. Chat with the AI

Type a message in the edit box and press **Enter** (or click **Send**). The AI streams its reply token by token.

```text
You: Write a Ruby function that returns the square of a number.
AI:  Sure! Here’s a simple Ruby function:
     def square(n)
       n * n
     end
```

## 3. Inject files with `/read`

You can give the AI context by reading a file or a whole directory:

```text
/read lib/scarpe_tui.rb
```

The file content is sent to the AI invisibly. It replies as if it has just studied the code.

## 4. Let the AI write files

Ask the AI to create a new file:

```text
Create a new Ruby file hello.rb that prints "Hello, world!"
```

If consent is required, Scarpe will show a confirmation panel:

```text
┌────────────────────────────────────────────┐
│ The AI wants to create/modify: hello.rb    │
│ Waiting for confirmation...                │
│  [Allow]  [Reject]                         │
└────────────────────────────────────────────┘
```

After you click **Allow**, the file is written to disk.

## 5. Let the AI run shell commands

```text
Install the sinatra gem
```

The AI proposes to execute `gem install sinatra`. Again, you confirm or reject.

```text
┌────────────────────────────────────────────┐
│ The AI wants to execute:                   │
│ gem install sinatra                        │
│  [Allow]  [Reject]                         │
└────────────────────────────────────────────┘
```

The output appears in the chat once the command finishes.

## 6. Keyboard shortcuts

| Key | Action |
|-----|--------|
| `Esc` / `Ctrl+C` | Quit |
| `Tab` | Cycle input focus |
| `Up` / `Down` | Edit text when an editor is focused; otherwise scroll a scrollable view |
| `Page Up` | Scroll toward the beginning of the current scrollable view |
| `Page Down` | Scroll toward the end of the current scrollable view |
| `Shift+Enter` | New line inside the edit box |

## 7. Setup and confirmation preferences

The setup screen uses native checkboxes instead of text-based confirmation fields:

- **Require File Write Confirmation** controls whether file changes require approval.
- **Require Bash Exec Confirmation** controls whether shell commands require approval.

Both preferences are saved as booleans in `~/.scarpe_ai_config.json`. Existing configuration files remain compatible; missing values default to enabled. The provider prompt is visible when the setup screen opens.

Scrollable views support mouse-wheel navigation, `Page Up`/`Page Down`, and a visible vertical scrollbar when content exceeds the viewport. The Ruby callbacks `scroll_to_start` and `scroll_to_end` are also available for buttons and other actions.

---

Scarpe AI combines the power of multiple LLM providers with direct file-system access, all from a comfortable terminal UI.

## License and third-party components

Scarpe TUI is distributed under the MIT License; see [LICENSE](LICENSE).

The Ruby-to-Rust native interface uses the Ruby `ffi` gem. FFI is not part of
this repository and is required at runtime.