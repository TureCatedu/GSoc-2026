use crate::*;
use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_int};
use std::panic::catch_unwind;
use std::ptr::{self, null_mut};
use crossterm::event::{Event, KeyCode, KeyEventKind, KeyModifiers, MouseButton, MouseEventKind};
use crossterm::style::Attribute::{Bold, Italic, Reset, Reverse, Underlined};
use crossterm::style::Color::AnsiValue;
use crate::{STATUS_ERR_NULL_PTR, STATUS_ERR_PANIC, STATUS_OK, STATUS_ERR_INVALID_ID};

/// Creates a new node in the virtual DOM.
/// The node type and optional text content are specified by the caller.
/// Returns the ID of the newly created node or an error status.
#[no_mangle]
pub extern "C" fn scarpe_tui_create_node(
    ctx_ptr: *mut ScarpeTuiContext,
    node_type_code: c_int,
    text_ptr: *const c_char, 
) -> c_int {
    if ctx_ptr.is_null() { return STATUS_ERR_NULL_PTR; }

    let result = catch_unwind(|| {
        let ctx = unsafe { &mut *ctx_ptr };
        let vacant_entry = ctx.nodes.vacant_entry();
        let new_id = vacant_entry.key();

        let node_type = match node_type_code {
            0 => NodeType::Root,
            1 => NodeType::Stack,
            2 => NodeType::Flow,
            3 => {
                let text_content = if !text_ptr.is_null() {
                    unsafe { CStr::from_ptr(text_ptr) }.to_string_lossy().into_owned()
                } else { String::new() };
                NodeType::Text(text_content)
            }
            4 => {
                let text_content = if !text_ptr.is_null() {
                    unsafe { CStr::from_ptr(text_ptr) }.to_string_lossy().into_owned()
                } else { String::new() };
                
                // Automatically assigns focus to the first EditLine created.
                if ctx.focused_node.is_none() {
                    ctx.focused_node = Some(new_id);
                }
                
                NodeType::EditLine(text_content)
            }
            5 => {
                let text_content = if !text_ptr.is_null() {
                    unsafe { CStr::from_ptr(text_ptr) }.to_string_lossy().into_owned()
                } else { String::new() };
                NodeType::Button(text_content)
            }
            6 => NodeType::Checkbox(false),
            7 => NodeType::Border,
            8 => {
                let text_content = if !text_ptr.is_null() {
                    unsafe { CStr::from_ptr(text_ptr) }.to_string_lossy().into_owned()
                } else { String::new() };
                
                // Automatically assigns focus if the first created node is an EditBox.
                if ctx.focused_node.is_none() {
                    ctx.focused_node = Some(new_id);
                }
                
                NodeType::EditBox(text_content)
            }
            9 => NodeType::DockBottom,
            10 => {
                let limit = if !text_ptr.is_null() {
                    unsafe { CStr::from_ptr(text_ptr) }.to_string_lossy().parse::<u16>().unwrap_or(10)
                } else { 10 };
                NodeType::ScrollArea { scroll_offset_y: 0, max_height: limit }
            }
            _ => NodeType::Stack,
        };

        if node_type_code == 0 { ctx.root_id = Some(new_id); }
        
        vacant_entry.insert(Node {
            id: new_id,
            node_type,
            children: Vec::new(),
            layout: ComputedLayout::default(),
            style: NodeStyle::default(), 
        });

        ctx.needs_redraw = true;
        new_id as c_int
    });

    result.unwrap_or(STATUS_ERR_PANIC)
}

/// Initializes the TUI context.
/// Sets up the terminal environment and returns a pointer to the context.
#[no_mangle]
pub extern "C" fn scarpe_tui_init(use_alternate: bool) -> *mut ScarpeTuiContext {
    let result = catch_unwind(|| match ScarpeTuiContext::new(use_alternate) {
        Ok(ctx) => Box::into_raw(Box::new(ctx)),
        Err(_) => ptr::null_mut(),
    });
    result.unwrap_or(ptr::null_mut())
}

/// Renders the current state of the TUI context to the terminal.
/// Returns a status code indicating success or failure.
#[no_mangle]
pub extern "C" fn scarpe_tui_render(ctx_ptr: *mut ScarpeTuiContext) -> c_int {
    if ctx_ptr.is_null() { return STATUS_ERR_NULL_PTR; }
    let result = catch_unwind(|| {
        let ctx = unsafe { &mut *ctx_ptr };
        match ctx.render() {
            Ok(_) => STATUS_OK,
            Err(_) => STATUS_ERR_IO,
        }
    });
    result.unwrap_or(STATUS_ERR_PANIC)
}

