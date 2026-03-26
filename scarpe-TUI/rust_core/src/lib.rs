use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind, MouseButton, MouseEventKind, EnableMouseCapture, DisableMouseCapture},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use ratatui::{
    backend::CrosstermBackend,
    layout::Rect,
    style::{Color, Style, Stylize},
    text::Span,
    widgets::{Block, Borders, Paragraph as TuiParagraph, Clear},
    Terminal,
};
use std::ffi::{CStr, CString};
use std::io::{stdout, Stdout}; // FIX 1: Rimosso `self`
use std::os::raw::c_char;
use std::sync::Mutex;
use std::time::Duration;

#[derive(Clone)]
enum WidgetNode {
    BeginStack,
    EndStack,
    BeginFlow,
    EndFlow,
    Paragraph(String),
    Button { id: i32, text: String },
    Input { id: i32, text: String }, 
}

struct ClickArea {
    id: i32,
    index: usize,
    x1: u16, y1: u16,
    x2: u16, y2: u16,
    is_input: bool,
}

#[derive(PartialEq)]
enum LayoutDir { Vertical, Horizontal }

static DOM_TREE: Mutex<Vec<WidgetNode>> = Mutex::new(Vec::new());
static FOCUSED_WIDGET: Mutex<usize> = Mutex::new(0);
static CLICK_AREAS: Mutex<Vec<ClickArea>> = Mutex::new(Vec::new());
static SCROLL_OFFSET: Mutex<u16> = Mutex::new(0);
static APP_TERMINAL: Mutex<Option<Terminal<CrosstermBackend<Stdout>>>> = Mutex::new(None);

#[no_mangle]
pub extern "C" fn scarpe_tui_init() {
    enable_raw_mode().unwrap();
    stdout().execute(EnterAlternateScreen).unwrap().execute(EnableMouseCapture).unwrap();
    let backend = CrosstermBackend::new(stdout());
    *APP_TERMINAL.lock().unwrap() = Some(Terminal::new(backend).unwrap());
}

#[no_mangle]
pub extern "C" fn scarpe_tui_shutdown() {
    DOM_TREE.lock().unwrap().clear(); 
    CLICK_AREAS.lock().unwrap().clear();
    *SCROLL_OFFSET.lock().unwrap() = 0; 
    let _ = APP_TERMINAL.lock().unwrap().take();
    stdout().execute(DisableMouseCapture).unwrap().execute(LeaveAlternateScreen).unwrap();
    disable_raw_mode().unwrap();
}

#[no_mangle] pub extern "C" fn scarpe_tui_begin_stack() { DOM_TREE.lock().unwrap().push(WidgetNode::BeginStack); }
#[no_mangle] pub extern "C" fn scarpe_tui_end_stack() { DOM_TREE.lock().unwrap().push(WidgetNode::EndStack); }
#[no_mangle] pub extern "C" fn scarpe_tui_begin_flow() { DOM_TREE.lock().unwrap().push(WidgetNode::BeginFlow); }
#[no_mangle] pub extern "C" fn scarpe_tui_end_flow() { DOM_TREE.lock().unwrap().push(WidgetNode::EndFlow); }

#[no_mangle]
pub extern "C" fn scarpe_tui_add_paragraph(c_text: *const c_char) {
    if c_text.is_null() { return; }
    if let Ok(str_slice) = unsafe { CStr::from_ptr(c_text) }.to_str() {
        DOM_TREE.lock().unwrap().push(WidgetNode::Paragraph(str_slice.to_owned()));
    }
}

#[no_mangle]
pub extern "C" fn scarpe_tui_add_button(id: i32, c_text: *const c_char) {
    if c_text.is_null() { return; }
    if let Ok(str_slice) = unsafe { CStr::from_ptr(c_text) }.to_str() {
        let mut tree = DOM_TREE.lock().unwrap();
        let is_first = !tree.iter().any(|w| matches!(w, WidgetNode::Button { .. } | WidgetNode::Input { .. }));
        tree.push(WidgetNode::Button { id, text: str_slice.to_owned() });
        if is_first { *FOCUSED_WIDGET.lock().unwrap() = tree.len() - 1; }
    }
}

#[no_mangle]
pub extern "C" fn scarpe_tui_add_input(id: i32, c_text: *const c_char) {
    if c_text.is_null() { return; }
    if let Ok(str_slice) = unsafe { CStr::from_ptr(c_text) }.to_str() {
        let mut tree = DOM_TREE.lock().unwrap();
        let is_first = !tree.iter().any(|w| matches!(w, WidgetNode::Button { .. } | WidgetNode::Input { .. }));
        tree.push(WidgetNode::Input { id, text: str_slice.to_owned() });
        if is_first { *FOCUSED_WIDGET.lock().unwrap() = tree.len() - 1; }
    }
}

