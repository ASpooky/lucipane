//! Global mouse input for the pen. Blocks the calling thread forever.

use crate::pen::Input;

/// rdev 0.5 is not used on macOS: it taps the keyboard too and looks up the keyboard layout
/// off the main thread on every key press, which macOS aborts the process for, and it never
/// reports the middle button.
#[cfg(target_os = "macos")]
pub fn listen(scale: f64, on_input: impl FnMut(Input)) -> Result<(), String> {
    use core_foundation::runloop::CFRunLoop;
    use core_graphics::event::{
        CGEventTap, CGEventTapLocation, CGEventTapOptions, CGEventTapPlacement, CGEventType,
        CallbackResult, EventField,
    };
    use std::cell::RefCell;

    const MIDDLE_BUTTON: i64 = 2;
    let on_input = RefCell::new(on_input);
    CGEventTap::with_enabled(
        CGEventTapLocation::Session,
        CGEventTapPlacement::HeadInsertEventTap,
        CGEventTapOptions::ListenOnly,
        vec![
            CGEventType::MouseMoved,
            CGEventType::LeftMouseDragged,
            CGEventType::RightMouseDragged,
            CGEventType::OtherMouseDragged,
            CGEventType::OtherMouseDown,
            CGEventType::OtherMouseUp,
        ],
        |_proxy, event_type, event| {
            let mut emit = on_input.borrow_mut();
            // Event locations are in points; the pen works in physical pixels.
            let point = event.location();
            emit(Input::Move {
                x: point.x * scale,
                y: point.y * scale,
            });
            let middle = event.get_integer_value_field(EventField::MOUSE_EVENT_BUTTON_NUMBER)
                == MIDDLE_BUTTON;
            match event_type {
                CGEventType::OtherMouseDown if middle => emit(Input::MiddleDown),
                CGEventType::OtherMouseUp if middle => emit(Input::MiddleUp),
                _ => {}
            }
            CallbackResult::Keep
        },
        CFRunLoop::run_current,
    )
    .map_err(|()| "could not create the event tap".to_string())
}

#[cfg(not(target_os = "macos"))]
pub fn listen(_scale: f64, mut on_input: impl FnMut(Input) + 'static) -> Result<(), String> {
    rdev::listen(move |event| {
        let input = match event.event_type {
            rdev::EventType::MouseMove { x, y } => Input::Move { x, y },
            rdev::EventType::ButtonPress(rdev::Button::Middle) => Input::MiddleDown,
            rdev::EventType::ButtonRelease(rdev::Button::Middle) => Input::MiddleUp,
            // rdev reports whole notches; smaller (touchpad) deltas come through as 0.
            rdev::EventType::Wheel { delta_y, .. } if delta_y != 0 => Input::Wheel {
                notches: delta_y as f64,
            },
            _ => return,
        };
        on_input(input);
    })
    .map_err(|e| format!("{e:?}"))
}
