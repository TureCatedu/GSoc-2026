use crate::*;
use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_int};
use std::panic::catch_unwind;
use std::ptr::{self, null_mut};
// use std::time::Duration;

use crossterm::event::{poll, read, Event, KeyCode, KeyModifiers};
use crate::{STATUS_ERR_NULL_PTR, STATUS_ERR_PANIC, STATUS_OK, STATUS_QUIT, STATUS_ERR_INVALID_ID};

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

                let text_content = if !text_ptr.is_null() {
                    unsafe { CStr::from_ptr(text_ptr) }
                        .to_string_lossy()
                        .into_owned()
                } else {
                    String::new()
                };

                if ctx.focused_node.is_none() {
                    ctx.focused_node = Some(new_id);
                }
                NodeType::EditLine(text_content)
            }
            _ => NodeType::Stack, // Default to Stack for unknown types
        };

        if node_type_code == 0 {
            ctx.root_id = Some(new_id);
        }
        
        vacant_entry.insert(Node {
            id: new_id,
            node_type,
            children: Vec::new(),
            layout: ComputedLayout::default(), 
        });

        ctx.needs_redraw = true;
        new_id as c_int
    });

    result.unwrap_or(STATUS_ERR_PANIC)
}

// This function initializes the TUI context and returns a pointer to it.
#[no_mangle]
pub extern "C" fn scarpe_tui_init(use_alternate: bool) -> *mut ScarpeTuiContext {
    // We use `catch_unwind` to ensure that any panics during initialization are caught,
    // allowing us to return a null pointer instead of crashing the application.
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

// This function appends a child node to a parent node in the virtual DOM. It checks for null pointers and valid IDs before performing the operation.
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

#[no_mangle]
pub extern "C" fn scarpe_tui_get_text(ctx_ptr: *mut ScarpeTuiContext, node_id: c_int) -> *mut c_char {
    if ctx_ptr.is_null() || node_id < 0 {
        return null_mut();
    }
    
    let result = catch_unwind(|| {
        let ctx = unsafe { &mut *ctx_ptr };
        
        if let Some(node) = ctx.nodes.get(node_id as usize) {
            if let NodeType::EditLine(ref text) = node.node_type {
                // Creiamo una C-String e passiamo la proprietà (ownership) fuori da Rust
                if let Ok(c_string) = CString::new(text.clone()) {
                    return c_string.into_raw(); 
                }
            }
        }
        null_mut()
    });

    result.unwrap_or(null_mut())
}

#[no_mangle]
pub extern "C" fn scarpe_tui_free_string(s: *mut c_char) {
    if s.is_null() { return; }
    let _ = catch_unwind(|| {
        unsafe {
            let _ = CString::from_raw(s); 
        }
    });
}

// This function polls for terminal events, such as key presses, and returns a status code indicating the result.
#[no_mangle]
pub extern "C" fn scarpe_tui_poll_events(ctx_ptr: *mut ScarpeTuiContext) -> c_int {
    if ctx_ptr.is_null() {
        return STATUS_ERR_NULL_PTR;
    }

    let result = catch_unwind(|| {
        let ctx = unsafe { &mut *ctx_ptr };
        let mut quit_requested = false;
        let mut state_changed = false; 

        while let Ok(true) = poll(std::time::Duration::from_millis(0)) {
            match read() {
                Ok(Event::Key(key_event)) => {
                    // Logica di uscita
                    if key_event.code == KeyCode::Esc || 
                       (key_event.modifiers.contains(KeyModifiers::CONTROL) && key_event.code == KeyCode::Char('c')) {
                        quit_requested = true;
                        break; 
                    }

                    // Digitazione
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