#[no_mangle]
pub extern "C" fn scarpe_tui_get_input_text(id: i32) -> *mut c_char {
    let tree = DOM_TREE.lock().unwrap();
    for node in tree.iter() {
        if let WidgetNode::Input { id: node_id, text } = node {
            if *node_id == id { return CString::new(text.clone()).unwrap().into_raw(); }
        }
    }
    CString::new("").unwrap().into_raw()
}

#[no_mangle]
pub extern "C" fn scarpe_tui_free_string(s: *mut c_char) {
    if !s.is_null() { unsafe { let _ = CString::from_raw(s); } }
}

#[no_mangle]
pub extern "C" fn scarpe_tui_render() {
    let tree = DOM_TREE.lock().unwrap();
    let current_focus = *FOCUSED_WIDGET.lock().unwrap();
    let mut click_areas = CLICK_AREAS.lock().unwrap();
    click_areas.clear();

    let mut term_guard = APP_TERMINAL.lock().unwrap();
    if let Some(terminal) = term_guard.as_mut() {
        terminal.draw(|frame| {
            frame.render_widget(Clear, frame.size());
            let max_w = frame.size().width; let max_h = frame.size().height;
            let main_block = Block::default().title(" Scarpe App (Mouse Support) ").borders(Borders::ALL);
            frame.render_widget(main_block, frame.size());

            let scroll_y = *SCROLL_OFFSET.lock().unwrap();
            let mut x = 2; let mut logical_y = 2; 
            let mut layout_stack = vec![LayoutDir::Vertical]; let mut row_max_h = 0;

            for (index, node) in tree.iter().enumerate() {
                let current_dir = layout_stack.last().unwrap_or(&LayoutDir::Vertical);
                let is_focused = index == current_focus;

                match node {
                    WidgetNode::BeginStack => { layout_stack.push(LayoutDir::Vertical); x = 2; }
                    WidgetNode::EndStack => { layout_stack.pop(); }
                    WidgetNode::BeginFlow => { layout_stack.push(LayoutDir::Horizontal); row_max_h = 0; }
                    WidgetNode::EndFlow => { layout_stack.pop(); logical_y += row_max_h; x = 2; }
                    
                    WidgetNode::Paragraph(text) => {
                        let w = text.chars().count() as u16; let h = 1;
                        let physical_y = logical_y as i32 - scroll_y as i32;
                        if physical_y >= 0 && physical_y < max_h as i32 && x < max_w {
                            let y = physical_y as u16;
                            let safe_w = w.min(max_w.saturating_sub(x)); let safe_h = h.min(max_h.saturating_sub(y));
                            if safe_w > 0 && safe_h > 0 {
                                let rect = Rect::new(x, y, safe_w, safe_h);
                                frame.render_widget(TuiParagraph::new(text.clone()), rect);
                            }
                        }
                        if *current_dir == LayoutDir::Vertical { logical_y += h + 1; } else { x += w + 2; row_max_h = row_max_h.max(h + 1); }
                    }
                    
                    WidgetNode::Button { id, text } => {
                        let btn_text = format!(" [ {} ] ", text);
                        let w = btn_text.chars().count() as u16; let h = 1;
                        let physical_y = logical_y as i32 - scroll_y as i32;
                        if physical_y >= 0 && physical_y < max_h as i32 && x < max_w {
                            let y = physical_y as u16;
                            let safe_w = w.min(max_w.saturating_sub(x)); let safe_h = h.min(max_h.saturating_sub(y));
                            if safe_w > 0 && safe_h > 0 {
                                let rect = Rect::new(x, y, safe_w, safe_h);
                                click_areas.push(ClickArea { id: *id, index, x1: x, y1: y, x2: x+safe_w-1, y2: y, is_input: false });
                                let style = if is_focused { Style::default().bg(Color::Yellow).fg(Color::Black).bold() } else { Style::default().bg(Color::Blue).fg(Color::White).bold() };
                                frame.render_widget(TuiParagraph::new(Span::styled(btn_text, style)), rect);
                            }
                        }
                        if *current_dir == LayoutDir::Vertical { logical_y += h + 1; } else { x += w + 1; row_max_h = row_max_h.max(h + 1); }
                    }

                    WidgetNode::Input { id, text } => {
                        let mut padded_text = text.clone();
                        let char_count = padded_text.chars().count();
                        if char_count < 20 {
                            let padding: String = vec![' '; 20 - char_count].into_iter().collect();
                            padded_text.push_str(&padding);
                        }

                        let display_text = if is_focused { format!(" > {}_ ", padded_text) } else { format!(" > {}  ", padded_text) };
                        let w = display_text.chars().count() as u16; let h = 1;
                        let physical_y = logical_y as i32 - scroll_y as i32;
                        if physical_y >= 0 && physical_y < max_h as i32 && x < max_w {
                            let y = physical_y as u16;
                            let safe_w = w.min(max_w.saturating_sub(x)); let safe_h = h.min(max_h.saturating_sub(y));
                            if safe_w > 0 && safe_h > 0 {
                                let rect = Rect::new(x, y, safe_w, safe_h);
                                click_areas.push(ClickArea { id: *id, index, x1: x, y1: y, x2: x+safe_w-1, y2: y, is_input: true });
                                let style = if is_focused { Style::default().bg(Color::White).fg(Color::Black) } else { Style::default().bg(Color::DarkGray).fg(Color::White) };
                                frame.render_widget(TuiParagraph::new(Span::styled(display_text, style)), rect);
                            }
                        }
                        if *current_dir == LayoutDir::Vertical { logical_y += h + 1; } else { x += w + 1; row_max_h = row_max_h.max(h + 1); }
                    }
                }
            }
        }).unwrap();
    }
}

