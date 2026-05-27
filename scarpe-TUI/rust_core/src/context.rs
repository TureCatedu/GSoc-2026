use crate::*;
use crossterm::cursor::MoveTo;
use crossterm::terminal::{Clear, ClearType, size};
use slab::Slab;
use std::mem::swap;
use std::io::{stdout, Error, Write};
impl ScarpeTuiContext {
    // Initializes a new Scarpe TUI context
    pub fn new(use_alternate: bool) -> Result<Self, Error> {
        enable_raw_mode()?; // Enable raw mode for terminal input
        if use_alternate {
            stdout().execute(EnterAlternateScreen)?; // Enter alternate screen buffer
        }
        
        let (width, height) = size().unwrap_or((80, 24));

        Ok(ScarpeTuiContext { 
            use_alternate,
            nodes: Slab::with_capacity(1024), // Preallocate space for nodes
            root_id: None,
            current_buffer: Buffer::new(width, height),
            next_buffer: Buffer::new(width, height),
            focused_node: None,
            needs_redraw:  true,
        })
    }

    // Renders the current state of the virtual DOM to the terminal
    pub fn render(&mut self) -> Result<(), Error> {

        if !self.needs_redraw {
            return Ok(());
        }

        let (term_width, term_height) = size().unwrap_or((80, 24));

        // Resize buffers if terminal size has changed
        if self.next_buffer.width != term_width || self.next_buffer.height != term_height {
            self.next_buffer = Buffer::new(term_width, term_height);
            self.current_buffer = Buffer::new(term_width, term_height);
            stdout().execute(Clear(ClearType::All))?;
        }

        self.next_buffer.reset(); 
        self.compute_layouts(); // Compute layouts for all nodes

        // Draw the root node and its children
        if let Some(root_id) = self.root_id {
            self.draw_node(root_id);
        }

        let mut out = stdout();
        // Compare current and next buffers, and update only changed cells
        for y in 0..term_height {
            for x in 0..term_width {
                let index = (y as usize) * (term_width as usize) + (x as usize);
                let next_cell = &self.next_buffer.content[index];
                let current_cell = &self.current_buffer.content[index];

                if next_cell != current_cell {
                    out.queue(MoveTo(x, y))?;
                    out.queue(Print(next_cell.ch))?;
                }
            }
        }
        
        out.flush()?; // Flush all queued commands to the terminal

        swap(&mut self.current_buffer, &mut self.next_buffer); // Swap buffers

        self.needs_redraw = false; // Reset redraw flag
        Ok(())
    }

    // Draws a node and its children recursively
    fn draw_node(&mut self, id: NodeId) {
        let (node_type, layout, children) = {
            if let Some(node) = self.nodes.get(id) {
                (node.node_type.clone(), node.layout, node.children.clone())
            } else {
                return;
            }
        };

        // Render text nodes
        if let NodeType::Text(ref text) = node_type {
            let mut cursor_x = layout.x;
            let mut cursor_y = layout.y;

            for ch in text.chars() {
                if cursor_x >= layout.x + layout.width {
                    cursor_x = layout.x;
                    cursor_y += 1;
                }
                if cursor_y >= layout.y + layout.height {
                    break;
                }
                self.next_buffer.set_char(cursor_x, cursor_y, ch);
                cursor_x += 1;
            }
        }

        if let NodeType::EditLine(text) = node_type {
            let mut cursor_x = layout.x;
            let cursor_y = layout.y;
            
            // Disegniamo un prefisso per far capire che è un input
            self.next_buffer.set_char(cursor_x, cursor_y, '>');
            cursor_x += 2; // Spazio dopo il >

            for ch in text.chars() {
                if cursor_x < layout.x + layout.width {
                    self.next_buffer.set_char(cursor_x, cursor_y, ch);
                    cursor_x += 1;
                }
            }
            // Disegniamo un cursore lampeggiante finto (un blocco o underscore)
            if cursor_x < layout.x + layout.width {
                self.next_buffer.set_char(cursor_x, cursor_y, '_');
            }
        }

        // Recursively draw child nodes
        for child_id in children {
            self.draw_node(child_id);
        }
    }

    // Shuts down the TUI context, restoring the terminal to its original state
    pub fn shutdown(&mut self) -> Result<(), Error> {
        if self.use_alternate {
            stdout().execute(LeaveAlternateScreen)?; // Leave alternate screen buffer
        }
        disable_raw_mode()?; // Disable raw mode
        Ok(())
    }

    // Computes layouts for all nodes in the virtual DOM
    pub fn compute_layouts(&mut self) {
        let (term_width, _term_height) = size().unwrap_or((80, 24));

        if let Some(root_id) = self.root_id {
            self.layout_node(root_id, 0, 0, term_width);
        }
    }

    // Computes the layout for a specific node and its children
    fn layout_node(
        &mut self,
        id: NodeId,
        start_x: u16,
        start_y: u16,
        max_width: u16,
    ) -> ComputedLayout {
        let (node_type, children) = {
            if let Some(node) = self.nodes.get(id) {
                (node.node_type.clone(), node.children.clone())
            } else {
                return ComputedLayout::default();
            }
        };

        let mut current_x = start_x;
        let mut current_y = start_y;
        let mut computed_width = 0;
        let mut computed_height = 0;

        match node_type {
            NodeType::Root | NodeType::Stack => {
                computed_width = max_width; // Stack nodes take up the full width

                for child_id in children {
                    let child_layout = self.layout_node(child_id, current_x, current_y, max_width);
                    current_y += child_layout.height;
                    computed_height += child_layout.height;
                }
            }
            NodeType::Flow => {
                let mut row_height = 0;

                for child_id in children {
                    let available_width = max_width.saturating_sub(current_x - start_x);
                    let mut child_layout =
                        self.layout_node(child_id, current_x, current_y, available_width);

                    if current_x + child_layout.width > start_x + max_width {
                        current_x = start_x; 
                        current_y += row_height; 
                        computed_height += row_height;
                        row_height = 0;

                        child_layout = self.layout_node(child_id, current_x, current_y, max_width);
                    }

                    current_x += child_layout.width;
                    row_height = row_height.max(child_layout.height);
                    computed_width = computed_width.max(current_x - start_x);
                }
                computed_height += row_height;
            }
            NodeType::Text(text) | NodeType::EditLine(text) => {
                let text_len = text.chars().count() as u16; 
                computed_width = text_len.min(max_width);
                if computed_width == 0 {
                    computed_width = 1;
                } 
                computed_height = (text_len as f32 / computed_width as f32).ceil() as u16;
            }
        }

        let final_layout = ComputedLayout {
            x: start_x,
            y: start_y,
            width: computed_width,
            height: computed_height,
        };

        if let Some(node) = self.nodes.get_mut(id) {
            node.layout = final_layout;
        }

        final_layout
    }
}