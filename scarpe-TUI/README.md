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

```
/read lib/scarpe_tui.rb
```

The file content is sent to the AI invisibly. It replies as if it has just studied the code.

## 4. Let the AI write files

Ask the AI to create a new file:

```
Create a new Ruby file hello.rb that prints "Hello, world!"
```

If consent is required, Scarpe will show a confirmation panel:

```
┌────────────────────────────────────────────┐
│ The AI wants to create/modify: hello.rb    │
│ Waiting for confirmation...                │
│  [Allow]  [Reject]                         │
└────────────────────────────────────────────┘
```

After you click **Allow**, the file is written to disk.

## 5. Let the AI run shell commands

```
Install the sinatra gem
```

The AI proposes to execute `gem install sinatra`. Again, you confirm or reject.

```
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
| `↑` `↓` `PgUp` `PgDn` | Scroll the conversation |
| `Shift+Enter` | New line inside the edit box |

---

Scarpe AI combines the power of multiple LLM providers with direct file‑system access, all from a comfortable terminal UI.
