## Completed

- Rust backend connected to the Ruby DSL through FFI.
- Two-level terminal buffer and incremental rendering.
- Recursive layout for root, stack, flow, border, dock bottom, and scroll areas.
- Keyboard and mouse input.
- Unicode editing for `EditLine` and `EditBox`.
- Cursor movement between lines while preserving the vertical column.
- Newline insertion in `EditBox` with `Shift+Enter`.
- Fallback support for terminals that report the combination as `Alt+Enter` or `Ctrl+Enter`.
- Submission with normal `Enter`.
- Native checkboxes for consent settings on the setup screen.
- Consent preferences persisted as boolean values.
- Scrolling with `Up`/`Down` in scrollable views when no editor is focused.
- Scrolling with `Page Up` and `Page Down`.
- Ruby buttons and `scroll_to_start` / `scroll_to_end` APIs.
- Vertical scrollbar with track and thumb in `ScrollArea`.
- Initial setup with the provider prompt visible.
- Rust tests for the editor and buffer.
- Debug and release builds verified.
- Ruby syntax and diff checks verified.

## Current Phase

Consolidating the distribution workflow:

1. Build the Rust backend in release mode.
2. Regenerate the bundled executable.
3. Verify that the bundle contains the updated release library.
4. Keep build artifacts separate from source changes.

## Scrolling Status

Scrolling for `ScrollArea` is complete for the current workflow:

- `Up`/`Down` and the mouse wheel change the view offset.
- `Page Up` and `Page Down` move the view to the beginning and end, respectively.
- `scroll_to_start` and `scroll_to_end` expose the same controls to Ruby callbacks.
- Rendering draws a track and thumb when the content exceeds the viewport.

Potential UX improvements remain, such as clearer focus indicators and additional tests for edge cases involving terminal resizing and nested content.

## Next Steps

### Phase 1 — Distribution

- Regenerate the release bundle.
- Verify FFI loading from both source and the bundled executable.
- Add an automated non-interactive bundle verification step.

### Phase 2 — API Robustness

- Add Ruby tests for FFI error-code validation.
- Verify dynamic text updates and submit callbacks.
- Review shutdown and terminal restoration behavior.

### Phase 3 — TUI UX

- Improve scrollbar and focus visual feedback.
- Improve focus handling and visual feedback.
- Add tests for terminal resizing, scrollbars, and mouse scrolling.

### Phase 4 — Quality and Distribution

- Document macOS and Linux compatibility.
- Automate testing and builds.
- Minimize unrelated changes before committing.