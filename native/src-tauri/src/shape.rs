//! Shapes an agent asks to draw with `lucipane draw`, in image pixels (the coordinates of
//! the `lucipane capture` screenshot), and their conversion to overlay strokes.

use crate::board::{Author, Point, Stroke};

/// Longest text label an agent may place; matches `Text` in `contract/src/board.ts`.
pub const MAX_TEXT_CHARS: usize = 500;

/// Most points a flattened path may have, to keep one drawing from flooding the board file.
pub const MAX_PATH_POINTS: usize = 4000;

/// How far (in image pixels) the flattened polyline may stray from the true curve.
const PATH_TOLERANCE: f64 = 0.5;

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
    /// A free drawing: one polyline per subpath of an SVG path, curves already flattened.
    Path { subpaths: Vec<Vec<[f64; 2]>> },
}

/// Parse SVG path data (`M 10 10 C 20 0, 40 0, 50 10 Z`, as in `<path d="...">`) and
/// flatten its curves into polylines, one per subpath. Subpaths that draw nothing (a lone
/// `M`) are dropped.
pub fn parse_svg_path(d: &str) -> Result<Vec<Vec<[f64; 2]>>, String> {
    let path = kurbo::BezPath::from_svg(d).map_err(|e| format!("invalid SVG path: {e}"))?;
    let mut subpaths: Vec<Vec<[f64; 2]>> = Vec::new();
    kurbo::flatten(path, PATH_TOLERANCE, |el| match el {
        kurbo::PathEl::MoveTo(p) => subpaths.push(vec![[p.x, p.y]]),
        kurbo::PathEl::LineTo(p) => match subpaths.last_mut() {
            Some(current) => current.push([p.x, p.y]),
            None => subpaths.push(vec![[p.x, p.y]]),
        },
        kurbo::PathEl::ClosePath => {
            if let Some(current) = subpaths.last_mut()
                && let Some(&start) = current.first()
            {
                current.push(start);
            }
        }
        // `flatten` only emits moves, lines and closes.
        _ => {}
    });
    subpaths.retain(|s| s.len() >= 2);

    let points: usize = subpaths.iter().map(Vec::len).sum();
    if points == 0 {
        return Err("the path draws nothing".into());
    }
    if points > MAX_PATH_POINTS {
        return Err(format!(
            "the path flattens to {points} points; keep it under {MAX_PATH_POINTS}"
        ));
    }
    if subpaths.iter().flatten().flatten().any(|v| !v.is_finite()) {
        return Err("the path has coordinates that are not finite numbers".into());
    }
    Ok(subpaths)
}

/// Convert to agent strokes in CSS pixels. `scale` is image pixels per CSS pixel. A path
/// becomes one freehand stroke per subpath; every other shape is a single stroke.
pub fn to_strokes(shape: &Shape, scale: f64) -> Vec<Stroke> {
    let css = |x: f64, y: f64| Point {
        x: x / scale,
        y: y / scale,
    };
    let author = Author::Agent;
    let stroke = match shape {
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
        Shape::Path { subpaths } => {
            return subpaths
                .iter()
                .map(|points| Stroke::Freehand {
                    author,
                    points: points.iter().map(|[x, y]| css(*x, *y)).collect(),
                })
                .collect();
        }
    };
    vec![stroke]
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
            to_strokes(&shape, 2.0),
            vec![Stroke::Box {
                author: Author::Agent,
                from: p(50.0, 25.0),
                to: p(70.0, 35.0)
            }]
        );
    }

    #[test]
    fn converts_an_arrow() {
        let shape = Shape::Arrow {
            from: [10.0, 20.0],
            to: [30.0, 40.0],
        };
        assert_eq!(
            to_strokes(&shape, 1.25),
            vec![Stroke::Arrow {
                author: Author::Agent,
                from: p(8.0, 16.0),
                to: p(24.0, 32.0)
            }]
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
            to_strokes(&shape, 1.5),
            vec![Stroke::Text {
                author: Author::Agent,
                at: p(20.0, 40.0),
                text: "here".into()
            }]
        );
    }

    #[test]
    fn converts_each_subpath_of_a_path_to_a_freehand_stroke() {
        let shape = Shape::Path {
            subpaths: vec![vec![[0.0, 0.0], [20.0, 0.0]], vec![[4.0, 8.0], [6.0, 10.0]]],
        };
        assert_eq!(
            to_strokes(&shape, 2.0),
            vec![
                Stroke::Freehand {
                    author: Author::Agent,
                    points: vec![p(0.0, 0.0), p(10.0, 0.0)]
                },
                Stroke::Freehand {
                    author: Author::Agent,
                    points: vec![p(2.0, 4.0), p(3.0, 5.0)]
                },
            ]
        );
    }

    #[test]
    fn parses_straight_segments_as_they_are() {
        assert_eq!(
            parse_svg_path("M 10 10 L 50 10 L 50 40").unwrap(),
            vec![vec![[10.0, 10.0], [50.0, 10.0], [50.0, 40.0]]]
        );
    }

    #[test]
    fn closes_a_subpath_back_to_its_start() {
        assert_eq!(
            parse_svg_path("M0 0 H10 V10 Z").unwrap(),
            vec![vec![[0.0, 0.0], [10.0, 0.0], [10.0, 10.0], [0.0, 0.0]]]
        );
    }

    #[test]
    fn splits_subpaths_and_drops_empty_ones() {
        let subpaths = parse_svg_path("M0 0 L5 5 M100 100 M20 20 L30 30").unwrap();
        assert_eq!(
            subpaths,
            vec![
                vec![[0.0, 0.0], [5.0, 5.0]],
                vec![[20.0, 20.0], [30.0, 30.0]]
            ]
        );
    }

    #[test]
    fn flattens_curves_close_to_the_true_shape() {
        // A circle of radius 50 around (100, 100), drawn with two arcs.
        let subpaths =
            parse_svg_path("M 50 100 A 50 50 0 1 0 150 100 A 50 50 0 1 0 50 100").unwrap();
        assert_eq!(subpaths.len(), 1);
        let points = &subpaths[0];
        assert!(
            points.len() > 16,
            "a circle needs many points, got {}",
            points.len()
        );
        for [x, y] in points {
            let r = ((x - 100.0).powi(2) + (y - 100.0).powi(2)).sqrt();
            assert!(
                (r - 50.0).abs() <= PATH_TOLERANCE + 0.01,
                "point off the circle: r={r}"
            );
        }
    }

    #[test]
    fn rejects_paths_that_are_invalid_or_draw_nothing() {
        assert!(parse_svg_path("hello").is_err());
        assert!(parse_svg_path("").is_err());
        assert!(parse_svg_path("M 10 10").is_err());
    }

    #[test]
    fn rejects_paths_with_too_many_points() {
        let zigzag: String = (0..MAX_PATH_POINTS)
            .map(|i| format!("L {i} {} ", i % 2))
            .collect();
        assert!(parse_svg_path(&format!("M 0 0 {zigzag}")).is_err());
    }
}
