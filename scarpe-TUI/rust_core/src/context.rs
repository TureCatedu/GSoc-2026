use crate::*;
use crossterm::QueueableCommand;
use crossterm::cursor::{MoveTo, Show, Hide};
use crossterm::event::{DisableMouseCapture, EnableMouseCapture};
use crossterm::style::{SetForegroundColor, SetBackgroundColor, SetAttribute, ResetColor, Color, Attribute};
use crossterm::terminal::{Clear, ClearType, size};
use slab::Slab;
use std::mem::swap;
use std::io::{stdout, Error, Write};

impl ScarpeTuiContext {
    // Initializes a new Scarpe TUI context with optional alternate screen buffer.
    pub fn new(use_alternate: bool) -> Result<Self, Error> {
        enable_raw_mode()?; // Enable raw mode for terminal input
        if use_alternate {
            stdout().execute(EnterAlternateScreen)?; // Enter the alternate screen buffer
        }
        stdout().execute(EnableMouseCapture)?; // Enable mouse capture
        let (width, height) = size().unwrap_or((80, 24));

        Ok(ScarpeTuiContext { 
            use_alternate,
            nodes: Slab::with_capacity(1024), // Preallocate space for graphical nodes
            root_id: None,
            current_buffer: Buffer::new(width, height),
            next_buffer: Buffer::new(width, height),
            focused_node: None,
            needs_redraw: true,
            clicked_button: None,
            scroll_offset_y: 0, // Initialize vertical scroll offset to zero
        })
    }

    // Renders the current state of the virtual DOM to the terminal using graphical diffing.
    pub fn render(&mut self) -> Result<(), Error> {
        if !self.needs_redraw {
            return Ok(());
        }

        let (term_width, term_height) = size().unwrap_or((80, 24));

        // Resize internal buffers if the physical terminal size has changed during execution.
        if self.next_buffer.width != term_width || self.next_buffer.height != term_height {
            self.next_buffer = Buffer::new(term_width, term_height);
            self.current_buffer = Buffer::new(term_width, term_height);
            stdout().execute(Clear(ClearType::All))?;
        }

        self.next_buffer.reset(); 
        self.compute_layouts(); // Recalculate absolute geometries of all nodes recursively.

        // Draw the tree starting from the official root node.
        if let Some(root_id) = self.root_id {
            self.draw_node(root_id);
        }

        let mut out = stdout();
        let mut current_fg = Color::Reset;
        let mut current_bg = Color::Reset;
        let mut current_attr = Attribute::Reset;

        for y in 0..term_height {
            for x in 0..term_width {
                let index = (y as usize) * (term_width as usize) + (x as usize);
                let next_cell = &self.next_buffer.content[index];
                let current_cell = &self.current_buffer.content[index];

                if next_cell != current_cell {
                    out.queue(MoveTo(x, y))?;
                    
                    // Apply modifiers first. Reset also clears physical colors on the terminal.
                    if next_cell.modifier != current_attr {
                        out.queue(SetAttribute(next_cell.modifier))?;
                        current_attr = next_cell.modifier;
                        
                        // Crossterm Attribute::Reset resets terminal colors, so we must reapply them.
                        if next_cell.modifier == Attribute::Reset {
                            current_fg = Color::Reset;
                            current_bg = Color::Reset;
                        }
                    }

                    // Apply colors after modifiers.
                    if next_cell.fg != current_fg {
                        out.queue(SetForegroundColor(next_cell.fg))?;
                        current_fg = next_cell.fg;
                    }
                    
                    if next_cell.bg != current_bg {
                        out.queue(SetBackgroundColor(next_cell.bg))?;
                        current_bg = next_cell.bg;
                    }

                    out.queue(Print(next_cell.ch))?;
                }
            }
        }
        out.queue(ResetColor)?; // Reset terminal colors at the end of rendering
        
        out.flush()?; // Flush the graphical command queue to Crossterm

        swap(&mut self.current_buffer, &mut self.next_buffer); // Swap buffers (Double-Buffering)

        self.needs_redraw = false; // Reset the redraw dirty flag
        Ok(())
    }

