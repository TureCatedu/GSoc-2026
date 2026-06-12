use crate::*;
use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_int};
use std::panic::catch_unwind;
use std::ptr::{self, null_mut};
use crossterm::event::{Event, KeyCode, KeyEventKind, KeyModifiers, MouseButton, MouseEventKind};
use crate::{STATUS_ERR_NULL_PTR, STATUS_ERR_PANIC, STATUS_OK, STATUS_QUIT, STATUS_ERR_INVALID_ID, STATUS_CLICKED};

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
                NodeType::EditLine(text_content)
            }
            5 => {
                let text_content = if !text_ptr.is_null() {
                    unsafe { std::ffi::CStr::from_ptr(text_ptr) }.to_string_lossy().into_owned()
                } else { String::new() };
                NodeType::Button(text_content)
            }
            6 => NodeType::Checkbox(false),
            7 => NodeType::Border,
            8 => {
                let text_content = if !text_ptr.is_null() {
                    unsafe { CStr::from_ptr(text_ptr) }.to_string_lossy().into_owned()
                } else { String::new() };
                NodeType::EditBox(text_content)
            }
            9 => NodeType::DockBottom,
            10 => {
                let limit = if !text_ptr.is_null() {
                    unsafe { std::ffi::CStr::from_ptr(text_ptr) }.to_string_lossy().parse::<u16>().unwrap_or(10)
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

#[no_mangle]
pub extern "C" fn scarpe_tui_init(use_alternate: bool) -> *mut ScarpeTuiContext {
    let result = catch_unwind(|| match ScarpeTuiContext::new(use_alternate) {
        Ok(ctx) => Box::into_raw(Box::new(ctx)),
        Err(_) => ptr::null_mut(),
    });
    result.unwrap_or(ptr::null_mut())
}

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

#[no_mangle]
pub extern "C" fn scarpe_tui_free_context(ctx_ptr: *mut ScarpeTuiContext) {
    if ctx_ptr.is_null() { return; }
    let _ = catch_unwind(|| {
        let mut ctx = unsafe { Box::from_raw(ctx_ptr) };
        let _ = ctx.shutdown();
    });
}

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

#[no_mangle]
pub extern "C" fn scarpe_tui_free_string(s: *mut c_char) {
    if s.is_null() { return; }
    let _ = catch_unwind(|| {
        unsafe { let _ = CString::from_raw(s); }
    });
}

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

#[no_mangle]
pub extern "C" fn scarpe_tui_set_style(
    ctx_ptr: *mut ScarpeTuiContext, node_id: c_int, fg: c_int, bg: c_int, modifier: c_int,
) -> c_int {
    if ctx_ptr.is_null() || node_id < 0 { return STATUS_ERR_NULL_PTR; }
    let result = catch_unwind(|| {
        let ctx = unsafe { &mut *ctx_ptr };
        if let Some(node) = ctx.nodes.get_mut(node_id as usize) {
            node.style.fg = if (0..=255).contains(&fg) { crossterm::style::Color::AnsiValue(fg as u8) } else { crossterm::style::Color::Reset };
            node.style.bg = if (0..=255).contains(&bg) { crossterm::style::Color::AnsiValue(bg as u8) } else { crossterm::style::Color::Reset };
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

#[no_mangle]
pub extern "C" fn scarpe_tui_update_text(
    ctx_ptr: *mut ScarpeTuiContext, node_id: c_int, new_text_ptr: *const c_char,
) -> c_int {
    if ctx_ptr.is_null() || new_text_ptr.is_null() || node_id < 0 { return STATUS_ERR_NULL_PTR; }
    let result = std::panic::catch_unwind(|| {
        let ctx = unsafe { &mut *ctx_ptr };
        let new_text = unsafe { std::ffi::CStr::from_ptr(new_text_ptr) }.to_string_lossy().into_owned();
        
        if let Some(node) = ctx.nodes.get_mut(node_id as usize) {
            match &mut node.node_type {
                NodeType::Text(ref mut text) |
                NodeType::EditLine(ref mut text) |
                NodeType::EditBox(ref mut text) |
                NodeType::Button(ref mut text) => {
                    *text = new_text;
                    ctx.needs_redraw = true;
                    return STATUS_OK;
                }
                _ => return STATUS_ERR_INVALID_ID,
            }
        }
        STATUS_ERR_INVALID_ID
    });
    result.unwrap_or(STATUS_ERR_PANIC)
}

// This function polls for user input events (keyboard and mouse) and updates the TUI context accordingly.
#[no_mangle]
pub extern "C" fn scarpe_tui_poll_events(ctx_ptr: *mut ScarpeTuiContext) -> c_int {
    if ctx_ptr.is_null() { return STATUS_ERR_NULL_PTR; }

    let result = catch_unwind(|| {
        let ctx = unsafe { &mut *ctx_ptr };
        let mut quit_requested = false;
        let mut state_changed = false; 
        
        // 1. FASE DI LETTURA: Estraiamo tutti gli eventi e li salviamo.
        // Questo rilascia immediatamente il "prestito" (borrow) su ctx.event_receiver
        let mut pending_events = Vec::new();
        if let Some(ref rx) = ctx.event_receiver {
            while let Ok(event) = rx.try_recv() {
                pending_events.push(event);
            }
        }
        
        // 2. FASE DI SCRITTURA/LOGICA: Ora ctx è completamente libero di essere mutato!
        for event in pending_events {
            match event {
                Event::Key(key_event) => {
                    if key_event.kind != KeyEventKind::Press { continue; }
                    
                    if key_event.code == KeyCode::Esc || 
                       (key_event.modifiers.contains(KeyModifiers::CONTROL) && key_event.code == KeyCode::Char('c')) {
                        quit_requested = true;
                        break; 
                    }

                    if key_event.code == KeyCode::Tab {
                        let mut inputs = Vec::new();
                        for (id, node) in ctx.nodes.iter() {
                            if matches!(node.node_type, NodeType::EditLine(_)) || matches!(node.node_type, NodeType::EditBox(_)) {
                                inputs.push(id);
                            }
                        }
                        if !inputs.is_empty() {
                            let current_idx = inputs.iter().position(|&id| Some(id) == ctx.focused_node).unwrap_or(0);
                            let next_idx = (current_idx + 1) % inputs.len();
                            ctx.focused_node = Some(inputs[next_idx]);
                            state_changed = true; 
                        }
                        continue; 
                    }
                    
                    if let Some(focus_id) = ctx.focused_node {
                        if let Some(node) = ctx.nodes.get_mut(focus_id) {
                            if let NodeType::EditLine(ref mut text) = node.node_type {
                                match key_event.code {
                                    KeyCode::Char(c) if key_event.modifiers.is_empty() || key_event.modifiers == KeyModifiers::SHIFT => {
                                        text.push(c);
                                        state_changed = true;
                                    }
                                    KeyCode::Backspace => {
                                        if text.pop().is_some() { state_changed = true; }
                                    }
                                    _ => {}
                                }
                            }
                            if let NodeType::EditBox(ref mut text) = node.node_type {
                                match key_event.code {
                                    KeyCode::Char(c) if key_event.modifiers.is_empty() || key_event.modifiers == KeyModifiers::SHIFT => {
                                        text.push(c);
                                        state_changed = true;
                                    }
                                    KeyCode::Enter => { 
                                        text.push('\n');
                                        state_changed = true;
                                    }
                                    KeyCode::Backspace => {
                                        if text.pop().is_some() { state_changed = true; }
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
                        MouseEventKind::Down(MouseButton::Left) => { click = true; }
                        MouseEventKind::ScrollUp => { scroll_dir = -1; }
                        MouseEventKind::ScrollDown => { scroll_dir = 1; }
                        _ => {}
                    }

                    // Ora chiamare handle_mouse è perfettamente legale per il Borrow Checker!
                    if click || scroll_dir != 0 {
                        let (changed, btn_id) = ctx.handle_mouse(mx, my, click, scroll_dir);
                        if changed { state_changed = true; }
                        if let Some(id) = btn_id {
                            ctx.clicked_button = Some(id);
                            return STATUS_CLICKED;
                        }
                    }
                }
                Event::Resize(_, _) => { state_changed = true; }
                _ => {}
            }
        }
        
        if state_changed { ctx.needs_redraw = true; }
        if quit_requested { STATUS_QUIT } else { STATUS_OK }
    });
    result.unwrap_or(STATUS_ERR_PANIC)
}