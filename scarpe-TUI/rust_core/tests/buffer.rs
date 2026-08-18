use crossterm::style::{Attribute, Color};
use rust_core::{Buffer, NodeStyle};

fn test_style() -> NodeStyle {
    NodeStyle {
        fg: Color::Green,
        bg: Color::Black,
        modifier: Attribute::Bold,
    }
}

#[test]
fn new_buffer_is_filled_with_default_cells() {
    let buffer = Buffer::new(4, 3);

    assert_eq!(buffer.width, 4);
    assert_eq!(buffer.height, 3);
    assert_eq!(buffer.content.len(), 12);
    assert!(buffer.content.iter().all(|cell| cell.ch == ' '));
}

#[test]
fn zero_sized_buffer_has_no_cells() {
    let buffer = Buffer::new(0, 0);

    assert_eq!(buffer.content.len(), 0);
}

#[test]
fn set_char_writes_character_and_style() {
    let mut buffer = Buffer::new(4, 2);
    let style = test_style();

    buffer.set_char(2, 1, 'A', style);

    let cell = buffer.content[6];
    assert_eq!(cell.ch, 'A');
    assert_eq!(cell.fg, Color::Green);
    assert_eq!(cell.bg, Color::Black);
    assert_eq!(cell.modifier, Attribute::Bold);
}

#[test]
fn set_char_ignores_coordinates_outside_buffer() {
    let mut buffer = Buffer::new(2, 2);
    let original = buffer.content.clone();

    buffer.set_char(2, 0, 'X', test_style());
    buffer.set_char(0, 2, 'Y', test_style());

    assert_eq!(buffer.content, original);
}

#[test]
fn set_char_clamped_respects_buffer_bounds_and_clip() {
    let mut buffer = Buffer::new(5, 3);
    let style = test_style();

    buffer.set_char_clamped(1, 1, 'A', style, Some((1, 1, 3, 2)));
    buffer.set_char_clamped(0, 1, 'B', style, Some((1, 1, 3, 2)));
    buffer.set_char_clamped(-1, 1, 'C', NodeStyle::default(), None);
    buffer.set_char_clamped(4, 2, 'D', NodeStyle::default(), None);

    assert_eq!(buffer.content[6].ch, 'A');
    assert_eq!(buffer.content[5].ch, ' ');
    assert_eq!(buffer.content[14].ch, 'D');
}

#[test]
fn empty_clip_rect_does_not_write() {
    let mut buffer = Buffer::new(2, 1);

    buffer.set_char_clamped(0, 0, 'X', test_style(), Some((1, 0, 1, 1)));

    assert_eq!(buffer.content[0].ch, ' ');
}

#[test]
fn reset_restores_all_cells_to_defaults() {
    let mut buffer = Buffer::new(3, 2);

    buffer.set_char(0, 0, 'X', test_style());
    buffer.set_char(2, 1, 'Y', test_style());
    buffer.reset();

    assert!(buffer.content.iter().all(|cell| cell.ch == ' '));
    assert!(buffer.content.iter().all(|cell| cell.fg == Color::Reset));
    assert!(buffer.content.iter().all(|cell| cell.bg == Color::Reset));
    assert!(buffer
        .content
        .iter()
        .all(|cell| cell.modifier == Attribute::Reset));
}
