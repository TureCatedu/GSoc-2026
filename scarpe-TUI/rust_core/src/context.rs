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
    pub fn new(use_alternate: bool) -> Result<Self, Error> {
        enable_raw_mode()?; 
        if use_alternate { stdout().execute(EnterAlternateScreen)?; }
        stdout().execute(EnableMouseCapture)?; 
        stdout().execute(Clear(ClearType::All))?; 

        let (width, height) = size().unwrap_or((80, 24));
        let (tx, rx) = mpsc::channel();
        
        thread::spawn(move || {
            loop {
                if let Ok(event) = crossterm::event::read() {
                    if tx.send(event).is_err() { break; }
                } else { break; }
            }
        });

        Ok(ScarpeTuiContext {
            use_alternate,
            nodes: Slab::with_capacity(1024), 
            root_id: None,
            current_buffer: Buffer::new(width, height), 
            next_buffer: Buffer::new(width, height),    
            focused_node: None,                         
            needs_redraw: true,                         
            clicked_button: None,                       
            scroll_offset_y: 0,                         
            event_receiver: Some(rx),      
            cursor_pos: None, // Inizializzazione della memoria             
        })
    }

    pub fn shutdown(&mut self) -> Result<(), Error> {
        if self.use_alternate { stdout().execute(LeaveAlternateScreen)?; }
        disable_raw_mode()?; 
        stdout().execute(DisableMouseCapture)?; 
        Ok(())
    }

    pub fn render(&mut self) -> Result<(), Error> {
        if !self.needs_redraw { return Ok(()); }

        let (term_width, term_height) = size().unwrap_or((80, 24)); 

        if self.next_buffer.width != term_width || self.next_buffer.height != term_height {
            self.next_buffer = Buffer::new(term_width, term_height);
            self.current_buffer = Buffer::new(term_width, term_height);
            stdout().execute(Clear(ClearType::All))?; 
        }

        self.next_buffer.reset(); 
        self.compute_layouts(); 

        if let Some(root_id) = self.root_id {
            self.start_drawing(root_id);
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

                    if next_cell.modifier != current_attr {
                        out.queue(SetAttribute(next_cell.modifier))?;
                        current_attr = next_cell.modifier;
                        if next_cell.modifier == Attribute::Reset {
                            current_fg = Color::Reset;
                            current_bg = Color::Reset;
                        }
                    }

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

        out.queue(ResetColor)?; 

        // NUOVA LOGICA: Posiziona il cursore lampeggiante solo DOPO aver finito di stampare tutto!
        if let Some((cx, cy)) = self.cursor_pos {
            out.queue(MoveTo(cx, cy))?;
            out.queue(Show)?;
        } else {
            out.queue(Hide)?;
        }

        out.flush()?; 

        swap(&mut self.current_buffer, &mut self.next_buffer); 
        self.needs_redraw = false; 
        Ok(())
    }

    pub fn compute_layouts(&mut self) {
        let (term_width, _) = size().unwrap_or((80, 24)); 
        if let Some(root_id) = self.root_id {
            self.layout_node(root_id, 0, 0, term_width); 
        }
    }

    fn layout_node(&mut self, id: NodeId, start_x: u16, start_y: u16, max_width: u16) -> ComputedLayout {
        let (node_type, children) = {
            if let Some(node) = self.nodes.get(id) {
                (node.node_type.clone(), node.children.clone())
            } else { return ComputedLayout::default(); }
        };

        let (computed_width, computed_height) = match node_type {
            NodeType::Root | NodeType::Stack => self.layout_stack(children, start_x, start_y, max_width),
            NodeType::Flow => self.layout_flow(children, start_x, start_y, max_width),
            NodeType::Border => self.layout_border(children, start_x, start_y, max_width),
            NodeType::Checkbox(_) => (3, 1), 
            NodeType::Text(ref text) | NodeType::EditLine(ref text) | NodeType::Button(ref text) => self.layout_simple_text(&node_type, text, max_width),
            NodeType::EditBox(ref text) => self.layout_edit_box(text, max_width),
            NodeType::DockBottom => self.layout_dock_bottom(children, start_x, max_width),
            NodeType::ScrollArea { scroll_offset_y: _, max_height } => self.layout_scroll_area(children, start_x, start_y, max_width, max_height),
        };

        let final_layout = ComputedLayout { x: start_x, y: start_y, width: computed_width, height: computed_height };
        if let Some(node) = self.nodes.get_mut(id) { node.layout = final_layout; }
        final_layout
    }

    fn layout_stack(&mut self, children: Vec<NodeId>, start_x: u16, start_y: u16, max_width: u16) -> (u16, u16) {
        let mut current_y = start_y;
        let mut computed_height = 0;
        for child_id in children {
            let child_layout = self.layout_node(child_id, start_x, current_y, max_width);
            current_y += child_layout.height; 
            computed_height += child_layout.height;
        }
        (max_width, computed_height)
    }

    fn layout_flow(&mut self, children: Vec<NodeId>, start_x: u16, start_y: u16, max_width: u16) -> (u16, u16) {
        let mut current_x = start_x;
        let mut current_y = start_y;
        let mut computed_width = 0;
        let mut computed_height = 0;
        let mut row_height = 0;

        for child_id in children {
            let available_width = max_width.saturating_sub(current_x - start_x);
            let mut child_layout = self.layout_node(child_id, current_x, current_y, available_width);

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

    fn layout_border(&mut self, children: Vec<NodeId>, start_x: u16, start_y: u16, max_width: u16) -> (u16, u16) {
        let child_start_x = start_x + 1; 
        let mut child_start_y = start_y + 1; 
        let available_child_width = max_width.saturating_sub(2); 

        let mut max_child_width = 0;
        let mut total_child_height = 0;

        for child_id in children {
            let child_layout = self.layout_node(child_id, child_start_x, child_start_y, available_child_width);
            child_start_y += child_layout.height;
            total_child_height += child_layout.height;
            max_child_width = max_child_width.max(child_layout.width);
        }
        (max_child_width + 2, total_child_height + 2) 
    }

    fn layout_simple_text(&self, node_type: &NodeType, text: &str, max_width: u16) -> (u16, u16) {
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
        
        if matches!(node_type, NodeType::Button(_)) { current_line_width += 4; }
        if matches!(node_type, NodeType::EditLine(_)) { current_line_width += 3; }
        
        max_w = max_w.max(current_line_width);
        let mut computed_width = max_w.min(max_width);
        if computed_width == 0 { computed_width = 1; }
        (computed_width, total_height)
    }

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
        (max_w.max(10).min(max_width), total_height.max(3)) 
    }

    fn layout_dock_bottom(&mut self, children: Vec<NodeId>, start_x: u16, max_width: u16) -> (u16, u16) {
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

    fn layout_scroll_area(&mut self, children: Vec<NodeId>, start_x: u16, start_y: u16, max_width: u16, max_height: u16) -> (u16, u16) {
        self.layout_stack(children, start_x, start_y, max_width);
        (max_width, max_height)
    }

    pub fn handle_mouse(&mut self, mx: u16, my: u16, click: bool, scroll_dir: i32) -> (bool, Option<NodeId>) {
        let mut state_changed = false;
        let mut clicked_btn = None;

        if let Some(root_id) = self.root_id {
            let (sc, cb, focus) = self.traverse_mouse(
                root_id, mx as i32, my as i32, click, scroll_dir, None, self.scroll_offset_y as i32
            );
            
            if sc { state_changed = true; }
            if cb.is_some() { clicked_btn = cb; }
            if focus.is_some() {
                self.focused_node = focus;
                state_changed = true;
            }
        }

        if scroll_dir != 0 && !state_changed {
            let mut max_content_height = 0;
            if let Some(root_id) = self.root_id {
                if let Some(root) = self.nodes.get(root_id) { max_content_height = root.layout.height; }
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
        &mut self, id: NodeId, mx: i32, my: i32, click: bool, scroll_dir: i32, clip: Option<(i32, i32, i32, i32)>, offset_y: i32,
    ) -> (bool, Option<NodeId>, Option<NodeId>) {
        let (node_type, layout, children) = {
            if let Some(node) = self.nodes.get(id) {
                (node.node_type.clone(), node.layout, node.children.clone())
            } else { return (false, None, None); }
        };

        let (current_offset, current_clip) = match node_type {
            NodeType::ScrollArea { scroll_offset_y, max_height } => {
                let cy1 = layout.y as i32 - offset_y;
                let cy2 = cy1 + max_height as i32;
                let clip_rect = Some((layout.x as i32, cy1, (layout.x + layout.width) as i32, cy2));
                (scroll_offset_y as i32, clip_rect)
            }
            NodeType::DockBottom => (0, None),
            _ => (offset_y, clip),
        };

        if let Some((cx1, cy1, cx2, cy2)) = clip {
            if mx < cx1 || mx >= cx2 || my < cy1 || my >= cy2 { return (false, None, None); }
        }

        let mut state_changed = false;
        let mut clicked_btn = None;
        let mut new_focus = None;

        for child_id in children {
            let (sc, cb, nf) = self.traverse_mouse(child_id, mx, my, click, scroll_dir, current_clip, current_offset);
            if sc { state_changed = true; }
            if cb.is_some() { clicked_btn = cb; }
            if nf.is_some() { new_focus = nf; }
        }

        let screen_y = layout.y as i32 - offset_y;
        let screen_x = layout.x as i32;
        let in_bounds = mx >= screen_x && mx < screen_x + layout.width as i32 && my >= screen_y && my < screen_y + layout.height as i32;

        if in_bounds {
            if scroll_dir != 0 {
                if let NodeType::ScrollArea { scroll_offset_y, max_height } = node_type {
                    let mut max_bottom = layout.y;
                    if let Some(node) = self.nodes.get(id) {
                        for &cid in &node.children {
                            if let Some(child) = self.nodes.get(cid) {
                                let bottom = child.layout.y + child.layout.height;
                                if bottom > max_bottom { max_bottom = bottom; }
                            }
                        }
                    }
                    
                    let content_height = max_bottom.saturating_sub(layout.y);
                    let max_scroll = content_height.saturating_sub(max_height);
                    
                    let mut new_offset = scroll_offset_y;
                    if scroll_dir < 0 && new_offset > 0 {
                        new_offset = new_offset.saturating_sub(1);
                    } else if scroll_dir > 0 && new_offset < max_scroll {
                        new_offset = new_offset.saturating_add(1);
                    }
                    
                    if new_offset != scroll_offset_y {
                        if let Some(node) = self.nodes.get_mut(id) {
                            if let NodeType::ScrollArea { scroll_offset_y: ref mut sy, .. } = node.node_type {
                                *sy = new_offset;
                                state_changed = true;
                            }
                        }
                    }
                }
            }

            if click {
                match node_type {
                    NodeType::Button(_) => { clicked_btn = Some(id); }
                    NodeType::Checkbox(checked) => {
                        if let Some(node) = self.nodes.get_mut(id) {
                            if let NodeType::Checkbox(ref mut state) = node.node_type {
                                *state = !checked;
                                state_changed = true;
                            }
                        }
                    }
                    NodeType::EditLine(_) | NodeType::EditBox(_) => { new_focus = Some(id); }
                    _ => {}
                }
            }
        }
        (state_changed, clicked_btn, new_focus)
    }

    fn start_drawing(&mut self, root_id: NodeId) {
        self.cursor_pos = None; // Reset della memoria del cursore ad ogni frame
        self.draw_node_recursive(root_id, None, self.scroll_offset_y as i32);
    }

    fn draw_node_recursive(&mut self, id: NodeId, clip: Option<(i32, i32, i32, i32)>, offset_y: i32) {
        let (node_type, layout, children, style) = {
            if let Some(node) = self.nodes.get(id) {
                (node.node_type.clone(), node.layout, node.children.clone(), node.style)
            } else { return; }
        };

        let (current_offset, current_clip) = match node_type {
            NodeType::ScrollArea { scroll_offset_y, max_height } => {
                let cy1 = layout.y as i32 - offset_y;
                let cy2 = cy1 + max_height as i32;
                let clip_rect = Some((layout.x as i32, cy1, (layout.x + layout.width) as i32, cy2));
                (scroll_offset_y as i32, clip_rect) 
            }
            NodeType::DockBottom => (0, None), 
            _ => (offset_y, clip),
        };

        match node_type {
            NodeType::Text(ref text) => self.draw_text(text, &layout, current_offset, style, current_clip),
            NodeType::EditLine(ref text) => self.draw_edit_line(id, text, &layout, current_offset, style, current_clip),
            NodeType::Button(ref text) => self.draw_button(text, &layout, current_offset, style, current_clip),
            NodeType::Checkbox(checked) => self.draw_checkbox(checked, &layout, current_offset, style, current_clip),
            NodeType::Border => self.draw_border(&layout, current_offset, style, current_clip),
            NodeType::EditBox(ref text) => self.draw_edit_box(id, text, &layout, current_offset, style, current_clip),
            _ => {}
        }

        for child_id in children {
            self.draw_node_recursive(child_id, current_clip, current_offset);
        }
    }

    fn draw_text(&mut self, text: &str, layout: &ComputedLayout, offset_y: i32, style: NodeStyle, clip: Option<(i32, i32, i32, i32)>) {
        let mut cursor_x = layout.x as i32;
        let mut cursor_y = layout.y as i32 - offset_y;
        let max_x = (layout.x + layout.width) as i32;

        for ch in text.chars() {
            if ch == '\n' {
                cursor_x = layout.x as i32;
                cursor_y += 1;
            } else {
                if cursor_x < max_x {
                    self.next_buffer.set_char_clamped(cursor_x, cursor_y, ch, style, clip);
                    cursor_x += 1;
                } else {
                    cursor_x = layout.x as i32;
                    cursor_y += 1;
                    self.next_buffer.set_char_clamped(cursor_x, cursor_y, ch, style, clip);
                    cursor_x += 1;
                }
            }
            if cursor_y >= (layout.y + layout.height) as i32 - offset_y { break; }
        }
    }

    fn draw_edit_line(&mut self, id: NodeId, text: &str, layout: &ComputedLayout, offset_y: i32, style: NodeStyle, clip: Option<(i32, i32, i32, i32)>) {
        let mut cursor_x = layout.x as i32;
        let cursor_y = layout.y as i32 - offset_y;
        let max_x = (layout.x + layout.width) as i32;

        self.next_buffer.set_char_clamped(cursor_x, cursor_y, '>', style, clip);
        cursor_x += 2;

        for ch in text.chars() {
            if cursor_x < max_x {
                self.next_buffer.set_char_clamped(cursor_x, cursor_y, ch, style, clip);
                cursor_x += 1;
            }
        }

        // Salvataggio della coordinata invece dell'esecuzione immediata!
        if Some(id) == self.focused_node {
            if cursor_y >= 0 && cursor_y < self.next_buffer.height as i32 {
                let mut show_cursor = true;
                if let Some((cx1, cy1, cx2, cy2)) = clip {
                    if cursor_x < cx1 || cursor_x >= cx2 || cursor_y < cy1 || cursor_y >= cy2 { show_cursor = false; }
                }
                if show_cursor {
                    self.cursor_pos = Some((cursor_x as u16, cursor_y as u16));
                }
            }
        }
    }

    fn draw_button(&mut self, text: &str, layout: &ComputedLayout, offset_y: i32, style: NodeStyle, clip: Option<(i32, i32, i32, i32)>) {
        let mut cursor_x = layout.x as i32;
        let cursor_y = layout.y as i32 - offset_y;

        self.next_buffer.set_char_clamped(cursor_x, cursor_y, '[', style, clip);
        self.next_buffer.set_char_clamped(cursor_x + 1, cursor_y, ' ', style, clip);
        cursor_x += 2;

        for ch in text.chars() {
            if cursor_x < (layout.x + layout.width - 2) as i32 {
                self.next_buffer.set_char_clamped(cursor_x, cursor_y, ch, style, clip);
                cursor_x += 1;
            }
        }

        if cursor_x < (layout.x + layout.width) as i32 {
            self.next_buffer.set_char_clamped(cursor_x, cursor_y, ' ', style, clip);
            self.next_buffer.set_char_clamped(cursor_x + 1, cursor_y, ']', style, clip);
        }
    }

    fn draw_checkbox(&mut self, checked: bool, layout: &ComputedLayout, offset_y: i32, style: NodeStyle, clip: Option<(i32, i32, i32, i32)>) {
        let cursor_x = layout.x as i32;
        let cursor_y = layout.y as i32 - offset_y;

        self.next_buffer.set_char_clamped(cursor_x, cursor_y, '[', style, clip);
        let mark = if checked { 'X' } else { ' ' };
        self.next_buffer.set_char_clamped(cursor_x + 1, cursor_y, mark, style, clip);
        self.next_buffer.set_char_clamped(cursor_x + 2, cursor_y, ']', style, clip);
    }

    fn draw_border(&mut self, layout: &ComputedLayout, offset_y: i32, style: NodeStyle, clip: Option<(i32, i32, i32, i32)>) {
        let x = layout.x as i32;
        let y = layout.y as i32 - offset_y;
        let w = layout.width as i32;
        let h = layout.height as i32;

        if w > 1 && h > 1 {
            self.next_buffer.set_char_clamped(x, y, '┌', style, clip);
            for i in 1..(w - 1) { self.next_buffer.set_char_clamped(x + i, y, '─', style, clip); }
            self.next_buffer.set_char_clamped(x + w - 1, y, '┐', style, clip);

            for j in 1..(h - 1) {
                self.next_buffer.set_char_clamped(x, y + j, '│', style, clip);
                self.next_buffer.set_char_clamped(x + w - 1, y + j, '│', style, clip);
            }

            self.next_buffer.set_char_clamped(x, y + h - 1, '└', style, clip);
            for i in 1..(w - 1) { self.next_buffer.set_char_clamped(x + i, y + h - 1, '─', style, clip); }
            self.next_buffer.set_char_clamped(x + w - 1, y + h - 1, '┘', style, clip);
        }
    }

    fn draw_edit_box(&mut self, id: NodeId, text: &str, layout: &ComputedLayout, offset_y: i32, style: NodeStyle, clip: Option<(i32, i32, i32, i32)>) {
        let mut cursor_x = layout.x as i32;
        let mut cursor_y = layout.y as i32 - offset_y;
        let max_x = (layout.x + layout.width) as i32;

        for ch in text.chars() {
            if ch == '\n' {
                cursor_x = layout.x as i32;
                cursor_y += 1;
            } else {
                if cursor_x < max_x {
                    self.next_buffer.set_char_clamped(cursor_x, cursor_y, ch, style, clip);
                    cursor_x += 1;
                } else {
                    cursor_x = layout.x as i32;
                    cursor_y += 1;
                    self.next_buffer.set_char_clamped(cursor_x, cursor_y, ch, style, clip);
                    cursor_x += 1;
                }
            }
        }

        // Salvataggio della coordinata invece dell'esecuzione immediata!
        if Some(id) == self.focused_node {
            if cursor_y >= 0 && cursor_y < self.next_buffer.height as i32 {
                let mut show_cursor = true;
                if let Some((cx1, cy1, cx2, cy2)) = clip {
                    if cursor_x < cx1 || cursor_x >= cx2 || cursor_y < cy1 || cursor_y >= cy2 { show_cursor = false; }
                }
                if show_cursor {
                    self.cursor_pos = Some((cursor_x as u16, cursor_y as u16));
                }
            }
        }
    }
}