/// Frees the resources associated with the TUI context.
/// Ensures that the terminal is restored to its original state.
#[no_mangle]
pub extern "C" fn scarpe_tui_free_context(ctx_ptr: *mut ScarpeTuiContext) {
    if ctx_ptr.is_null() { return; }
    let _ = catch_unwind(|| {
        let mut ctx = unsafe { Box::from_raw(ctx_ptr) };
        let _ = ctx.shutdown();
    });
}

/// Appends a child node to a parent node in the virtual DOM.
/// Returns a status code indicating success or failure.
#[no_mangle]
pub extern "C" fn scarpe_tui_append_child(ctx_ptr: *mut ScarpeTuiContext, parent_id: c_int, child_id: c_int) -> c_int {
    if ctx_ptr.is_null() { return STATUS_ERR_NULL_PTR; }
    if parent_id < 0 || child_id < 0 { return STATUS_ERR_INVALID_ID; }

    let result = catch_unwind(|| {
        let ctx = unsafe { &mut *ctx_ptr };
        let p_id = parent_id as usize;
        let c_id = child_id as usize;

        if !ctx.nodes.contains(p_id) || !ctx.nodes.contains(c_id) {
            return STATUS_ERR_INVALID_ID;
        }

        if let Some(parent) = ctx.nodes.get_mut(p_id) {
            parent.children.push(c_id);
            ctx.needs_redraw = true; 
        }
        STATUS_OK
    });
    result.unwrap_or(STATUS_ERR_PANIC)
}

/// Retrieves the text content of a node (EditLine or EditBox).
/// Returns a pointer to a C-string containing the text.
#[no_mangle]
pub extern "C" fn scarpe_tui_get_text(ctx_ptr: *mut ScarpeTuiContext, node_id: c_int) -> *mut c_char {
    if ctx_ptr.is_null() || node_id < 0 { return null_mut(); }
    let result = catch_unwind(|| {
        let ctx = unsafe { &mut *ctx_ptr };
        if let Some(node) = ctx.nodes.get(node_id as usize) {
            match &node.node_type {
                NodeType::EditLine(ref text) | NodeType::EditBox(ref text) => {
                    if let Ok(c_string) = CString::new(text.clone()) {
                        return c_string.into_raw(); 
                    }
                }
                _ => {}
            }
        }
        null_mut()
    });
    result.unwrap_or(null_mut())
}

/// Frees a C-string that was previously allocated in Rust.
/// Prevents memory leaks by deallocating the string.
#[no_mangle]
pub extern "C" fn scarpe_tui_free_string(s: *mut c_char) {
    if s.is_null() { return; }
    let _ = catch_unwind(|| {
        unsafe { let _ = CString::from_raw(s); }
    });
}

/// Retrieves the ID of the last clicked button, if any.
/// Returns -1 if no button was clicked.
#[no_mangle]
pub extern "C" fn scarpe_tui_get_clicked_button(ctx_ptr: *mut ScarpeTuiContext) -> c_int {
    if ctx_ptr.is_null() { return -1; }
    let result = catch_unwind(|| {
        let ctx = unsafe { &mut *ctx_ptr };
        if let Some(id) = ctx.clicked_button.take() {
            return id as c_int;
        }
        -1
    });
    result.unwrap_or(-1)
}

/// Retrieves the state of a checkbox node (checked or unchecked).
/// Returns 1 for checked, 0 for unchecked, and -1 for invalid nodes.
#[no_mangle]
pub extern "C" fn scarpe_tui_get_checkbox_state(ctx_ptr: *mut ScarpeTuiContext, node_id: c_int) -> c_int {
    if ctx_ptr.is_null() || node_id < 0 { return -1; }
    let result = catch_unwind(|| {
        let ctx = unsafe { &mut *ctx_ptr };
        if let Some(node) = ctx.nodes.get(node_id as usize) {
            if let NodeType::Checkbox(state) = node.node_type {
                return if state { 1 } else { 0 };
            }
        }
        -1
    });
    result.unwrap_or(-1)
}

