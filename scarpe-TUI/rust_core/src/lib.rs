mod context;
mod ffi;

use std::os::raw::c_int;

use crossterm::style::{Print, Color, Attribute};
use crossterm::{
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use slab::Slab;

// Constants representing status codes for various outcomes
const STATUS_OK: c_int = 0; // Operation completed successfully
const STATUS_ERR_NULL_PTR: c_int = -1; // Null pointer error
const STATUS_ERR_PANIC: c_int = -2; // Panic occurred during execution
const STATUS_ERR_IO: c_int = -3; // Input/output error
const STATUS_ERR_INVALID_ID: c_int = -4; // Invalid node ID error
const STATUS_QUIT: c_int = 1; // Quit signal
const STATUS_CLICKED: c_int = 2; // Button clicked signal

pub type NodeId = usize; // Type alias for node identifiers

// Enum representing different types of nodes in the virtual DOM
#[derive(Debug, Clone)]
pub enum NodeType {
    Root, // Represents the root node of the virtual DOM
    Stack, // Represents a stack layout node
    Flow, // Represents a flow layout node
    Text(String), // Represents a text node with a string value
    EditLine(String), // Represents an editable line with a string value
    Button(String), // Represents a button with a label
    Checkbox(bool), // Represents a checkbox with a boolean state
}

// Struct representing the computed layout of a node
#[derive(Default, Debug, Clone, Copy)]
pub struct ComputedLayout {
    pub x: u16, // X-coordinate of the node
    pub y: u16, // Y-coordinate of the node
    pub width: u16, // Width of the node
    pub height: u16, // Height of the node
}

// Struct representing a node in the virtual DOM
pub struct Node {
    pub id: NodeId, // Unique identifier for the node
    pub node_type: NodeType, // Type of the node (e.g., Text, Button)
    pub children: Vec<NodeId>, // Child nodes of this node
    pub layout: ComputedLayout, // Layout information for this node
    pub style: NodeStyle, // Style information for this node
}

// Struct representing the style of a node
#[derive(Debug, Clone, Copy)]
pub struct NodeStyle {
    pub fg: Color, // Foreground color
    pub bg: Color, // Background color
    pub modifier: Attribute, // Text attributes (e.g., bold, italic)
}

impl Default for NodeStyle {
    fn default() -> Self {
        NodeStyle { 
            fg: Color::Reset, // Default foreground color
            bg: Color::Reset, // Default background color
            modifier: Attribute::Reset, // Default text attributes
        }
    }
}

// Context for the Scarpe TUI application, managing the virtual DOM and rendering buffers
pub struct ScarpeTuiContext {
    pub use_alternate: bool, // Whether to use the alternate screen buffer
    pub nodes: Slab<Node>, // Storage for nodes in the virtual DOM
    pub root_id: Option<NodeId>, // ID of the root node
    pub current_buffer: Buffer, // Current rendering buffer
    pub next_buffer: Buffer, // Next rendering buffer
    pub focused_node: Option<NodeId>, // ID of the currently focused node
    pub needs_redraw: bool, // Flag indicating whether a redraw is needed
    pub clicked_button: Option<NodeId>, // ID of the node that was clicked
    pub scroll_offset_y: u16, // Scroll offset for the UI
}

// Represents a single cell in the rendering buffer
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Cell {
    pub ch: char, // Character to display in the cell
    pub fg: Color, // Foreground color of the cell
    pub bg: Color, // Background color of the cell
    pub modifier: Attribute, // Text attributes for the cell
}

impl Default for Cell {
    fn default() -> Self {
        Cell { 
            ch: ' ', // Default character is a space
            fg: Color::Reset, // Default foreground color
            bg: Color::Reset, // Default background color
            modifier: Attribute::Reset, // Default text attributes
        }
    }
}

// Buffer for rendering the terminal UI
pub struct Buffer {
    pub width: u16, // Width of the buffer
    pub height: u16, // Height of the buffer
    pub content: Vec<Cell>, // Content of the buffer as a grid of cells
}

impl Buffer {
    // Creates a new buffer with the specified dimensions
    pub fn new(width: u16, height: u16) -> Self {
        let size = (width as usize) * (height as usize);
        Buffer {
            width,
            height,
            content: vec![Cell::default(); size], // Initialize all cells to default
        }
    }

    // Resets the buffer by clearing all cells
    pub fn reset(&mut self) {
        for cell in self.content.iter_mut() {
            *cell = Cell::default(); // Set each cell to its default value
        }
    }

    // Sets a character at a specific position in the buffer
    pub fn set_char(&mut self, x: u16, y: u16, ch: char, style: NodeStyle) {
        if x < self.width && y < self.height {
            let index = (y as usize) * (self.width as usize) + (x as usize);
            self.content[index].ch = ch; // Set the character
            self.content[index].fg = style.fg; // Set the foreground color
            self.content[index].bg = style.bg; // Set the background color
            self.content[index].modifier = style.modifier; // Set the text attributes
        }
    }

    // Sets a character at a specific position in the buffer, clamping the coordinates to the buffer's dimensions
    pub fn set_char_clamped(&mut self, x: i32, y: i32, ch: char, style: NodeStyle) {
        if x >= 0 && x < self.width as i32 && y >= 0 && y < self.height as i32 {
            self.set_char(x as u16, y as u16, ch, style);
        }
    }
}