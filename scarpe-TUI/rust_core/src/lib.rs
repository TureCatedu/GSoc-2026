mod context;
mod ffi;

use std::os::raw::c_int;

use crossterm::{style::Print, QueueableCommand};
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
    Root, // Represents the root node of the virtual DOM
    Stack, // Represents a stack layout node
    Flow, // Represents a flow layout node
    Text(String), // Represents a text node with a string value
    EditLine(String), // Represents an editable line with a string value
    Button(String), // Represents a button with a label
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
}

// Represents a single cell in the rendering buffer
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Cell {
    pub ch: char, // Character to display in the cell
}

impl Default for Cell {
    fn default() -> Self {
        Cell { ch: ' ' } // Default cell is empty (space character)
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
    pub fn set_char(&mut self, x: u16, y: u16, ch: char) {
        if x < self.width && y < self.height {
            let index = (y as usize) * (self.width as usize) + (x as usize);
            self.content[index].ch = ch; // Update the character at the specified position
        }
    }
}