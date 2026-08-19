# Scarpe AI User Guide

Scarpe AI is a terminal-based AI assistant with a retro TUI. This guide covers setup, usage, commands, and keyboard shortcuts.

## Starting Scarpe AI

After building the Rust core and bundling (see `building.md`), launch the program:

```bash
./scarpe
```

Or if running from source:

```bash
ruby examples/app.rb
```

On first launch, a setup screen appears. Fill in:

- **Provider**: `openai`, `anthropic`, `gemini`, or `openrouter`
- **API Key**: your provider API key
- **Model**: model name (e.g., `gpt-4o`)
- **Theme Color**: `cyan`, `green`, `magenta`, `yellow`, or `white`
- **Require File Write Confirmation**: toggle the checkbox to ask before the AI creates or modifies files
- **Require Bash Exec Confirmation**: toggle the checkbox to ask before the AI executes shell commands

The setup screen uses native checkboxes for the two confirmation options. Click a checkbox to toggle it, then click **SAVE**. Settings are saved to `~/.scarpe_ai_config.json` as boolean values (`true` or `false`).

The provider prompt is visible when the setup screen opens. Use the setup screen's scroll behavior if the terminal is too small to display all fields.

## Main Screen

The main UI consists of:

- A header with the title and usage hints.
- A scrollable chat history area showing USER and AI messages.
- A bottom input panel with a multi-line text box, **Send**, and **Exit** buttons.

## Chatting with the AI

1. Type your message in the edit box.
2. Press **Enter** (or click **Send**) to send.
3. The AI streams its response token by token.

Use **Shift+Enter** to insert a newline in the edit box. While an editor is focused, `Up` and `Down` retain their normal cursor/navigation behavior.

## Special Commands

- `/read <path>` – read a file or directory and include it in the conversation.
  - For a file: the content is added invisibly to the AI context.
  - For a directory: all readable files (up to 150KB each) are included recursively, excluding common build/ignored directories (`.git`, `node_modules`, `target`, etc.).
- `/setup` or `/config` – reopen the configuration screen.
- `/clear` – clear the chat history and restart the conversation.

## Letting the AI Write Files

Ask the AI to create or modify a file. If confirmation is required, a panel appears showing the file path and **Allow** / **Reject** buttons.

- Click **Allow** to write the file (creating parent directories if needed).
- Click **Reject** to cancel.

If confirmation is disabled, the file is written automatically and a confirmation message appears.

## Letting the AI Run Shell Commands

Ask the AI to execute a shell command (e.g., "Install the sinatra gem"). If confirmation is required, a panel shows the command with **Allow** / **Reject**.

- Click **Allow** to run the command. The output (stdout or stderr, truncated to 1000 characters) is displayed in the chat.
- Click **Reject** to cancel.

If confirmation is disabled, the command runs automatically and the output appears.

## Keyboard Shortcuts

| Key | Action |
|-----|--------|
| `Esc` / `Ctrl+C` | Quit |
| `Tab` | Cycle focus between input fields |
| `Up` / `Down` | Move through text when an editor is focused; otherwise scroll the current scrollable view |
| `Page Up` | Scroll toward the beginning of the current scrollable view |
| `Page Down` | Scroll toward the end of the current scrollable view |
| `Shift+Enter` | New line in the edit box |

## Mouse Support

- Click **buttons** to activate them.
- Click **edit fields** to focus.
- Click **checkboxes** to toggle.
- Use the **scroll wheel** over the chat area to scroll.

## Theme Colors

The theme color is used for user message borders, header background, send button, and input border. Available colors:

- `cyan`
- `green`
- `magenta`
- `yellow`
- `white`

## Files

| File | Purpose |
|------|---------|
| `~/.scarpe_ai_config.json` | Provider, API key, model, preferences |
| `~/.scarpe_ai_history.json` | Full chat message history (including system prompt) |
| `~/.scarpe_ai/error.log` | Fatal error log for the bundled executable |
| `~/.scarpe_ai/librust_core.dylib` | Extracted Rust library for the bundled executable |

## Tips

- The AI is instructed to use `<write_file>` and `<execute_bash>` XML tags for file and command operations. These tags are parsed and acted upon by the application.
- The chat history is persisted between sessions and loaded at startup.
- Only the last 20 messages are sent to the AI provider (plus the system prompt) to manage context length.
- The system prompt informs the AI that it is a software engineer with file system access.