/// Sets the style (foreground, background, and modifiers) for a node.
/// Updates the node's style and marks the context for redraw.
#[no_mangle]
pub extern "C" fn scarpe_tui_set_style(
    ctx_ptr: *mut ScarpeTuiContext, node_id: c_int, fg: c_int, bg: c_int, modifier: c_int,
) -> c_int {
    if ctx_ptr.is_null() || node_id < 0 { return STATUS_ERR_NULL_PTR; }
    let result = catch_unwind(|| {
        let ctx = unsafe { &mut *ctx_ptr };
        if let Some(node) = ctx.nodes.get_mut(node_id as usize) {
            node.style.fg = if (0..=255).contains(&fg) { AnsiValue(fg as u8) } else { Color::Reset };
            node.style.bg = if (0..=255).contains(&bg) { AnsiValue(bg as u8) } else { Color::Reset };
            node.style.modifier = match modifier {
                1 => Bold,
                2 => Underlined,
                3 => Italic,
                4 => Reverse,
                _ => Reset,
            };
            ctx.needs_redraw = true; 
            return STATUS_OK;
        }
        STATUS_ERR_INVALID_ID
    });
    result.unwrap_or(STATUS_ERR_PANIC)
}

/// Updates the text content of a node in the virtual DOM.
/// The node can be of type Text, EditLine, EditBox, or Button.
/// Returns a status code indicating success or failure.
#[no_mangle]
pub extern "C" fn scarpe_tui_update_text(
    ctx_ptr: *mut ScarpeTuiContext, node_id: c_int, new_text_ptr: *const c_char,
) -> c_int {
    if ctx_ptr.is_null() || new_text_ptr.is_null() || node_id < 0 {
        return STATUS_ERR_NULL_PTR;
    }

    let result = std::panic::catch_unwind(|| {
        let ctx = unsafe { &mut *ctx_ptr };
        let new_text = unsafe { CStr::from_ptr(new_text_ptr) }
            .to_string_lossy()
            .into_owned();

        if let Some(node) = ctx.nodes.get_mut(node_id as usize) {
            match &mut node.node_type {
                NodeType::Text(ref mut text)
                | NodeType::EditLine(ref mut text)
                | NodeType::EditBox(ref mut text)
                | NodeType::Button(ref mut text) => {
                    *text = new_text;
                    ctx.needs_redraw = true; // Mark the context for redraw
                    return STATUS_OK;
                }
                _ => return STATUS_ERR_INVALID_ID, // Invalid node type for text update
            }
        }
        STATUS_ERR_INVALID_ID // Node ID not found
    });

    result.unwrap_or(STATUS_ERR_PANIC) // Handle panics gracefully
}

