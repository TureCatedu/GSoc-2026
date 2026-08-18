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
    /// Initializes a new TUI context.
    /// Sets up the terminal in raw mode, clears the screen, and starts a thread to listen for terminal events.
    pub fn new(use_alternate: bool) -> Result<Self, Error> {
        enable_raw_mode()?; // Enable raw mode for terminal input
        if use_alternate {
            stdout().execute(EnterAlternateScreen)?; // Switch to the alternate screen buffer
        }
        stdout().execute(EnableMouseCapture)?; // Enable mouse input capture
        stdout().execute(Clear(ClearType::All))?; // Clear the terminal screen

        let (width, height) = size().unwrap_or((80, 24)); // Get terminal dimensions
        let (tx, rx) = mpsc::channel(); // Create a channel for terminal events

        // Spawn a thread to continuously read terminal events and send them through the channel
        thread::spawn(move || loop {
            if let Ok(event) = crossterm::event::read() {
                if tx.send(event).is_err() {
                    break;
                }
            } else {
                break;
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
            cursor_pos: None,                           // Initialize cursor position
            initial_scroll_done: false,                 // First layout opens at the bottom
        })
    }

    /// Shuts down the TUI context.
    /// Restores the terminal to its original state by disabling raw mode, leaving the alternate screen buffer, and disabling mouse capture.
    pub fn shutdown(&mut self) -> Result<(), Error> {
        if self.use_alternate {
            stdout().execute(LeaveAlternateScreen)?; // Switch back to the main screen buffer
        }
        disable_raw_mode()?; // Disable raw mode for terminal input
        stdout().execute(DisableMouseCapture)?; // Disable mouse input capture
        Ok(())
    }

    /// Renders the current state of the virtual DOM to the terminal.
    /// Compares the current and next buffers to minimize the number of terminal updates.
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
        self.initial_scroll_done = true;

        // Draw the root node and its children
        if let Some(root_id) = self.root_id {
            self.start_drawing(root_id);
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

        // Position the blinking cursor after rendering
        if let Some((cx, cy)) = self.cursor_pos {
            out.queue(MoveTo(cx, cy))?;
            out.queue(Show)?;
        } else {
            out.queue(Hide)?;
        }

        out.flush()?; // Flush all queued terminal commands

        swap(&mut self.current_buffer, &mut self.next_buffer); // Swap the buffers
        self.needs_redraw = false; // Mark the screen as not needing a redraw
        Ok(())
    }

    /// Computes the layout of all nodes in the virtual DOM.
    /// Starts from the root node and recursively calculates the position and size of each node.
    pub fn compute_layouts(&mut self) {
        let (term_width, _) = size().unwrap_or((80, 24)); // Get terminal width
        if let Some(root_id) = self.root_id {
            self.layout_node(root_id, 0, 0, term_width); // Compute layout starting from the root node
        }
    }

    // Recursively computes the layout for a node and its children based on the node type.
    // Returns the computed width and height of the node.
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

        let (computed_width, computed_height) = match node_type {
            NodeType::Root | NodeType::Stack => {
                self.layout_stack(children, start_x, start_y, max_width)
            }
            NodeType::Flow => self.layout_flow(children, start_x, start_y, max_width),
            NodeType::Border => self.layout_border(children, start_x, start_y, max_width),
            NodeType::Checkbox(_) => (3, 1),
            NodeType::Text(ref text)
            | NodeType::EditLine(ref text)
            | NodeType::Button(ref text) => self.layout_simple_text(&node_type, text, max_width),
            NodeType::EditBox(ref text) => self.layout_edit_box(text, max_width),
            NodeType::DockBottom => self.layout_dock_bottom(children, start_x, max_width),
            NodeType::ScrollArea {
                scroll_offset_y: _,
                max_height,
            } => {
                let mut actual_height = max_height;
                if actual_height == 0 {
                    let (_, term_height) = crossterm::terminal::size().unwrap_or((80, 24));

                    let mut dock_h = 6;
                    let mut dock_id_opt = None;

                    if let Some(root_id) = self.root_id {
                        if let Some(root) = self.nodes.get(root_id) {
                            for &cid in &root.children {
                                if let Some(child) = self.nodes.get(cid) {
                                    if matches!(child.node_type, NodeType::DockBottom) {
                                        dock_id_opt = Some(cid);
                                        break;
                                    }
                                }
                            }
                        }
                    }

                    if let Some(dock_id) = dock_id_opt {
                        let dl = self.layout_node(dock_id, start_x, 0, max_width);
                        dock_h = dl.height;
                    }

                    actual_height = term_height
                        .saturating_sub(start_y)
                        .saturating_sub(dock_h)
                        .saturating_sub(2);
                }
                self.layout_scroll_area(id, children, start_x, start_y, max_width, actual_height)
            }
        };

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

    /// Computes the layout for stack nodes, arranging children vertically.
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

    /// Computes the layout for flow nodes, arranging children horizontally and wrapping to the next row if needed.
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

            if current_x + child_layout.width > start_x + max_width {
                current_x = start_x; // Wrap to the next row
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

    /// Computes the layout for bordered nodes, adding padding around the children.
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
    fn layout_simple_text(&self, node_type: &NodeType, text: &str, max_width: u16) -> (u16, u16) {
        // Calculates the layout for simple text nodes, buttons, and edit lines.
        // Handles line wrapping and ensures the text fits within the specified maximum width.
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

        // Adjusts the width for buttons and edit lines to account for additional characters.
        if matches!(node_type, NodeType::Button(_)) {
            current_line_width += 4;
        }
        if matches!(node_type, NodeType::EditLine(_)) {
            current_line_width += 3;
        }

        max_w = max_w.max(current_line_width);
        let mut computed_width = max_w.min(max_width);
        if computed_width == 0 {
            computed_width = 1;
        }
        (computed_width, total_height)
    }

    fn layout_edit_box(&self, text: &str, max_width: u16) -> (u16, u16) {
        // Calculates the layout for multi-line editable text boxes.
        // Handles line wrapping and ensures the text fits within the specified maximum width.
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
        (max_w.max(10).min(max_width), total_height.max(3)) // Ensures minimum dimensions for the edit box.
    }

    fn layout_dock_bottom(
        &mut self,
        children: Vec<NodeId>,
        start_x: u16,
        max_width: u16,
    ) -> (u16, u16) {
        // Calculates the layout for nodes docked at the bottom of the screen.
        // Positions the children nodes starting from the bottom of the terminal.
        let (_, term_height) = crossterm::terminal::size().unwrap_or((80, 24));
        let mut temp_y = 0;
        let mut comp_h = 0;

        for &child_id in &children {
            let child_layout = self.layout_node(child_id, start_x, temp_y, max_width);
            temp_y += child_layout.height;
            comp_h += child_layout.height;
        }

        let actual_y = term_height.saturating_sub(comp_h);
        self.layout_stack(children, start_x, actual_y, max_width);

        (max_width, comp_h)
    }

    // Calculates the layout for scrollable areas, determining the maximum scroll offset based on the content height and the specified maximum height.
    fn layout_scroll_area(
        &mut self,
        id: NodeId,
        children: Vec<NodeId>,
        start_x: u16,
        start_y: u16,
        max_width: u16,
        max_height: u16,
    ) -> (u16, u16) {
        // Preserve the user's position, but follow the bottom when they were already there.
        let old_content_height = children
            .iter()
            .filter_map(|child| self.nodes.get(*child))
            .map(|child| (child.layout.y + child.layout.height).saturating_sub(start_y))
            .max()
            .unwrap_or(0);
        let old_max_scroll = old_content_height.saturating_sub(max_height);
        let (old_offset, was_initial) = self
            .nodes
            .get(id)
            .and_then(|node| match node.node_type {
                NodeType::ScrollArea {
                    scroll_offset_y, ..
                } => Some((scroll_offset_y, self.initial_scroll_done == false)),
                _ => None,
            })
            .unwrap_or((0, false));
        let was_at_bottom = old_offset >= old_max_scroll;

        let (_, computed_height) = self.layout_stack(children, start_x, start_y, max_width);
        let max_scroll = computed_height.saturating_sub(max_height);

        if let Some(node) = self.nodes.get_mut(id) {
            if let NodeType::ScrollArea {
                ref mut scroll_offset_y,
                ..
            } = node.node_type
            {
                if was_initial || was_at_bottom {
                    *scroll_offset_y = max_scroll;
                } else {
                    *scroll_offset_y = (*scroll_offset_y).min(max_scroll);
                }
            }
        }

        (max_width, max_height)
    }

    /// Scroll every scroll area by a bounded amount.
    pub fn scroll_by(&mut self, delta: i32) -> bool {
        let mut changed = false;
        let ids: Vec<NodeId> = self
            .nodes
            .iter()
            .filter_map(|(id, node)| {
                matches!(node.node_type, NodeType::ScrollArea { .. }).then_some(id)
            })
            .collect();

        for id in ids {
            let (layout, children, offset) = match self.nodes.get(id) {
                Some(node) => match node.node_type {
                    NodeType::ScrollArea {
                        scroll_offset_y, ..
                    } => (node.layout, node.children.clone(), scroll_offset_y),
                    _ => continue,
                },
                None => continue,
            };
            let content_height = children
                .iter()
                .filter_map(|child| self.nodes.get(*child))
                .map(|child| (child.layout.y + child.layout.height).saturating_sub(layout.y))
                .max()
                .unwrap_or(0);
            let max_scroll = content_height.saturating_sub(layout.height);
            let next = if delta < 0 {
                offset.saturating_sub((-delta) as u16)
            } else {
                offset.saturating_add(delta as u16).min(max_scroll)
            };
            if next != offset {
                if let Some(node) = self.nodes.get_mut(id) {
                    if let NodeType::ScrollArea {
                        ref mut scroll_offset_y,
                        ..
                    } = node.node_type
                    {
                        *scroll_offset_y = next;
                    }
                }
                changed = true;
            }
        }
        changed
    }

    /// Move every scroll area to its beginning or end.
    pub fn scroll_to(&mut self, end: bool) -> bool {
        let mut changed = false;
        let ids: Vec<NodeId> = self
            .nodes
            .iter()
            .filter_map(|(id, node)| {
                matches!(node.node_type, NodeType::ScrollArea { .. }).then_some(id)
            })
            .collect();

        for id in ids {
            let (layout, children, offset) = match self.nodes.get(id) {
                Some(node) => match node.node_type {
                    NodeType::ScrollArea {
                        scroll_offset_y, ..
                    } => (node.layout, node.children.clone(), scroll_offset_y),
                    _ => continue,
                },
                None => continue,
            };
            let content_height = children
                .iter()
                .filter_map(|child| self.nodes.get(*child))
                .map(|child| (child.layout.y + child.layout.height).saturating_sub(layout.y))
                .max()
                .unwrap_or(0);
            let max_scroll = content_height.saturating_sub(layout.height);
            let next = if end { max_scroll } else { 0 };
            if next != offset {
                if let Some(node) = self.nodes.get_mut(id) {
                    if let NodeType::ScrollArea {
                        ref mut scroll_offset_y,
                        ..
                    } = node.node_type
                    {
                        *scroll_offset_y = next;
                    }
                }
                changed = true;
            }
        }
        changed
    }

    pub fn handle_mouse(
        &mut self,
        mx: u16,
        my: u16,
        click: bool,
        scroll_dir: i32,
    ) -> (bool, Option<NodeId>) {
        // Handles mouse events, including clicks and scroll actions.
        // Updates the state of the application based on the mouse position and actions.
        let mut state_changed = false;
        let mut clicked_btn = None;

        if let Some(root_id) = self.root_id {
            let (sc, cb, focus) = self.traverse_mouse(
                root_id,
                mx as i32,
                my as i32,
                click,
                scroll_dir,
                None,
                self.scroll_offset_y as i32,
            );

            if sc {
                state_changed = true;
            }
            if cb.is_some() {
                clicked_btn = cb;
            }
            if focus.is_some() {
                self.focused_node = focus;
                state_changed = true;
            }
        }

        // Handles scrolling if no other state changes occurred.
        if scroll_dir != 0 && !state_changed {
            let mut max_content_height = 0;
            if let Some(root_id) = self.root_id {
                if let Some(root) = self.nodes.get(root_id) {
                    max_content_height = root.layout.height;
                }
            }
            let term_height = self.next_buffer.height;
            let max_scroll = max_content_height.saturating_sub(term_height);

            if scroll_dir < 0 && self.scroll_offset_y > 0 {
                self.scroll_offset_y = self.scroll_offset_y.saturating_sub(1);
                state_changed = true;
            } else if scroll_dir > 0 && self.scroll_offset_y < max_scroll {
                self.scroll_offset_y = self.scroll_offset_y.saturating_add(1);
                state_changed = true;
            }
        }
        (state_changed, clicked_btn)
    }

    fn traverse_mouse(
        &mut self,
        id: NodeId,
        mx: i32,
        my: i32,
        click: bool,
        scroll_dir: i32,
        clip: Option<(i32, i32, i32, i32)>,
        offset_y: i32,
    ) -> (bool, Option<NodeId>, Option<NodeId>) {
        let (node_type, layout, children) = {
            if let Some(node) = self.nodes.get(id) {
                (node.node_type.clone(), node.layout, node.children.clone())
            } else {
                return (false, None, None);
            }
        };

        let (current_offset, current_clip) = match node_type {
            NodeType::ScrollArea {
                scroll_offset_y, ..
            } => {
                let cy1 = layout.y as i32 - offset_y;
                let cy2 = cy1 + layout.height as i32;
                let clip_rect = Some((layout.x as i32, cy1, (layout.x + layout.width) as i32, cy2));
                (scroll_offset_y as i32, clip_rect)
            }
            NodeType::DockBottom => (0, None),
            _ => (offset_y, clip),
        };

        if let Some((cx1, cy1, cx2, cy2)) = clip {
            if mx < cx1 || mx >= cx2 || my < cy1 || my >= cy2 {
                return (false, None, None);
            }
        }

        let mut state_changed = false;
        let mut clicked_btn = None;
        let mut new_focus = None;

        for child_id in children {
            let (sc, cb, nf) = self.traverse_mouse(
                child_id,
                mx,
                my,
                click,
                scroll_dir,
                current_clip,
                current_offset,
            );
            if sc {
                state_changed = true;
            }
            if cb.is_some() {
                clicked_btn = cb;
            }
            if nf.is_some() {
                new_focus = nf;
            }
        }

        let screen_y = layout.y as i32 - offset_y;
        let screen_x = layout.x as i32;
        let in_bounds = mx >= screen_x
            && mx < screen_x + layout.width as i32
            && my >= screen_y
            && my < screen_y + layout.height as i32;

        if in_bounds {
            if scroll_dir != 0 {
                if let NodeType::ScrollArea {
                    scroll_offset_y, ..
                } = node_type
                {
                    let mut max_bottom = layout.y;
                    if let Some(node) = self.nodes.get(id) {
                        for &cid in &node.children {
                            if let Some(child) = self.nodes.get(cid) {
                                let bottom = child.layout.y + child.layout.height;
                                if bottom > max_bottom {
                                    max_bottom = bottom;
                                }
                            }
                        }
                    }

                    let content_height = max_bottom.saturating_sub(layout.y);
                    let max_scroll = content_height.saturating_sub(layout.height);

                    let mut new_offset = scroll_offset_y;
                    if scroll_dir < 0 && new_offset > 0 {
                        new_offset = new_offset.saturating_sub(1);
                    } else if scroll_dir > 0 && new_offset < max_scroll {
                        new_offset = new_offset.saturating_add(1);
                    }

                    if new_offset != scroll_offset_y {
                        if let Some(node) = self.nodes.get_mut(id) {
                            if let NodeType::ScrollArea {
                                scroll_offset_y: ref mut sy,
                                ..
                            } = node.node_type
                            {
                                *sy = new_offset;
                                state_changed = true;
                            }
                        }
                    }
                }
            }

            if click {
                match node_type {
                    NodeType::Button(_) => {
                        clicked_btn = Some(id);
                    }
                    NodeType::Checkbox(checked) => {
                        if let Some(node) = self.nodes.get_mut(id) {
                            if let NodeType::Checkbox(ref mut state) = node.node_type {
                                *state = !checked;
                                state_changed = true;
                            }
                        }
                    }
                    NodeType::EditLine(_) | NodeType::EditBox(_) => {
                        new_focus = Some(id);
                    }
                    _ => {}
                }
            }
        }
        (state_changed, clicked_btn, new_focus)
    }

    fn start_drawing(&mut self, root_id: NodeId) {
        // Resets the cursor position for each frame and starts drawing the root node recursively.
        self.cursor_pos = None;
        self.draw_node_recursive(root_id, None, self.scroll_offset_y as i32);
    }

    fn draw_node_recursive(
        &mut self,
        id: NodeId,
        clip: Option<(i32, i32, i32, i32)>,
        offset_y: i32,
    ) {
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

        let (current_offset, current_clip) = match node_type {
            NodeType::ScrollArea {
                scroll_offset_y, ..
            } => {
                let cy1 = layout.y as i32 - offset_y;
                let cy2 = cy1 + layout.height as i32;
                let clip_rect = Some((layout.x as i32, cy1, (layout.x + layout.width) as i32, cy2));
                (scroll_offset_y as i32, clip_rect)
            }
            NodeType::DockBottom => (0, None),
            _ => (offset_y, clip),
        };

        match node_type {
            NodeType::Text(ref text) => {
                self.draw_text(text, &layout, current_offset, style, current_clip)
            }
            NodeType::EditLine(ref text) => {
                let cursor = self
                    .nodes
                    .get(id)
                    .and_then(|node| node.editor.as_ref())
                    .map(|editor| editor.cursor());
                self.draw_edit_line(
                    id,
                    text,
                    cursor,
                    &layout,
                    current_offset,
                    style,
                    current_clip,
                )
            }
            NodeType::Button(ref text) => {
                self.draw_button(text, &layout, current_offset, style, current_clip)
            }
            NodeType::Checkbox(checked) => {
                self.draw_checkbox(checked, &layout, current_offset, style, current_clip)
            }
            NodeType::Border => self.draw_border(&layout, current_offset, style, current_clip),
            NodeType::EditBox(ref text) => {
                let cursor = self
                    .nodes
                    .get(id)
                    .and_then(|node| node.editor.as_ref())
                    .map(|editor| editor.cursor());
                self.draw_edit_box(
                    id,
                    text,
                    cursor,
                    &layout,
                    current_offset,
                    style,
                    current_clip,
                )
            }
            _ => {}
        }

        for child_id in children {
            self.draw_node_recursive(child_id, current_clip, current_offset);
        }

        if matches!(node_type, NodeType::ScrollArea { .. }) {
            self.draw_scrollbar(id, &layout, current_offset, style, clip);
        }
    }

    /// Draws a compact vertical scrollbar for a scroll area.
    /// The track is clipped to the area and the thumb reflects the actual
    /// content height and current offset.
    fn draw_scrollbar(
        &mut self,
        id: NodeId,
        layout: &ComputedLayout,
        offset_y: i32,
        style: NodeStyle,
        clip: Option<(i32, i32, i32, i32)>,
    ) {
        let viewport = layout.height as i32;
        if layout.width == 0 || viewport <= 0 {
            return;
        }

        let (scroll_offset, children) = match self.nodes.get(id) {
            Some(node) => {
                let scroll_offset = match node.node_type {
                    NodeType::ScrollArea {
                        scroll_offset_y, ..
                    } => scroll_offset_y as i32,
                    _ => return,
                };
                (scroll_offset, node.children.clone())
            }
            None => return,
        };

        // Chat content is often wrapped in Stack/Border nodes. Include every
        // descendant when determining the actual scrollable content height.
        let mut pending = children;
        let mut content_bottom = layout.y as i32;
        while let Some(child_id) = pending.pop() {
            if let Some(child) = self.nodes.get(child_id) {
                content_bottom =
                    content_bottom.max(child.layout.y as i32 + child.layout.height as i32);
                pending.extend(child.children.iter().copied());
            }
        }

        let content_height = (content_bottom - layout.y as i32).max(0);

        if content_height <= viewport {
            return;
        }

        let x = layout.x as i32 + layout.width as i32 - 1;
        let y = layout.y as i32 - offset_y;
        let max_scroll = content_height - viewport;

        // Draw the track only inside the clipped viewport.
        for row in 0..viewport {
            self.next_buffer
                .set_char_clamped(x, y + row, '│', style, clip);
        }

        // Keep the thumb at least one cell high and proportional to the
        // visible fraction of the scrollable content.
        let thumb_height = ((viewport * viewport) / content_height)
            .max(1)
            .min(viewport);
        let thumb_travel = viewport - thumb_height;
        let thumb_start = if max_scroll == 0 {
            0
        } else {
            (scroll_offset.clamp(0, max_scroll) * thumb_travel) / max_scroll
        };

        for row in thumb_start..thumb_start + thumb_height {
            self.next_buffer
                .set_char_clamped(x, y + row, '█', style, clip);
        }
    }

    fn draw_text(
        &mut self,
        text: &str,
        layout: &ComputedLayout,
        offset_y: i32,
        style: NodeStyle,
        clip: Option<(i32, i32, i32, i32)>,
    ) {
        let mut cursor_x = layout.x as i32;
        let mut cursor_y = layout.y as i32 - offset_y;
        let max_x = (layout.x + layout.width) as i32;

        let mut current_style = style;
        let mut in_bold = false;

        let mut chars = text.chars().peekable();

        while let Some(ch) = chars.next() {
            if ch == '*' && chars.peek() == Some(&'*') {
                chars.next();
                in_bold = !in_bold;

                current_style.modifier = if in_bold {
                    Attribute::Bold
                } else {
                    style.modifier
                };
                continue;
            }

            if ch == '\n' {
                cursor_x = layout.x as i32;
                cursor_y += 1;
            } else {
                if cursor_x < max_x {
                    self.next_buffer
                        .set_char_clamped(cursor_x, cursor_y, ch, current_style, clip);
                    cursor_x += 1;
                } else {
                    // Line wrap
                    cursor_x = layout.x as i32;
                    cursor_y += 1;
                    self.next_buffer
                        .set_char_clamped(cursor_x, cursor_y, ch, current_style, clip);
                    cursor_x += 1;
                }
            }

            if cursor_y >= (layout.y + layout.height) as i32 - offset_y {
                break;
            }
        }
    }

    fn draw_edit_line(
        &mut self,
        id: NodeId,
        text: &str,
        cursor: Option<usize>,
        layout: &ComputedLayout,
        offset_y: i32,
        style: NodeStyle,
        clip: Option<(i32, i32, i32, i32)>,
    ) {
        // Draws an editable single-line text field with a prefix and cursor handling.
        let mut cursor_x = layout.x as i32;
        let cursor_y = layout.y as i32 - offset_y;
        let max_x = (layout.x + layout.width) as i32;

        self.next_buffer
            .set_char_clamped(cursor_x, cursor_y, '>', style, clip);
        cursor_x += 2;

        let cursor_byte = cursor.unwrap_or(text.len()).min(text.len());
        for (index, ch) in text.char_indices() {
            if cursor_x < max_x {
                self.next_buffer
                    .set_char_clamped(cursor_x, cursor_y, ch, style, clip);
                cursor_x += 1;
            }
            if index + ch.len_utf8() >= cursor_byte {
                break;
            }
        }
        if cursor_byte == 0 {
            cursor_x = layout.x as i32 + 2;
        }

        // Saves the cursor position for later rendering if the node is focused.
        if Some(id) == self.focused_node {
            if cursor_y >= 0 && cursor_y < self.next_buffer.height as i32 {
                let mut show_cursor = true;
                if let Some((cx1, cy1, cx2, cy2)) = clip {
                    if cursor_x < cx1 || cursor_x >= cx2 || cursor_y < cy1 || cursor_y >= cy2 {
                        show_cursor = false;
                    }
                }
                if show_cursor {
                    self.cursor_pos = Some((cursor_x as u16, cursor_y as u16));
                }
            }
        }
    }

    fn draw_button(
        &mut self,
        text: &str,
        layout: &ComputedLayout,
        offset_y: i32,
        style: NodeStyle,
        clip: Option<(i32, i32, i32, i32)>,
    ) {
        // Draws a button with a label enclosed in square brackets.
        let mut cursor_x = layout.x as i32;
        let cursor_y = layout.y as i32 - offset_y;

        self.next_buffer
            .set_char_clamped(cursor_x, cursor_y, '[', style, clip);
        self.next_buffer
            .set_char_clamped(cursor_x + 1, cursor_y, ' ', style, clip);
        cursor_x += 2;

        for ch in text.chars() {
            if cursor_x < (layout.x + layout.width - 2) as i32 {
                self.next_buffer
                    .set_char_clamped(cursor_x, cursor_y, ch, style, clip);
                cursor_x += 1;
            }
        }

        if cursor_x < (layout.x + layout.width) as i32 {
            self.next_buffer
                .set_char_clamped(cursor_x, cursor_y, ' ', style, clip);
            self.next_buffer
                .set_char_clamped(cursor_x + 1, cursor_y, ']', style, clip);
        }
    }

    fn draw_checkbox(
        &mut self,
        checked: bool,
        layout: &ComputedLayout,
        offset_y: i32,
        style: NodeStyle,
        clip: Option<(i32, i32, i32, i32)>,
    ) {
        // Draws a checkbox with a checked or unchecked state.
        let cursor_x = layout.x as i32;
        let cursor_y = layout.y as i32 - offset_y;

        self.next_buffer
            .set_char_clamped(cursor_x, cursor_y, '[', style, clip);
        let mark = if checked { 'X' } else { ' ' };
        self.next_buffer
            .set_char_clamped(cursor_x + 1, cursor_y, mark, style, clip);
        self.next_buffer
            .set_char_clamped(cursor_x + 2, cursor_y, ']', style, clip);
    }

    fn draw_border(
        &mut self,
        layout: &ComputedLayout,
        offset_y: i32,
        style: NodeStyle,
        clip: Option<(i32, i32, i32, i32)>,
    ) {
        // Draws a border around a node using box-drawing characters.
        let x = layout.x as i32;
        let y = layout.y as i32 - offset_y;
        let w = layout.width as i32;
        let h = layout.height as i32;

        if w > 1 && h > 1 {
            self.next_buffer.set_char_clamped(x, y, '┌', style, clip);
            for i in 1..(w - 1) {
                self.next_buffer
                    .set_char_clamped(x + i, y, '─', style, clip);
            }
            self.next_buffer
                .set_char_clamped(x + w - 1, y, '┐', style, clip);

            for j in 1..(h - 1) {
                self.next_buffer
                    .set_char_clamped(x, y + j, '│', style, clip);
                self.next_buffer
                    .set_char_clamped(x + w - 1, y + j, '│', style, clip);
            }

            self.next_buffer
                .set_char_clamped(x, y + h - 1, '└', style, clip);
            for i in 1..(w - 1) {
                self.next_buffer
                    .set_char_clamped(x + i, y + h - 1, '─', style, clip);
            }
            self.next_buffer
                .set_char_clamped(x + w - 1, y + h - 1, '┘', style, clip);
        }
    }

    fn draw_edit_box(
        &mut self,
        id: NodeId,
        text: &str,
        cursor: Option<usize>,
        layout: &ComputedLayout,
        offset_y: i32,
        style: NodeStyle,
        clip: Option<(i32, i32, i32, i32)>,
    ) {
        // Draws a multi-line editable text box, handling line wrapping and cursor positioning.
        let mut cursor_x = layout.x as i32;
        let mut cursor_y = layout.y as i32 - offset_y;
        let max_x = (layout.x + layout.width) as i32;

        let cursor_byte = cursor.unwrap_or(text.len()).min(text.len());
        for (index, ch) in text.char_indices() {
            if ch == '\n' {
                cursor_x = layout.x as i32;
                cursor_y += 1;
            } else {
                if cursor_x >= max_x {
                    cursor_x = layout.x as i32;
                    cursor_y += 1;
                }
                self.next_buffer
                    .set_char_clamped(cursor_x, cursor_y, ch, style, clip);
                cursor_x += 1;
            }
            if index + ch.len_utf8() >= cursor_byte {
                break;
            }
        }
        if cursor_byte == 0 {
            cursor_x = layout.x as i32;
            cursor_y = layout.y as i32 - offset_y;
        }

        // Saves the cursor position for later rendering if the node is focused.
        if Some(id) == self.focused_node {
            if cursor_y >= 0 && cursor_y < self.next_buffer.height as i32 {
                let mut show_cursor = true;
                if let Some((cx1, cy1, cx2, cy2)) = clip {
                    if cursor_x < cx1 || cursor_x >= cx2 || cursor_y < cy1 || cursor_y >= cy2 {
                        show_cursor = false;
                    }
                }
                if show_cursor {
                    self.cursor_pos = Some((cursor_x as u16, cursor_y as u16));
                }
            }
        }
    }
}
