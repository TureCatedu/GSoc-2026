#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextEditor {
    text: String,
    cursor: usize,
}

impl TextEditor {
    pub fn new(text: impl Into<String>) -> Self {
        let text = text.into();
        let cursor = text.len();
        Self { text, cursor }
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    pub fn cursor(&self) -> usize {
        self.cursor
    }

    pub fn insert(&mut self, ch: char) {
        self.text.insert(self.cursor, ch);
        self.cursor += ch.len_utf8();
    }

    pub fn backspace(&mut self) -> bool {
        if self.cursor == 0 {
            return false;
        }

        let previous = self
            .text
            .get(..self.cursor)
            .and_then(|prefix| prefix.char_indices().next_back())
            .map(|(index, _)| index)
            .expect("cursor must be on a valid character boundary");

        self.text.drain(previous..self.cursor);
        self.cursor = previous;
        true
    }

    pub fn move_left(&mut self) -> bool {
        if self.cursor == 0 {
            return false;
        }

        self.cursor = self
            .text
            .get(..self.cursor)
            .and_then(|prefix| prefix.char_indices().next_back())
            .map(|(index, _)| index)
            .expect("cursor must be on a valid character boundary");
        true
    }

    pub fn move_right(&mut self) -> bool {
        if self.cursor == self.text.len() {
            return false;
        }

        let next = self
            .text
            .get(self.cursor..)
            .and_then(|suffix| suffix.char_indices().nth(1))
            .map(|(index, _)| self.cursor + index)
            .unwrap_or(self.text.len());

        self.cursor = next;
        true
    }

    pub fn insert_newline(&mut self) {
        self.insert('\n');
    }

    pub fn move_home(&mut self) -> bool {
        let line_start = self.text[..self.cursor]
            .rfind('\n')
            .map(|index| index + 1)
            .unwrap_or(0);
        let changed = self.cursor != line_start;
        self.cursor = line_start;
        changed
    }

    pub fn move_end(&mut self) -> bool {
        let line_end = self.text[self.cursor..]
            .find('\n')
            .map(|index| self.cursor + index)
            .unwrap_or(self.text.len());
        let changed = self.cursor != line_end;
        self.cursor = line_end;
        changed
    }

    pub fn move_up(&mut self) -> bool {
        let current_start = self.text[..self.cursor]
            .rfind('\n')
            .map(|index| index + 1)
            .unwrap_or(0);
        if current_start == 0 {
            return false;
        }

        let column = self.text[current_start..self.cursor].chars().count();
        let previous_end = current_start - 1;
        let previous_start = self.text[..previous_end]
            .rfind('\n')
            .map(|index| index + 1)
            .unwrap_or(0);
        let target = self.text[previous_start..previous_end]
            .char_indices()
            .nth(column)
            .map(|(index, _)| previous_start + index)
            .unwrap_or(previous_end);

        let changed = target != self.cursor;
        self.cursor = target;
        changed
    }

    pub fn move_down(&mut self) -> bool {
        let current_start = self.text[..self.cursor]
            .rfind('\n')
            .map(|index| index + 1)
            .unwrap_or(0);
        let column = self.text[current_start..self.cursor].chars().count();
        let current_end = self.text[self.cursor..]
            .find('\n')
            .map(|index| self.cursor + index)
            .unwrap_or(self.text.len());
        if current_end == self.text.len() {
            return false;
        }

        let next_start = current_end + 1;
        let next_end = self.text[next_start..]
            .find('\n')
            .map(|index| next_start + index)
            .unwrap_or(self.text.len());
        let target = self.text[next_start..next_end]
            .char_indices()
            .nth(column)
            .map(|(index, _)| next_start + index)
            .unwrap_or(next_end);

        let changed = target != self.cursor;
        self.cursor = target;
        changed
    }
}

#[cfg(test)]
mod tests {
    use super::TextEditor;

    #[test]
    fn inserts_at_cursor_and_moves_cursor() {
        let mut editor = TextEditor::new("ac");

        editor.move_left();
        editor.insert('b');

        assert_eq!(editor.text(), "abc");
        assert_eq!(editor.cursor(), 2);
    }

    #[test]
    fn backspace_removes_previous_unicode_character() {
        let mut editor = TextEditor::new("a€");

        assert!(editor.backspace());

        assert_eq!(editor.text(), "a");
        assert_eq!(editor.cursor(), 1);
        assert!(!editor.backspace() || editor.text() == "");
    }

    #[test]
    fn movement_respects_unicode_boundaries() {
        let mut editor = TextEditor::new("a€b");

        editor.move_left();
        assert_eq!(editor.cursor(), "a€".len());

        editor.move_left();
        assert_eq!(editor.cursor(), 1);

        editor.move_right();
        assert_eq!(editor.cursor(), "a€".len());

        editor.move_right();
        assert_eq!(editor.cursor(), "a€b".len());
    }

    #[test]
    fn home_and_end_stay_within_current_line() {
        let mut editor = TextEditor::new("one\ntwo");

        assert!(editor.move_home());
        assert_eq!(editor.cursor(), 4);
        assert!(!editor.move_home());
        assert!(editor.move_end());
        assert_eq!(editor.cursor(), 7);
    }

    #[test]
    fn vertical_movement_preserves_column_when_possible() {
        let mut editor = TextEditor::new("abcd\nxy\n1234");

        assert!(editor.move_up());
        assert_eq!(editor.cursor(), 7);
        assert!(editor.move_up());
        assert_eq!(editor.cursor(), 2);

        assert!(editor.move_home());
        assert!(editor.move_down());
        assert_eq!(editor.cursor(), 5);
        assert!(editor.move_down());
        assert_eq!(editor.cursor(), 8);
    }

    #[test]
    fn newline_is_inserted_at_cursor() {
        let mut editor = TextEditor::new("ab");

        editor.move_left();
        editor.insert_newline();

        assert_eq!(editor.text(), "a\nb");
    }
}
