# Building and Running Scarpe TUI

This document explains how to compile, run, and bundle the Scarpe TUI project.

## Prerequisites

- **Ruby** 3.0+ (with bundler if installing gems)
- **Rust** (stable toolchain) and Cargo
- **`ffi` gem** (for running from source) – the bundled executable includes the `ffi` gem requirement, so end users need ffi installed or Ruby can load it.
- macOS (the current FFI bindings target `librust_core.dylib`; Linux support exists but requires building for the appropriate `.so` file)

## Building the Rust Core

### Debug build (for development)

```bash
cd rust_core
cargo build
```

This creates `rust_core/target/debug/librust_core.dylib`, which is the path expected by `ext/mylib.rb` when running Ruby files directly.

### Release build (for bundling/performance)

```bash
cd rust_core
cargo build --release
```

This creates `rust_core/target/release/librust_core.dylib`, used by `build_cli.rb`.

## Installing Ruby Dependencies

From the project root:

```bash
bundle install
```

Or manually:

```bash
gem install ffi -v '~> 1.15.5'
```

## Running the AI Application Directly (Development)

Make sure a debug or release Rust library exists (debug preferred because `ext/mylib.rb` points to `target/debug`). Then:

```bash
ruby examples/app.rb
```

Optionally pass `--setup` to force the configuration screen:

```bash
ruby examples/app.rb --setup
```

## Bundling the Self-Contained Executable

Once the release library is built, run:

```bash
ruby build_cli.rb
```

This generates an executable named `scarpe` in the project root. The script:

1. Reads `rust_core/target/release/librust_core.dylib`.
2. Base64-encodes it and embeds it into a Ruby script template.
3. Strips development-only requires and adjusts library loading to use the embedded binary.
4. Sets the executable bit.

To run the bundled program:

```bash
./scarpe
```

On first run, it extracts the embedded library to `~/.scarpe_ai/librust_core.dylib` (using a size check). Subsequent runs skip extraction if the file matches.

> **Note:** The bundled executable still requires Ruby and the `ffi`, `net/http`, `json`, `fileutils`, and `base64` gems/standard libraries. It does not require the local `rust_core/target/` directory.

## Configuration

The AI application stores its configuration in `~/.scarpe_ai_config.json`. If the file is missing or incomplete, the setup UI is displayed automatically. You can also force setup with `--setup`.

Configuration fields:

| Field                  | Type    | Description                                             |
|------------------------|---------|---------------------------------------------------------|
| `provider`             | string  | One of `openai`, `anthropic`, `gemini`, `openrouter`    |
| `api_key`              | string  | API key for the chosen provider                         |
| `model`                | string  | Model name (e.g., `gpt-4o`, `claude-3-5-sonnet-20241022`) |
| `theme_color`          | string  | UI accent color (`cyan`, `green`, `magenta`, `yellow`, `white`) |
| `require_file_consent` | boolean | If `true`, ask before writing files created by the AI   |
| `require_bash_consent` | boolean | If `true`, ask before executing shell commands from the AI |

## Troubleshooting

- **`Error: Compile Rust project first...`** – Run `cd rust_core && cargo build --release` before `build_cli.rb`.
- **`FFI::NotFoundError` or `LoadError`** – The Rust library file could not be found. Ensure you built the correct target (debug for development, release for bundling) and that the library path in `ext/mylib.rb` matches your machine.
- **Terminal not restored after crash** – The Rust core normally restores terminal state during `shutdown`. If the program is killed abnormally, run `reset` or `stty sane` to restore the terminal.
- **Bundled executable fails to load ffi** – Install the `ffi` gem (`gem install ffi`).