/// Polls for user input events (keyboard and mouse) and updates the TUI context accordingly.
/// Handles events such as key presses, mouse clicks, and scroll actions.
/// Returns a status code indicating the result of the event handling.
#[no_mangle]
pub extern "C" fn scarpe_tui_poll_events(ctx_ptr: *mut ScarpeTuiContext) -> c_int {
    if ctx_ptr.is_null() {
        return -1;
    }

    let result = catch_unwind(|| {
        let ctx = unsafe { &mut *ctx_ptr };
        let mut quit_requested = false;
        let mut state_changed = false;
        let mut submitted = false;

        let mut pending_events = Vec::new();
        if let Some(ref rx) = ctx.event_receiver {
            while let Ok(event) = rx.try_recv() {
                pending_events.push(event);
            }
        }

        for event in pending_events {
            match event {
                Event::Key(key_event) => {
                    if key_event.kind != KeyEventKind::Press {
                        continue;
                    }

                    if key_event.code == KeyCode::Esc
                        || (key_event.modifiers.contains(KeyModifiers::CONTROL)
                            && key_event.code == KeyCode::Char('c'))
                    {
                        quit_requested = true;
                        break;
                    }

                    match key_event.code {
                        KeyCode::Up => {
                            ctx.scroll_offset_y = 1;
                            for (_, node) in ctx.nodes.iter_mut() {
                                if let NodeType::ScrollArea { ref mut scroll_offset_y, .. } = node.node_type {
                                    *scroll_offset_y = scroll_offset_y.saturating_sub(1);
                                    state_changed = true;
                                }
                            }
                        }
                        KeyCode::Down => {
                            ctx.scroll_offset_y = 1;
                            for (_, node) in ctx.nodes.iter_mut() {
                                if let NodeType::ScrollArea { ref mut scroll_offset_y, .. } = node.node_type {
                                    *scroll_offset_y = scroll_offset_y.saturating_add(1);
                                    state_changed = true;
                                }
                            }
                        }
                        KeyCode::PageUp => {
                            ctx.scroll_offset_y = 1;
                            for (_, node) in ctx.nodes.iter_mut() {
                                if let NodeType::ScrollArea { ref mut scroll_offset_y, .. } = node.node_type {
                                    *scroll_offset_y = scroll_offset_y.saturating_sub(5);
                                    state_changed = true;
                                }
                            }
                        }
                        KeyCode::PageDown => {
                            ctx.scroll_offset_y = 1;
                            for (_, node) in ctx.nodes.iter_mut() {
                                if let NodeType::ScrollArea { ref mut scroll_offset_y, .. } = node.node_type {
                                    *scroll_offset_y = scroll_offset_y.saturating_add(5);
                                    state_changed = true;
                                }
                            }
                        }
                        KeyCode::Tab => {
                            let mut inputs = Vec::new();
                            for (id, node) in ctx.nodes.iter() {
                                if matches!(node.node_type, NodeType::EditLine(_))
                                    || matches!(node.node_type, NodeType::EditBox(_))
                                {
                                    inputs.push(id);
                                }
                            }
                            if !inputs.is_empty() {
                                let current_idx = inputs
                                    .iter()
                                    .position(|&id| Some(id) == ctx.focused_node)
                                    .unwrap_or(0);
                                let next_idx = (current_idx + 1) % inputs.len();
                                ctx.focused_node = Some(inputs[next_idx]);
                                state_changed = true;
                            }
                        }
                        _ => {}
                    }

                    if let Some(focus_id) = ctx.focused_node {
                        if let Some(node) = ctx.nodes.get_mut(focus_id) {
                            if let NodeType::EditLine(ref mut text) = node.node_type {
                                match key_event.code {
                                    KeyCode::Char(c)
                                        if key_event.modifiers.is_empty()
                                            || key_event.modifiers == KeyModifiers::SHIFT =>
                                    {
                                        text.push(c);
                                        state_changed = true;
                                    }
                                    KeyCode::Backspace => {
                                        if text.pop().is_some() {
                                            state_changed = true;
                                        }
                                    }
                                    KeyCode::Enter => {
                                        ctx.clicked_button = Some(focus_id);
                                        ctx.scroll_offset_y = 0;
                                        submitted = true;
                                    }
                                    _ => {}
                                }
                            }
                            if let NodeType::EditBox(ref mut text) = node.node_type {
                                match key_event.code {
                                    KeyCode::Char(c)
                                        if key_event.modifiers.is_empty()
                                            || key_event.modifiers == KeyModifiers::SHIFT =>
                                    {
                                        text.push(c);
                                        state_changed = true;
                                    }
                                    KeyCode::Enter => {
                                        if key_event.modifiers.contains(KeyModifiers::SHIFT) || key_event.modifiers.contains(KeyModifiers::ALT) {
                                            text.push('\n');
                                            state_changed = true;
                                        } else {
                                            ctx.clicked_button = Some(focus_id);
                                            ctx.scroll_offset_y = 0;
                                            submitted = true;
                                        }
                                    }
                                    KeyCode::Backspace => {
                                        if text.pop().is_some() {
                                            state_changed = true;
                                        }
                                    }
                                    _ => {}
                                }
                            }
                        }
                    }
                }
                Event::Mouse(mouse_event) => {
                    let mx = mouse_event.column;
                    let my = mouse_event.row;
                    let mut click = false;
                    let mut scroll_dir = 0;

                    match mouse_event.kind {
                        MouseEventKind::Down(MouseButton::Left) => {
                            click = true;
                        }
                        MouseEventKind::ScrollUp => {
                            scroll_dir = -1;
                            ctx.scroll_offset_y = 1;
                        }
                        MouseEventKind::ScrollDown => {
                            scroll_dir = 1;
                            ctx.scroll_offset_y = 1;
                        }
                        _ => {}
                    }

                    if click || scroll_dir != 0 {
                        let (changed, btn_id) = ctx.handle_mouse(mx, my, click, scroll_dir);
                        if changed {
                            state_changed = true;
                        }
                        if let Some(id) = btn_id {
                            ctx.clicked_button = Some(id);
                            ctx.scroll_offset_y = 0;
                            return 2;
                        }
                    }
                }
                Event::Resize(_, _) => {
                    state_changed = true;
                }
                _ => {}
            }
        }

        if state_changed {
            ctx.needs_redraw = true;
        }

        if quit_requested {
            1
        } else if submitted {
            2
        } else {
            0
        }
    });

    result.unwrap_or(-2)
}