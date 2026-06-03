use crate::*;
use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_int};
use std::panic::catch_unwind;
use std::ptr::{self, null_mut};
use std::time::Duration;
use crossterm::event::Event::Mouse;
use crossterm::event::{Event, KeyCode, KeyEventKind, KeyModifiers, MouseButton, MouseEventKind, poll, read};
use crate::{STATUS_ERR_NULL_PTR, STATUS_ERR_PANIC, STATUS_OK, STATUS_QUIT, STATUS_ERR_INVALID_ID, STATUS_CLICKED};

// This function creates a new node in the virtual DOM based on the provided type and text content. 
// It checks for null pointers and handles any panics gracefully, returning appropriate status codes 
// based on the outcome of the node creation process.
#[no_mangle]
pub extern "C" fn scarpe_tui_create_node(
    ctx_ptr: *mut ScarpeTuiContext,
    node_type_code: c_int,
    text_ptr: *const c_char, 
) -> c_int {
    if ctx_ptr.is_null() {
        return STATUS_ERR_NULL_PTR;
    }

    let result = catch_unwind(|| {
        let ctx = unsafe { &mut *ctx_ptr };
        let vacant_entry = ctx.nodes.vacant_entry();
        let new_id = vacant_entry.key();

        let node_type = match node_type_code {
            0 => NodeType::Root,
            1 => NodeType::Stack,
            2 => NodeType::Flow,
            3 => {
                // Extract text content for a Text node, if provided.
                let text_content = if !text_ptr.is_null() {
                    unsafe { CStr::from_ptr(text_ptr) }
                        .to_string_lossy()
                        .into_owned()
                } else {
                    String::new()
                };
                NodeType::Text(text_content)
            }
            4 => {
                // Extract text content for an EditLine node, if provided.
                let text_content = if !text_ptr.is_null() {
                    unsafe { CStr::from_ptr(text_ptr) }
                        .to_string_lossy()
                        .into_owned()
                } else {
                    String::new()
                };

                // Set focus to this EditLine if no other node is focused.
                if ctx.focused_node.is_none() {
                    ctx.focused_node = Some(new_id);
                }
                NodeType::EditLine(text_content)
            }
            5 => {
                // Extract text content for a Button node, if provided.
                let text_content = if !text_ptr.is_null() {
                    unsafe { std::ffi::CStr::from_ptr(text_ptr) }.to_string_lossy().into_owned()
                } else { String::new() };
                NodeType::Button(text_content)
            }
            6 => NodeType::Checkbox(false),
            _ => NodeType::Stack, // Default to Stack for unknown types.
        };

        if node_type_code == 0 {
            ctx.root_id = Some(new_id);
        }
        
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

// This function initializes the TUI context and returns a pointer to it.
// It ensures that any panics during initialization are caught, returning a null pointer instead of crashing.
#[no_mangle]
pub extern "C" fn scarpe_tui_init(use_alternate: bool) -> *mut ScarpeTuiContext {
    let result = catch_unwind(|| match ScarpeTuiContext::new(use_alternate) {
        Ok(ctx) => Box::into_raw(Box::new(ctx)),
        Err(_) => ptr::null_mut(),
    });

    result.unwrap_or(ptr::null_mut())
}

// This function renders the current state of the TUI context to the terminal. 
// It checks for null pointers and handles any panics gracefully, 
// returning appropriate status codes based on the outcome of the rendering process.
#[no_mangle]
pub extern "C" fn scarpe_tui_render(ctx_ptr: *mut ScarpeTuiContext) -> c_int {
    if ctx_ptr.is_null() {
        return STATUS_ERR_NULL_PTR;
    }

    let result = catch_unwind(|| {
        let ctx = unsafe { &mut *ctx_ptr };
        match ctx.render() {
            Ok(_) => STATUS_OK,
            Err(_) => STATUS_ERR_IO,
        }
    });

    result.unwrap_or(STATUS_ERR_PANIC)
}

// This function frees the resources associated with the TUI context. 
// It checks for null pointers and ensures that the shutdown process is handled gracefully.
#[no_mangle]
pub extern "C" fn scarpe_tui_free_context(ctx_ptr: *mut ScarpeTuiContext) {
    if ctx_ptr.is_null() {
        return;
    }

    let _ = catch_unwind(|| {
        let mut ctx = unsafe { Box::from_raw(ctx_ptr) };
        let _ = ctx.shutdown();
    });
}

// This function appends a child node to a parent node in the virtual DOM. 
// It checks for null pointers and valid IDs before performing the operation.
#[no_mangle]
pub extern "C" fn scarpe_tui_append_child(
    ctx_ptr: *mut ScarpeTuiContext,
    parent_id: c_int,
    child_id: c_int,
) -> c_int {
    if ctx_ptr.is_null() {
        return STATUS_ERR_NULL_PTR;
    }

    if parent_id < 0 || child_id < 0 {
        return STATUS_ERR_INVALID_ID;
    }

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

// This function retrieves the text content of a node, specifically for EditLine nodes.
// It returns a pointer to a C-string, which must be freed later to avoid memory leaks.
#[no_mangle]
pub extern "C" fn scarpe_tui_get_text(ctx_ptr: *mut ScarpeTuiContext, node_id: c_int) -> *mut c_char {
    if ctx_ptr.is_null() || node_id < 0 {
        return null_mut();
    }
    
    let result = catch_unwind(|| {
        let ctx = unsafe { &mut *ctx_ptr };
        
        if let Some(node) = ctx.nodes.get(node_id as usize) {
            if let NodeType::EditLine(ref text) = node.node_type {
                // Create a C-string and pass ownership out of Rust.
                if let Ok(c_string) = CString::new(text.clone()) {
                    return c_string.into_raw(); 
                }
            }
        }
        null_mut()
    });

    result.unwrap_or(null_mut())
}

// This function frees a C-string that was previously allocated in Rust.
#[no_mangle]
pub extern "C" fn scarpe_tui_free_string(s: *mut c_char) {
    if s.is_null() { return; }
    let _ = catch_unwind(|| {
        unsafe {
            let _ = CString::from_raw(s); 
        }
    });
}

// This function retrieves the ID of the last clicked button, if any.
// It resets the clicked button state after reading.
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
#[no_mangle]
pub extern "C" fn scarpe_tui_get_checkbox_state(ctx_ptr: *mut ScarpeTuiContext, node_id: c_int) -> c_int {
    if ctx_ptr.is_null() || node_id < 0 {
        return -1;
    }
    
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

// This function sets the style of a node, including foreground color, background color, and text modifiers.
#[no_mangle]
pub extern "C" fn scarpe_tui_set_style(
    ctx_ptr: *mut ScarpeTuiContext,
    node_id: c_int,
    fg: c_int,
    bg: c_int,
    modifier: c_int,
) -> c_int {
    if ctx_ptr.is_null() || node_id < 0 {
        return STATUS_ERR_NULL_PTR;
    }
    
    let result = catch_unwind(|| {
        let ctx = unsafe { &mut *ctx_ptr };
        if let Some(node) = ctx.nodes.get_mut(node_id as usize) {
            
            // Map the integer values to crossterm colors and attributes, using Reset for out-of-range values.
            node.style.fg = if (0..=255).contains(&fg) {
                crossterm::style::Color::AnsiValue(fg as u8)
            } else {
                crossterm::style::Color::Reset
            };

            // Background color mapping, using Reset for out-of-range values.
            node.style.bg = if (0..=255).contains(&bg) {
                crossterm::style::Color::AnsiValue(bg as u8)
            } else {
                crossterm::style::Color::Reset
            };

            // Modifier mapping, using Reset for out-of-range values.
            node.style.modifier = match modifier {
                1 => crossterm::style::Attribute::Bold,
                2 => crossterm::style::Attribute::Underlined,
                3 => crossterm::style::Attribute::Italic,
                4 => crossterm::style::Attribute::Reverse,
                _ => crossterm::style::Attribute::Reset,
            };
            
            ctx.needs_redraw = true; 
            return STATUS_OK;
        }
        STATUS_ERR_INVALID_ID
    });

    result.unwrap_or(STATUS_ERR_PANIC)
}

// This function polls for terminal events, such as key presses or mouse clicks, and processes them.
// It updates the application state and returns a status code indicating the result.
#[no_mangle]
pub extern "C" fn scarpe_tui_poll_events(ctx_ptr: *mut ScarpeTuiContext) -> c_int {
    if ctx_ptr.is_null() {
        return STATUS_ERR_NULL_PTR;
    }

    let result = catch_unwind(|| {
        let ctx = unsafe { &mut *ctx_ptr };
        let mut quit_requested = false;
        let mut state_changed = false; 
        while let Ok(true) = poll(Duration::from_millis(0)) {
            match read() {
                Ok(Event::Key(key_event)) => {
                    if key_event.kind != KeyEventKind::Press {
                        continue;
                    }
                    // Handle exit logic.
                    if key_event.code == KeyCode::Esc || 
                       (key_event.modifiers.contains(KeyModifiers::CONTROL) && key_event.code == KeyCode::Char('c')) {
                        quit_requested = true;
                        break; 
                    }

                    if key_event.code == crossterm::event::KeyCode::Tab {
                        // Collect all EditLine node IDs
                        let mut edit_lines = Vec::new();
                        for (id, node) in ctx.nodes.iter() {
                            if matches!(node.node_type, NodeType::EditLine(_)) {
                                edit_lines.push(id);
                            }
                        }
                        
                        // Pass focus to the next EditLine in the array
                        if !edit_lines.is_empty() {
                            let current_idx = edit_lines.iter().position(|&id| Some(id) == ctx.focused_node).unwrap_or(0);
                            let next_idx = (current_idx + 1) % edit_lines.len();
                            ctx.focused_node = Some(edit_lines[next_idx]);
                            
                            state_changed = true; // Signal the render to redraw
                        }
                        continue; 
                    }
                    
                    // Handle typing in EditLine nodes.
                    if let Some(focus_id) = ctx.focused_node {
                        if let Some(node) = ctx.nodes.get_mut(focus_id) {
                            if let NodeType::EditLine(ref mut text) = node.node_type {
                                match key_event.code {
                                    KeyCode::Char(c) if key_event.modifiers.is_empty() || key_event.modifiers == KeyModifiers::SHIFT => {
                                        text.push(c);
                                        state_changed = true;
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
                Ok(Mouse(mouse_event)) => {
                    match mouse_event.kind {
                        MouseEventKind::Down(MouseButton::Left) => {
                            let mx = mouse_event.column;
                            let my = mouse_event.row;

                        
                            let absolute_y = my.saturating_add(ctx.scroll_offset_y);

                            let mut clicked_button_id = None;
                            let mut clicked_checkbox_id = None;

                            for (id, node) in ctx.nodes.iter() {
                                let l = node.layout;
                                if mx >= l.x && mx < l.x + l.width && absolute_y >= l.y && absolute_y < l.y + l.height {
                                    if matches!(node.node_type, NodeType::Button(_)) {
                                        clicked_button_id = Some(id);
                                        break;
                                    } else if matches!(node.node_type, NodeType::Checkbox(_)) {
                                        clicked_checkbox_id = Some(id);
                                        break;
                                    }
                                }
                            }

                            // Set the clicked button ID in the context if a button was clicked, which will be read by the application logic.
                            if let Some(id) = clicked_button_id {
                                ctx.clicked_button = Some(id);
                                return STATUS_CLICKED; 
                            }

                            // Toggle the checkbox state if a checkbox was clicked.
                            if let Some(id) = clicked_checkbox_id {
                                if let Some(node) = ctx.nodes.get_mut(id) {
                                    if let NodeType::Checkbox(ref mut state) = node.node_type {
                                        *state = !*state; // Toggle the checkbox state
                                        state_changed = true; // Signal the render to redraw with the new checkbox state
                                    }
                                }
                            }
                        }
                        MouseEventKind::ScrollUp => {

                            // Scroll up by decreasing the scroll offset, ensuring it doesn't go below zero.
                            if ctx.scroll_offset_y > 0 {
                                ctx.scroll_offset_y = ctx.scroll_offset_y.saturating_sub(1);
                                state_changed = true;
                            }
                        }
                        MouseEventKind::ScrollDown => {

                            // Calculate the maximum content height based on the layout of the root node.
                            let mut max_content_height = 0;
                            if let Some(root_id) = ctx.root_id {
                                if let Some(root) = ctx.nodes.get(root_id) {
                                    max_content_height = root.layout.height;
                                }
                            }

                            // The maximum scroll offset is the total content height minus the terminal height, 
                            // ensuring we don't scroll past the end of the content.
                            let term_height = ctx.next_buffer.height;   
                            let max_scroll = max_content_height.saturating_sub(term_height);

                            if ctx.scroll_offset_y < max_scroll {
                                ctx.scroll_offset_y = ctx.scroll_offset_y.saturating_add(1);
                                state_changed = true;
                            }
                        }
                        _ => {}
                    }
                }
                Ok(Event::Resize(_, _)) => {
                    state_changed = true;
                }
                _ => {}
            }
        }
        
        if state_changed {
            ctx.needs_redraw = true;
        }

        if quit_requested { STATUS_QUIT } else { STATUS_OK }
    });

    result.unwrap_or(STATUS_ERR_PANIC)
}