    // Draws a node and all its children recursively, applying the scroll offset.
    fn draw_node(&mut self, id: NodeId) {
        let (node_type, layout, children, style) = {
            if let Some(node) = self.nodes.get(id) {
                (node.node_type.clone(), node.layout, node.children.clone(), node.style)
            } else {
                return;
            }
        };

        // Convert the unsigned scroll offset to signed for safe arithmetic.
        let offset_y = self.scroll_offset_y as i32;

        // Render text widgets (Text/Para).
        if let NodeType::Text(ref text) = node_type {
            let mut cursor_x = layout.x as i32;
            let mut cursor_y = layout.y as i32 - offset_y; // Adjust for scroll offset.

            for ch in text.chars() {
                if cursor_x >= (layout.x + layout.width) as i32 {
                    cursor_x = layout.x as i32;
                    cursor_y += 1;
                }
                if cursor_y >= (layout.y + layout.height) as i32 - offset_y {
                    break;
                }  
                // Automatically clip coordinates outside the screen.
                self.next_buffer.set_char_clamped(cursor_x, cursor_y, ch, style);
                cursor_x += 1;
            }
        }

        // Render interactive input fields (EditLine).
        if let NodeType::EditLine(ref text) = node_type {
            let mut cursor_x = layout.x as i32;
            let cursor_y = layout.y as i32 - offset_y;
            let max_x = (layout.x + layout.width) as i32;

            self.next_buffer.set_char_clamped(cursor_x, cursor_y, '>', style); // EditLine prefix
            cursor_x += 2; 

            for ch in text.chars() {
                if cursor_x < max_x {
                    self.next_buffer.set_char_clamped(cursor_x, cursor_y, ch, style);
                    cursor_x += 1;
                }
            }

            // Synchronize the native blinking cursor of the OS.
            if Some(id) == self.focused_node {
                // Show the cursor only if the active EditLine row is visible on the screen.
                if cursor_y >= 0 && cursor_y < self.next_buffer.height as i32 {
                    let _ = stdout().execute(MoveTo(cursor_x as u16, cursor_y as u16));
                    let _ = stdout().execute(Show); 
                } else {
                    let _ = stdout().execute(Hide); // Hide if the element is scrolled out of the viewport.
                }
            }
        }

        // Render clickable buttons (Button).
        if let NodeType::Button(ref text) = node_type {
            let mut cursor_x = layout.x as i32;
            let cursor_y = layout.y as i32 - offset_y;
            
            self.next_buffer.set_char_clamped(cursor_x, cursor_y, '[', style); 
            self.next_buffer.set_char_clamped(cursor_x + 1, cursor_y, ' ', style);
            cursor_x += 2;

            for ch in text.chars() {
                if cursor_x < (layout.x + layout.width - 2) as i32 {
                    self.next_buffer.set_char_clamped(cursor_x, cursor_y, ch, style);
                    cursor_x += 1;
                }
            }

            if cursor_x < (layout.x + layout.width) as i32 {
                self.next_buffer.set_char_clamped(cursor_x, cursor_y, ' ', style);
                self.next_buffer.set_char_clamped(cursor_x + 1, cursor_y, ']', style);
            }
        }

        // Render checkboxes (Checkbox).
        if let NodeType::Checkbox(checked) = node_type {
            let cursor_x = layout.x as i32;
            let cursor_y = layout.y as i32 - offset_y;

            self.next_buffer.set_char_clamped(cursor_x, cursor_y, '[', style);
            let mark = if checked { 'X' } else { ' ' };
            self.next_buffer.set_char_clamped(cursor_x + 1, cursor_y, mark, style);
            self.next_buffer.set_char_clamped(cursor_x + 2, cursor_y, ']', style);
        }

        // Recursively draw child nodes of the interface.
        for child_id in children {
            self.draw_node(child_id);
        }
    }

    // Safely shuts down the TUI environment, restoring the user's terminal to its original state.
    pub fn shutdown(&mut self) -> Result<(), Error> {
        if self.use_alternate {
            stdout().execute(LeaveAlternateScreen)?; // Leave the alternate screen buffer
        }
        disable_raw_mode()?; // Disable raw mode
        stdout().execute(DisableMouseCapture)?; // Disable mouse capture
        Ok(())
    }

    // Starts the geometric computation of the interface starting from the Root node.
    pub fn compute_layouts(&mut self) {
        let (term_width, _term_height) = size().unwrap_or((80, 24));

        if let Some(root_id) = self.root_id {
            self.layout_node(root_id, 0, 0, term_width);
        }
    }

    // Resolves absolute positions and dimensions (Stack and Flow Layout Solvers).
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
                computed_width = max_width; // Stack nodes take up the full width.

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
            NodeType::Text(ref text) | NodeType::EditLine(ref text) | NodeType::Button(ref text) => {
                let mut text_len = text.chars().count() as u16;
                
                if matches!(node_type, NodeType::Button(_)) {
                    text_len += 4; // Account for button brackets.
                }

                if matches!(node_type, NodeType::EditLine(_)) {
                    text_len += 3; // Account for EditLine prefix.
                }

                computed_width = text_len.min(max_width);
                if computed_width == 0 { computed_width = 1; }
                computed_height = (text_len as f32 / computed_width as f32).ceil() as u16;
                
                if computed_height == 0 { computed_height = 1; }
            }
            NodeType::Checkbox(_) => {
                computed_width = 3; // Checkbox width is fixed.
                computed_height = 1; // Checkbox height is fixed.
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