#[no_mangle]
pub extern "C" fn scarpe_tui_poll_event() -> i32 {
    if event::poll(Duration::from_millis(50)).unwrap_or(false) {
        if let Ok(event) = event::read() {
            match event {
                Event::Key(key) => {
                    if key.kind == KeyEventKind::Release { return 0; }
                    let mut focus = FOCUSED_WIDGET.lock().unwrap();
                    let mut tree = DOM_TREE.lock().unwrap();
                    
                    let interactables: Vec<usize> = tree.iter().enumerate().filter_map(|(i, w)| {
                        if matches!(w, WidgetNode::Button { .. } | WidgetNode::Input { .. }) { Some(i) } else { None }
                    }).collect();

                    if interactables.is_empty() {
                        if key.code == KeyCode::Char('q') || key.code == KeyCode::Esc { return -1; }
                        return 0;
                    }

                    let mut current_pos = interactables.iter().position(|&i| i == *focus).unwrap_or(0);

                    match key.code {
                        KeyCode::Esc => return -1, 
                        KeyCode::Down | KeyCode::Tab => { current_pos = (current_pos + 1) % interactables.len(); *focus = interactables[current_pos]; }
                        KeyCode::Up => { current_pos = if current_pos == 0 { interactables.len() - 1 } else { current_pos - 1 }; *focus = interactables[current_pos]; }
                        KeyCode::Enter => { if let WidgetNode::Button { id, .. } = &tree[*focus] { return *id; } }
                        KeyCode::Backspace => { if let WidgetNode::Input { text, .. } = &mut tree[*focus] { text.pop(); } }
                        KeyCode::Char(c) => {
                            if let WidgetNode::Input { text, .. } = &mut tree[*focus] {
                                text.push(c); 
                            } else if c == 'q' {
                                return -1; 
                            }
                        }
                        _ => {}
                    }
                },
                Event::Mouse(mouse_event) => {
                    let mut scroll = SCROLL_OFFSET.lock().unwrap();
                    match mouse_event.kind {
                        MouseEventKind::ScrollDown => { *scroll = scroll.saturating_add(2); }
                        MouseEventKind::ScrollUp => { *scroll = scroll.saturating_sub(2); }
                        MouseEventKind::Down(MouseButton::Left) => {
                            let click_areas = CLICK_AREAS.lock().unwrap();
                            for area in click_areas.iter() {
                                // FIX 2: Ora controlliamo anche y2 per gestire futuri widget multi-riga
                                if mouse_event.column >= area.x1 && mouse_event.column <= area.x2 && mouse_event.row >= area.y1 && mouse_event.row <= area.y2 {
                                    *FOCUSED_WIDGET.lock().unwrap() = area.index;
                                    
                                    if area.is_input {
                                        return 0;
                                    } else {
                                        return area.id;
                                    }
                                }
                            }
                        }
                        _ => {}
                    }
                },
                _ => {}
            }
        }
    }
    0
}