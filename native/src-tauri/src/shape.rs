//! Shapes an agent asks to draw with `lucipane draw`, in image pixels (the coordinates of
//! the `lucipane capture` screenshot), and their conversion to overlay strokes.

use crate::board::{Author, Point, Stroke};

/// Longest text label an agent may place; matches `Text` in `contract/src/board.ts`.
pub const MAX_TEXT_CHARS: usize = 500;

#[derive(Debug, Clone, PartialEq)]
pub enum Shape {
    Box {
        x: f64,
        y: f64,
        width: f64,
        height: f64,
    },
    /// The head is at `to`.
    Arrow { from: [f64; 2], to: [f64; 2] },
    /// `x`, `y` is the top-left corner.
    Text { x: f64, y: f64, text: String },
}

/// Convert to an agent stroke in CSS pixels. `scale` is image pixels per CSS pixel.
pub fn to_stroke(shape: &Shape, scale: f64) -> Stroke {
    let css = |x: f64, y: f64| Point {
        x: x / scale,
        y: y / scale,
    };
    let author = Author::Agent;
    match shape {
        Shape::Box {
            x,
            y,
            width,
            height,
        } => Stroke::Box {
            author,
            from: css(*x, *y),
            to: css(x + width, y + height),
        },
        Shape::Arrow { from, to } => Stroke::Arrow {
            author,
            from: css(from[0], from[1]),
            to: css(to[0], to[1]),
        },
        Shape::Text { x, y, text } => Stroke::Text {
            author,
            at: css(*x, *y),
            text: text.clone(),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(x: f64, y: f64) -> Point {
        Point { x, y }
    }

    #[test]
    fn converts_a_box_to_css_pixels() {
        let shape = Shape::Box {
            x: 100.0,
            y: 50.0,
            width: 40.0,
            height: 20.0,
        };
        assert_eq!(
            to_stroke(&shape, 2.0),
            Stroke::Box {
                author: Author::Agent,
                from: p(50.0, 25.0),
                to: p(70.0, 35.0)
            }
        );
    }

    #[test]
    fn converts_an_arrow() {
        let shape = Shape::Arrow {
            from: [10.0, 20.0],
            to: [30.0, 40.0],
        };
        assert_eq!(
            to_stroke(&shape, 1.25),
            Stroke::Arrow {
                author: Author::Agent,
                from: p(8.0, 16.0),
                to: p(24.0, 32.0)
            }
        );
    }

    #[test]
    fn converts_text() {
        let shape = Shape::Text {
            x: 30.0,
            y: 60.0,
            text: "here".into(),
        };
        assert_eq!(
            to_stroke(&shape, 1.5),
            Stroke::Text {
                author: Author::Agent,
                at: p(20.0, 40.0),
                text: "here".into()
            }
        );
    }
}
