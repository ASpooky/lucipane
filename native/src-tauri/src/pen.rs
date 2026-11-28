//! Turns raw global mouse input into pen events for the overlay.
//! Pure logic: no OS hooks and no Tauri, so it can be unit-tested.

use serde::Serialize;

/// Mouse input we care about, decoupled from the hook library.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Input {
    /// Cursor moved to a global position in physical pixels.
    Move {
        x: f64,
        y: f64,
    },
    MiddleDown,
    MiddleUp,
    /// The vertical wheel turned by this many notches (positive: away from the user). It does
    /// not draw; it scrolls the window under the cursor (see scroll.rs).
    Wheel {
        notches: f64,
    },
}

/// Sent to the WebView. Coordinates are CSS pixels relative to the overlay window.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum PenEvent {
    Down { x: f64, y: f64 },
    Move { x: f64, y: f64 },
    Up,
}

/// Where the overlay sits on the virtual desktop.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Viewport {
    /// Top-left corner in global physical pixels.
    pub origin_x: f64,
    pub origin_y: f64,
    /// Physical pixels per CSS pixel (the monitor's scale factor).
    pub scale: f64,
}

impl Viewport {
    pub fn to_local(self, x: f64, y: f64) -> (f64, f64) {
        (
            (x - self.origin_x) / self.scale,
            (y - self.origin_y) / self.scale,
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct PenState {
    /// Last known cursor position (global physical). Button events carry no position.
    pub cursor: Option<(f64, f64)>,
    pub drawing: bool,
}

/// Advance the state by one input, maybe emitting a pen event.
pub fn step(state: PenState, input: Input, viewport: &Viewport) -> (PenState, Option<PenEvent>) {
    match input {
        Input::Move { x, y } => {
            let next = PenState {
                cursor: Some((x, y)),
                ..state
            };
            let event = state.drawing.then(|| {
                let (x, y) = viewport.to_local(x, y);
                PenEvent::Move { x, y }
            });
            (next, event)
        }
        Input::MiddleDown => match state.cursor {
            Some((x, y)) if !state.drawing => {
                let (x, y) = viewport.to_local(x, y);
                (
                    PenState {
                        drawing: true,
                        ..state
                    },
                    Some(PenEvent::Down { x, y }),
                )
            }
            _ => (state, None),
        },
        Input::MiddleUp if state.drawing => (
            PenState {
                drawing: false,
                ..state
            },
            Some(PenEvent::Up),
        ),
        Input::MiddleUp | Input::Wheel { .. } => (state, None),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const VP: Viewport = Viewport {
        origin_x: 100.0,
        origin_y: 50.0,
        scale: 2.0,
    };

    fn run(inputs: &[Input]) -> Vec<PenEvent> {
        let mut state = PenState::default();
        let mut out = Vec::new();
        for &input in inputs {
            let (next, ev) = step(state, input, &VP);
            state = next;
            out.extend(ev);
        }
        out
    }

    #[test]
    fn the_wheel_does_not_draw() {
        let wheel = Input::Wheel { notches: -1.0 };
        assert!(run(&[Input::Move { x: 0.0, y: 0.0 }, Input::MiddleDown, wheel]).len() == 1);
        assert!(run(&[wheel]).is_empty());
    }

    #[test]
    fn converts_global_physical_to_local_css() {
        assert_eq!(VP.to_local(300.0, 250.0), (100.0, 100.0));
    }

    #[test]
    fn moves_without_middle_button_emit_nothing() {
        assert!(run(&[Input::Move { x: 300.0, y: 250.0 }]).is_empty());
    }

    #[test]
    fn middle_drag_emits_down_move_up() {
        let events = run(&[
            Input::Move { x: 300.0, y: 250.0 },
            Input::MiddleDown,
            Input::Move { x: 310.0, y: 260.0 },
            Input::MiddleUp,
        ]);
        assert_eq!(
            events,
            vec![
                PenEvent::Down { x: 100.0, y: 100.0 },
                PenEvent::Move { x: 105.0, y: 105.0 },
                PenEvent::Up,
            ]
        );
    }

    #[test]
    fn middle_down_before_any_move_is_ignored() {
        assert!(run(&[Input::MiddleDown, Input::MiddleUp]).is_empty());
    }

    #[test]
    fn middle_up_without_down_is_ignored() {
        assert!(run(&[Input::Move { x: 0.0, y: 0.0 }, Input::MiddleUp]).is_empty());
    }

    #[test]
    fn moves_after_release_emit_nothing() {
        let events = run(&[
            Input::Move { x: 300.0, y: 250.0 },
            Input::MiddleDown,
            Input::MiddleUp,
            Input::Move { x: 310.0, y: 260.0 },
        ]);
        assert_eq!(
            events,
            vec![PenEvent::Down { x: 100.0, y: 100.0 }, PenEvent::Up]
        );
    }
}
