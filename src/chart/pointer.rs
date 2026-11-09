//! Pointer listeners that exist only while they are needed.
//!
//! On the native renderers (Android and iOS web views) every Rust event
//! handler is a synchronous call from the page into Rust, and bubbling
//! listeners are delegated once per event name on the root. A chart that
//! always listened for `pointermove` would make every pointer move in the
//! whole app wait on Rust. So a chart listens for moves only while a finger
//! is down on it or a mouse is over it: the listener is added on
//! `pointerdown`/`pointerenter` and removed again on release or leave. It is
//! the same element throughout, so a touch the element captured keeps
//! reaching it.

use dioxus::core::{Attribute, AttributeValue, ListenerCallback};
use dioxus::html::PlatformEventData;
use dioxus::prelude::*;

/// An `on…` listener attribute that is present only when `handler` is
/// `Some`, for spreading onto an element (`div { ..attributes }`). Going from
/// `Some` to `None` removes the listener from the page.
pub(crate) fn optional_listener<D>(
    name: &'static str,
    handler: Option<impl FnMut(Event<D>) + 'static>,
) -> Attribute
where
    D: for<'a> From<&'a PlatformEventData> + 'static,
{
    let value = match handler {
        Some(mut handler) => AttributeValue::Listener(
            ListenerCallback::new(move |event: Event<PlatformEventData>| {
                handler(event.map(|data| D::from(data)));
            })
            .erase(),
        ),
        None => AttributeValue::None,
    };
    Attribute::new(name, value, None, false)
}

/// How far across an element a pointer is, `0.0..=1.0`, from its offset in
/// the element and the element's width. `None` until the width is known.
pub(crate) fn fraction(offset_x: f64, width: f64) -> Option<f64> {
    (width > 0.0 && offset_x.is_finite()).then(|| (offset_x / width).clamp(0.0, 1.0))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fractions_clamp_and_wait_for_a_width() {
        assert_eq!(fraction(50.0, 200.0), Some(0.25));
        assert_eq!(fraction(-5.0, 200.0), Some(0.0));
        assert_eq!(fraction(500.0, 200.0), Some(1.0));
        assert_eq!(fraction(5.0, 0.0), None);
    }
}
