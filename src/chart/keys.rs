//! Arrow-key movement between a chart's marks.

use dioxus::prelude::*;

/// What a key does to the active mark.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum KeyMove {
    /// Make the mark at this position active.
    To(usize),
    /// Select the active mark (Enter or Space).
    Activate,
    /// Clear the active mark (Escape).
    Leave,
    /// Not a chart key: let it through.
    None,
}

/// Right/Down go forward and Left/Up back, wrapping; Home and End jump to
/// the ends. With nothing active yet, any arrow starts at the first mark.
pub(crate) fn key_move(key: &Key, at: Option<usize>, count: usize) -> KeyMove {
    if count == 0 {
        return KeyMove::None;
    }
    let last = count - 1;
    match key {
        Key::Named(NamedKey::ArrowRight | NamedKey::ArrowDown) => {
            KeyMove::To(at.map_or(0, |i| if i >= last { 0 } else { i + 1 }))
        }
        Key::Named(NamedKey::ArrowLeft | NamedKey::ArrowUp) => {
            KeyMove::To(at.map_or(0, |i| if i == 0 { last } else { i - 1 }))
        }
        Key::Named(NamedKey::Home) => KeyMove::To(0),
        Key::Named(NamedKey::End) => KeyMove::To(last),
        Key::Named(NamedKey::Enter) => KeyMove::Activate,
        Key::Character(c) if c == " " => KeyMove::Activate,
        Key::Named(NamedKey::Escape) => KeyMove::Leave,
        _ => KeyMove::None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arrows_wrap_and_ends_jump() {
        let right = Key::Named(NamedKey::ArrowRight);
        let left = Key::Named(NamedKey::ArrowLeft);
        assert_eq!(key_move(&right, None, 3), KeyMove::To(0));
        assert_eq!(key_move(&right, Some(2), 3), KeyMove::To(0));
        assert_eq!(key_move(&left, Some(0), 3), KeyMove::To(2));
        assert_eq!(
            key_move(&Key::Named(NamedKey::End), None, 3),
            KeyMove::To(2)
        );
        assert_eq!(
            key_move(&Key::Character(" ".into()), Some(1), 3),
            KeyMove::Activate
        );
        assert_eq!(key_move(&right, None, 0), KeyMove::None);
        assert_eq!(
            key_move(&Key::Character("a".into()), None, 3),
            KeyMove::None
        );
    }
}
