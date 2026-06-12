mod context;
mod ffi;

use std::sync::mpsc::Receiver;

use std::os::raw::c_int;

use crossterm::event::Event;
use crossterm::style::{Print, Color, Attribute};
use crossterm::{
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use slab::Slab;

// Constants representing status codes for various outcomes
const STATUS_OK: c_int = 0; 
const STATUS_ERR_NULL_PTR: c_int = -1; 
const STATUS_ERR_PANIC: c_int = -2; 
const STATUS_ERR_IO: c_int = -3; 
const STATUS_ERR_INVALID_ID: c_int = -4; 
const STATUS_QUIT: c_int = 1; 
const STATUS_CLICKED: c_int = 2; 

pub type NodeId = usize; 

// Enum representing different types of nodes in the virtual DOM
#[derive(Debug, Clone)]
pub enum NodeType {
    Root, 
    Stack, 
    Flow, 
    Text(String), 
    EditLine(String), 
    EditBox(String), 
    Button(String), 
    Checkbox(bool), 
    Border, 
    DockBottom, 
    ScrollArea { scroll_offset_y: u16, max_height: u16 }, 
}

// Struct representing the computed layout of a node
#[derive(Default, Debug, Clone, Copy)]
pub struct ComputedLayout {
    pub x: u16, 
    pub y: u16, 
    pub width: u16, 
    pub height: u16, 
}

// Struct representing a node in the virtual DOM
pub struct Node {
    pub id: NodeId, 
    pub node_type: NodeType, 
    pub children: Vec<NodeId>, 
    pub layout: ComputedLayout, 
    pub style: NodeStyle, 
}

// Struct representing the style of a node
#[derive(Debug, Clone, Copy)]
pub struct NodeStyle {
    pub fg: Color, 
    pub bg: Color, 
    pub modifier: Attribute, 
}

impl Default for NodeStyle {
    fn default() -> Self {
        NodeStyle { 
            fg: Color::Reset, 
            bg: Color::Reset, 
            modifier: Attribute::Reset, 
        }
    }
}

// Context for the Scarpe TUI application, managing the virtual DOM and rendering buffers
pub struct ScarpeTuiContext {
    pub use_alternate: bool, 
    pub nodes: Slab<Node>, 
    pub root_id: Option<NodeId>, 
    pub current_buffer: Buffer, 
    pub next_buffer: Buffer, 
    pub focused_node: Option<NodeId>, 
    pub needs_redraw: bool, 
    pub clicked_button: Option<NodeId>, 
    pub scroll_offset_y: u16, 
    pub event_receiver: Option<Receiver<Event>>, 
    pub cursor_pos: Option<(u16, u16)>,
}

// Represents a single cell in the rendering buffer
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Cell {
    pub ch: char, 
    pub fg: Color, 
    pub bg: Color, 
    pub modifier: Attribute, 
}

impl Default for Cell {
    fn default() -> Self {
        Cell { 
            ch: ' ', 
            fg: Color::Reset, 
            bg: Color::Reset, 
            modifier: Attribute::Reset, 
        }
    }
}

// Buffer for rendering the terminal UI
pub struct Buffer {
    pub width: u16, 
    pub height: u16, 
    pub content: Vec<Cell>, 
}

impl Buffer {
    // Creates a new buffer with the specified dimensions
    pub fn new(width: u16, height: u16) -> Self {
        let size = (width as usize) * (height as usize);
        Buffer {
            width,
            height,
            content: vec![Cell::default(); size], 
        }
    }

    // Resets the buffer by clearing all cells
    pub fn reset(&mut self) {
        for cell in self.content.iter_mut() {
            *cell = Cell::default(); 
        }
    }

    // Sets a character at a specific position in the buffer
    pub fn set_char(&mut self, x: u16, y: u16, ch: char, style: NodeStyle) {
        if x < self.width && y < self.height {
            let index = (y as usize) * (self.width as usize) + (x as usize);
            self.content[index].ch = ch; 
            self.content[index].fg = style.fg; 
            self.content[index].bg = style.bg; 
            self.content[index].modifier = style.modifier; 
        }
    }

    // Sets a character at a specific position in the buffer, with optional clipping
    pub fn set_char_clamped(&mut self, x: i32, y: i32, ch: char, style: NodeStyle, clip: Option<(i32, i32, i32, i32)>) {
        // Check if the position is within the clipping rectangle, if provided
        if let Some((cx1, cy1, cx2, cy2)) = clip {
            if x < cx1 || x >= cx2 || y < cy1 || y >= cy2 { return; }
        }
        
        // Check if the position is within the buffer bounds
        if x >= 0 && x < self.width as i32 && y >= 0 && y < self.height as i32 {
            self.set_char(x as u16, y as u16, ch, style);
        }
    }
}