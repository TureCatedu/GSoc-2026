use crate::*;
use crossterm::cursor::{Hide, MoveTo, Show};
use crossterm::event::{DisableMouseCapture, EnableMouseCapture};
use crossterm::style::{
    Attribute, Color, ResetColor, SetAttribute, SetBackgroundColor, SetForegroundColor,
};
use crossterm::terminal::{size, Clear, ClearType};
use crossterm::QueueableCommand;
use slab::Slab;
use std::io::{stdout, Error, Write};
use std::mem::swap;
use std::sync::mpsc;
use std::thread;

impl ScarpeTuiContext {
    // Initializes a new TUI context. Sets up the terminal in raw mode, clears the screen,
    // and starts a thread to listen for terminal events. Returns the initialized context.
    pub fn new(use_alternate: bool) -> Result<Self, Error> {
        enable_raw_mode()?; // Enables raw mode for terminal input
        if use_alternate {
            stdout().execute(EnterAlternateScreen)?; // Switches to the alternate screen buffer
        }
        stdout().execute(EnableMouseCapture)?; // Enables mouse input capture
        stdout().execute(Clear(ClearType::All))?; // Clears the terminal screen

        // Get the terminal dimensions (width and height)
        let (width, height) = size().unwrap_or((80, 24));

        // Create a channel for sending and receiving terminal events
        let (tx, rx) = mpsc::channel();
        thread::spawn(move || {
            // Continuously read terminal events and send them through the channel
            loop {
                if let Ok(event) = crossterm::event::read() {
                    if tx.send(event).is_err() {
                        break;
                    }
                } else {
                    break;
                }
            }
        });

        // Initialize the TUI context with default values
        Ok(ScarpeTuiContext {
            use_alternate,
            nodes: Slab::with_capacity(1024), // Preallocate space for nodes
            root_id: None,
            current_buffer: Buffer::new(width, height), // Buffer for the current frame
            next_buffer: Buffer::new(width, height),    // Buffer for the next frame
            focused_node: None,                         // No node is focused initially
            needs_redraw: true,                         // Mark the screen as needing a redraw
            clicked_button: None,                       // No button is clicked initially
            scroll_offset_y: 0,                         // No vertical scrolling initially
            event_receiver: Some(rx),                   // Receiver for terminal events
        })
    }

    // Shuts down the TUI context. Restores the terminal to its original state by disabling raw mode,
    // leaving the alternate screen buffer (if used), and disabling mouse capture.
    pub fn shutdown(&mut self) -> Result<(), Error> {
        if self.use_alternate {
            stdout().execute(LeaveAlternateScreen)?; // Switches back to the main screen buffer
        }
        disable_raw_mode()?; // Disables raw mode for terminal input
        stdout().execute(DisableMouseCapture)?; // Disables mouse input capture
        Ok(())
    }

    // Renders the current state of the virtual DOM to the terminal. Compares the current and next buffers
    // to minimize the number of terminal updates. Handles resizing and redraws the entire screen if needed.
    pub fn render(&mut self) -> Result<(), Error> {
        if !self.needs_redraw {
            return Ok(()); // Skip rendering if no redraw is needed
        }

        let (term_width, term_height) = size().unwrap_or((80, 24)); // Get terminal dimensions

        // Resize buffers if the terminal size has changed
        if self.next_buffer.width != term_width || self.next_buffer.height != term_height {
            self.next_buffer = Buffer::new(term_width, term_height);
            self.current_buffer = Buffer::new(term_width, term_height);
            stdout().execute(Clear(ClearType::All))?; // Clear the screen
        }

        self.next_buffer.reset(); // Clear the next buffer
        self.compute_layouts(); // Compute the layout of all nodes

        // Draw the root node and its children
        if let Some(root_id) = self.root_id {
            self.draw_node(root_id);
        }

        let mut out = stdout(); // Get the terminal output handle
        let mut current_fg = Color::Reset; // Track the current foreground color
        let mut current_bg = Color::Reset; // Track the current background color
        let mut current_attr = Attribute::Reset; // Track the current text attributes

        // Iterate through each cell in the terminal and update only the changed cells
        for y in 0..term_height {
            for x in 0..term_width {
                let index = (y as usize) * (term_width as usize) + (x as usize);
                let next_cell = &self.next_buffer.content[index];
                let current_cell = &self.current_buffer.content[index];

                if next_cell != current_cell {
                    out.queue(MoveTo(x, y))?; // Move the cursor to the cell's position

                    // Update text attributes if they have changed
                    if next_cell.modifier != current_attr {
                        out.queue(SetAttribute(next_cell.modifier))?;
                        current_attr = next_cell.modifier;

                        // Reset colors if attributes are reset
                        if next_cell.modifier == Attribute::Reset {
                            current_fg = Color::Reset;
                            current_bg = Color::Reset;
                        }
                    }

                    // Update foreground color if it has changed
                    if next_cell.fg != current_fg {
                        out.queue(SetForegroundColor(next_cell.fg))?;
                        current_fg = next_cell.fg;
                    }

                    // Update background color if it has changed
                    if next_cell.bg != current_bg {
                        out.queue(SetBackgroundColor(next_cell.bg))?;
                        current_bg = next_cell.bg;
                    }

                    out.queue(Print(next_cell.ch))?; // Print the character in the cell
                }
            }
        }

        out.queue(ResetColor)?; // Reset terminal colors
        out.flush()?; // Flush all queued terminal commands

        swap(&mut self.current_buffer, &mut self.next_buffer); // Swap the buffers
        self.needs_redraw = false; // Mark the screen as not needing a redraw
        Ok(())
    }

    // Computes the layout of all nodes in the virtual DOM. Starts from the root node and recursively
    // calculates the position and size of each node.
    pub fn compute_layouts(&mut self) {
        let (term_width, _) = size().unwrap_or((80, 24)); // Get terminal width
        if let Some(root_id) = self.root_id {
            self.layout_node(root_id, 0, 0, term_width); // Compute layout starting from the root node
        }
    }

    // Computes the layout for a specific node and its children. Determines the position and size of the node
    // based on its type and the available space.
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
                return ComputedLayout::default(); // Return default layout if the node does not exist
            }
        };

        // Compute the layout based on the node type
        let (computed_width, computed_height) = match node_type {
            NodeType::Root | NodeType::Stack => {
                self.layout_stack(children, start_x, start_y, max_width)
            }
            NodeType::Flow => self.layout_flow(children, start_x, start_y, max_width),
            NodeType::Border => self.layout_border(children, start_x, start_y, max_width),
            NodeType::Checkbox(_) => (3, 1), // Fixed size for checkboxes
            NodeType::Text(ref text)
            | NodeType::EditLine(ref text)
            | NodeType::Button(ref text) => self.layout_simple_text(&node_type, text, max_width),
            NodeType::EditBox(ref text) => self.layout_edit_box(text, max_width),
        };

        // Update the node's layout in the virtual DOM
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

    // Layout computation for stack nodes. Arranges children vertically.
    fn layout_stack(
        &mut self,
        children: Vec<NodeId>,
        start_x: u16,
        start_y: u16,
        max_width: u16,
    ) -> (u16, u16) {
        let mut current_y = start_y;
        let mut computed_height = 0;
        for child_id in children {
            let child_layout = self.layout_node(child_id, start_x, current_y, max_width);
            current_y += child_layout.height; // Move to the next row
            computed_height += child_layout.height;
        }
        (max_width, computed_height)
    }

    // Layout computation for flow nodes. Arranges children horizontally, wrapping to the next row if needed.
    fn layout_flow(
        &mut self,
        children: Vec<NodeId>,
        start_x: u16,
        start_y: u16,
        max_width: u16,
    ) -> (u16, u16) {
        let mut current_x = start_x;
        let mut current_y = start_y;
        let mut computed_width = 0;
        let mut computed_height = 0;
        let mut row_height = 0;

        for child_id in children {
            let available_width = max_width.saturating_sub(current_x - start_x);
            let mut child_layout =
                self.layout_node(child_id, current_x, current_y, available_width);

            // Wrap to the next row if the child does not fit
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
        (computed_width, computed_height)
    }

    // Layout computation for bordered nodes. Adds padding around the children.
    fn layout_border(
        &mut self,
        children: Vec<NodeId>,
        start_x: u16,
        start_y: u16,
        max_width: u16,
    ) -> (u16, u16) {
        let child_start_x = start_x + 1; // Add padding on the left
        let mut child_start_y = start_y + 1; // Add padding on the top
        let available_child_width = max_width.saturating_sub(2); // Subtract padding from the width

        let mut max_child_width = 0;
        let mut total_child_height = 0;

        for child_id in children {
            let child_layout = self.layout_node(
                child_id,
                child_start_x,
                child_start_y,
                available_child_width,
            );
            child_start_y += child_layout.height;
            total_child_height += child_layout.height;
            max_child_width = max_child_width.max(child_layout.width);
        }
        (max_child_width + 2, total_child_height + 2) // Add padding to the computed dimensions
    }

    // Layout computation for simple text nodes, buttons, and edit lines.
    fn layout_simple_text(&self, node_type: &NodeType, text: &str, max_width: u16) -> (u16, u16) {
        let mut total_height = 1;
        let mut current_line_width = 0;
        let mut max_w = 0;

        // Iterates through each character in the text to calculate the width and height of the node.
        for ch in text.chars() {
            if ch == '\n' {
                total_height += 1;
                max_w = max_w.max(current_line_width);
                current_line_width = 0;
            } else {
                current_line_width += 1;
                if current_line_width >= max_width {
                    total_height += 1;
                    max_w = max_w.max(current_line_width);
                    current_line_width = 0;
                }
            }
        }
        
        if matches!(node_type, NodeType::Button(_)) { current_line_width += 4; }
        if matches!(node_type, NodeType::EditLine(_)) { current_line_width += 3; }
        
        max_w = max_w.max(current_line_width);
        
        let mut computed_width = max_w.min(max_width);
        if computed_width == 0 { computed_width = 1; }
        
        (computed_width, total_height)
    }

    // Layout computation for multi-line editable text boxes.
    fn layout_edit_box(&self, text: &str, max_width: u16) -> (u16, u16) {
        let mut total_height = 1;
        let mut current_line_width = 0;
        let mut max_w = 0;

        for ch in text.chars() {
            if ch == '\n' {
                total_height += 1;
                max_w = max_w.max(current_line_width);
                current_line_width = 0;
            } else {
                current_line_width += 1;
                if current_line_width >= max_width {
                    total_height += 1;
                    max_w = max_w.max(current_line_width);
                    current_line_width = 0;
                }
            }
        }
        max_w = max_w.max(current_line_width);
        (max_w.max(10).min(max_width), total_height.max(3)) // Ensure minimum dimensions
    }

    fn draw_node(&mut self, id: NodeId) {
        // Retrieves the node's type, layout, children, and style from the virtual DOM.
        // If the node does not exist, the function exits early.
        let (node_type, layout, children, style) = {
            if let Some(node) = self.nodes.get(id) {
                (
                    node.node_type.clone(),
                    node.layout,
                    node.children.clone(),
                    node.style,
                )
            } else {
                return;
            }
        };

        // Adjusts the vertical position of the node based on the current scroll offset.
        let offset_y = self.scroll_offset_y as i32;

        // Matches the node type and calls the appropriate drawing function.
        // Each node type has a specific rendering logic.
        match node_type {
            NodeType::Text(ref text) => self.draw_text(text, &layout, offset_y, style),
            NodeType::EditLine(ref text) => self.draw_edit_line(id, text, &layout, offset_y, style),
            NodeType::Button(ref text) => self.draw_button(text, &layout, offset_y, style),
            NodeType::Checkbox(checked) => self.draw_checkbox(checked, &layout, offset_y, style),
            NodeType::Border => self.draw_border(&layout, offset_y, style),
            NodeType::EditBox(ref text) => self.draw_edit_box(id, text, &layout, offset_y, style),
            _ => {}
        }

        // Recursively draws all child nodes of the current node.
        for child_id in children {
            self.draw_node(child_id);
        }
    }

    // Draws a text node. Handles line breaks and automatic wrapping based on the layout's width.
    fn draw_text(&mut self, text: &str, layout: &ComputedLayout, offset_y: i32, style: NodeStyle) {
        let mut cursor_x = layout.x as i32;
        let mut cursor_y = layout.y as i32 - offset_y;
        let max_x = (layout.x + layout.width) as i32;

        for ch in text.chars() {
            if ch == '\n' {
                cursor_x = layout.x as i32;
                cursor_y += 1;
            } else {
                if cursor_x < max_x {
                    self.next_buffer.set_char_clamped(cursor_x, cursor_y, ch, style);
                    cursor_x += 1;
                } else {
                    // Wrapping automatico
                    cursor_x = layout.x as i32;
                    cursor_y += 1;
                    self.next_buffer.set_char_clamped(cursor_x, cursor_y, ch, style);
                    cursor_x += 1;
                }
            }

            if cursor_y >= (layout.y + layout.height) as i32 - offset_y { break; }
        }
    }

    fn draw_edit_line(
        &mut self,
        id: NodeId,
        text: &str,
        layout: &ComputedLayout,
        offset_y: i32,
        style: NodeStyle,
    ) {
        // Draws an editable single-line text field.
        // Adds a prefix ('>') to indicate the field is editable and handles cursor positioning.
        let mut cursor_x = layout.x as i32;
        let cursor_y = layout.y as i32 - offset_y;
        let max_x = (layout.x + layout.width) as i32;

        self.next_buffer
            .set_char_clamped(cursor_x, cursor_y, '>', style);
        cursor_x += 2;

        for ch in text.chars() {
            if cursor_x < max_x {
                self.next_buffer
                    .set_char_clamped(cursor_x, cursor_y, ch, style);
                cursor_x += 1;
            }
        }

        // If the node is focused, positions the terminal cursor at the end of the text.
        if Some(id) == self.focused_node {
            if cursor_y >= 0 && cursor_y < self.next_buffer.height as i32 {
                let _ = stdout().execute(MoveTo(cursor_x as u16, cursor_y as u16));
                let _ = stdout().execute(Show);
            } else {
                let _ = stdout().execute(Hide);
            }
        }
    }

    fn draw_button(
        &mut self,
        text: &str,
        layout: &ComputedLayout,
        offset_y: i32,
        style: NodeStyle,
    ) {
        // Draws a button with a label enclosed in square brackets.
        // Ensures the button fits within the specified layout dimensions.
        let mut cursor_x = layout.x as i32;
        let cursor_y = layout.y as i32 - offset_y;

        self.next_buffer
            .set_char_clamped(cursor_x, cursor_y, '[', style);
        self.next_buffer
            .set_char_clamped(cursor_x + 1, cursor_y, ' ', style);
        cursor_x += 2;

        for ch in text.chars() {
            if cursor_x < (layout.x + layout.width - 2) as i32 {
                self.next_buffer
                    .set_char_clamped(cursor_x, cursor_y, ch, style);
                cursor_x += 1;
            }
        }

        if cursor_x < (layout.x + layout.width) as i32 {
            self.next_buffer
                .set_char_clamped(cursor_x, cursor_y, ' ', style);
            self.next_buffer
                .set_char_clamped(cursor_x + 1, cursor_y, ']', style);
        }
    }

    fn draw_checkbox(
        &mut self,
        checked: bool,
        layout: &ComputedLayout,
        offset_y: i32,
        style: NodeStyle,
    ) {
        // Draws a checkbox with a checked or unchecked state.
        // The checkbox is represented as '[X]' for checked and '[ ]' for unchecked.
        let cursor_x = layout.x as i32;
        let cursor_y = layout.y as i32 - offset_y;

        self.next_buffer
            .set_char_clamped(cursor_x, cursor_y, '[', style);
        let mark = if checked { 'X' } else { ' ' };
        self.next_buffer
            .set_char_clamped(cursor_x + 1, cursor_y, mark, style);
        self.next_buffer
            .set_char_clamped(cursor_x + 2, cursor_y, ']', style);
    }

    fn draw_border(&mut self, layout: &ComputedLayout, offset_y: i32, style: NodeStyle) {
        // Draws a border around the node using box-drawing characters.
        // The border includes corners, horizontal lines, and vertical lines.
        let x = layout.x as i32;
        let y = layout.y as i32 - offset_y;
        let w = layout.width as i32;
        let h = layout.height as i32;

        if w > 1 && h > 1 {
            self.next_buffer.set_char_clamped(x, y, '┌', style);
            for i in 1..(w - 1) {
                self.next_buffer.set_char_clamped(x + i, y, '─', style);
            }
            self.next_buffer.set_char_clamped(x + w - 1, y, '┐', style);

            for j in 1..(h - 1) {
                self.next_buffer.set_char_clamped(x, y + j, '│', style);
                self.next_buffer
                    .set_char_clamped(x + w - 1, y + j, '│', style);
            }

            self.next_buffer.set_char_clamped(x, y + h - 1, '└', style);
            for i in 1..(w - 1) {
                self.next_buffer
                    .set_char_clamped(x + i, y + h - 1, '─', style);
            }
            self.next_buffer
                .set_char_clamped(x + w - 1, y + h - 1, '┘', style);
        }
    }

    fn draw_edit_box(
        &mut self,
        id: NodeId,
        text: &str,
        layout: &ComputedLayout,
        offset_y: i32,
        style: NodeStyle,
    ) {
        // Draws a multi-line editable text box.
        // Handles line wrapping and positions the cursor at the end of the text if the node is focused.
        let mut cursor_x = layout.x as i32;
        let mut cursor_y = layout.y as i32 - offset_y;
        let max_x = (layout.x + layout.width) as i32;

        for ch in text.chars() {
            if ch == '\n' {
                cursor_x = layout.x as i32;
                cursor_y += 1;
            } else {
                if cursor_x < max_x {
                    self.next_buffer
                        .set_char_clamped(cursor_x, cursor_y, ch, style);
                    cursor_x += 1;
                } else {
                    cursor_x = layout.x as i32;
                    cursor_y += 1;
                    self.next_buffer
                        .set_char_clamped(cursor_x, cursor_y, ch, style);
                    cursor_x += 1;
                }
            }
        }

        // If the node is focused, positions the terminal cursor at the end of the text.
        if Some(id) == self.focused_node {
            if cursor_y >= 0 && cursor_y < self.next_buffer.height as i32 {
                let _ = stdout().execute(MoveTo(cursor_x as u16, cursor_y as u16));
                let _ = stdout().execute(Show);
            } else {
                let _ = stdout().execute(Hide);
            }
        }
    }
}
