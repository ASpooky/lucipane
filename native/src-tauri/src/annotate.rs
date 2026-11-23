//! Board strokes (CSS pixels) -> annotations in screenshot pixels
//! (contract: `Annotation` in `contract/src/capture.ts`).

use serde::Serialize;

use crate::board::{Author, BoardSnapshot, Point, Stroke};

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum Annotation {
    Freehand {
        author: Author,
        points: Vec<[i64; 2]>,
    },
    Box {
        author: Author,
        x: i64,
        y: i64,
        width: i64,
        height: i64,
    },
    Arrow {
        author: Author,
        from: [i64; 2],
        to: [i64; 2],
    },
    Text {
        author: Author,
        x: i64,
        y: i64,
        text: String,
    },
}

/// How far (in image pixels) a simplified freehand line may stray from what was drawn.
/// Small enough to keep the shape of handwriting, large enough to drop mouse jitter.
const SIMPLIFY_TOLERANCE_PX: f64 = 1.5;

/// Scale every stroke to image pixels. Freehand points are rounded, consecutive duplicates
/// dropped and the line simplified: dense mouse samples carry no extra meaning for an agent
/// and cost tokens. Boxes are normalized to a top-left corner and a non-negative size.
pub fn to_annotations(snapshot: &BoardSnapshot) -> Vec<Annotation> {
    let px = |p: &Point| -> [i64; 2] {
        [
            (p.x * snapshot.scale).round() as i64,
            (p.y * snapshot.scale).round() as i64,
        ]
    };
    snapshot
        .strokes
        .iter()
        .map(|stroke| match stroke {
            Stroke::Freehand { author, points } => {
                let mut scaled: Vec<[i64; 2]> = points.iter().map(px).collect();
                scaled.dedup();
                Annotation::Freehand {
                    author: *author,
                    points: simplify(&scaled, SIMPLIFY_TOLERANCE_PX),
                }
            }
            Stroke::Box { author, from, to } => {
                let [x1, y1] = px(from);
                let [x2, y2] = px(to);
                Annotation::Box {
                    author: *author,
                    x: x1.min(x2),
                    y: y1.min(y2),
                    width: (x1 - x2).abs(),
                    height: (y1 - y2).abs(),
                }
            }
            Stroke::Arrow { author, from, to } => Annotation::Arrow {
                author: *author,
                from: px(from),
                to: px(to),
            },
            Stroke::Text { author, at, text } => {
                let [x, y] = px(at);
                Annotation::Text {
                    author: *author,
                    x,
                    y,
                    text: text.clone(),
                }
            }
        })
        .collect()
}

/// Drop points that lie within `tolerance` pixels of the line through their neighbours
/// (Ramer–Douglas–Peucker). The first and last points are always kept.
pub fn simplify(points: &[[i64; 2]], tolerance: f64) -> Vec<[i64; 2]> {
    if points.len() <= 2 {
        return points.to_vec();
    }
    let mut keep = vec![false; points.len()];
    keep[0] = true;
    keep[points.len() - 1] = true;
    // Iterative to avoid deep recursion on long strokes.
    let mut spans = vec![(0, points.len() - 1)];
    while let Some((start, end)) = spans.pop() {
        let farthest = (start + 1..end)
            .map(|i| {
                (
                    i,
                    distance_to_segment(points[i], points[start], points[end]),
                )
            })
            .max_by(|a, b| a.1.total_cmp(&b.1));
        if let Some((i, distance)) = farthest
            && distance > tolerance
        {
            keep[i] = true;
            spans.push((start, i));
            spans.push((i, end));
        }
    }
    points
        .iter()
        .zip(keep)
        .filter_map(|(p, k)| k.then_some(*p))
        .collect()
}

/// Distance from `p` to the segment a–b. For a closed stroke (a == b) this is the
/// distance to that point, so loops keep their shape.
fn distance_to_segment(p: [i64; 2], a: [i64; 2], b: [i64; 2]) -> f64 {
    let [px, py] = p.map(|v| v as f64);
    let [ax, ay] = a.map(|v| v as f64);
    let [bx, by] = b.map(|v| v as f64);
    let (dx, dy) = (bx - ax, by - ay);
    let length_sq = dx * dx + dy * dy;
    let t = if length_sq == 0.0 {
        0.0
    } else {
        (((px - ax) * dx + (py - ay) * dy) / length_sq).clamp(0.0, 1.0)
    };
    (px - (ax + t * dx)).hypot(py - (ay + t * dy))
}

#[cfg(test)]
mod simplify_tests {
    use super::simplify;

    #[test]
    fn keeps_short_lines_as_they_are() {
        assert_eq!(simplify(&[], 2.0), Vec::<[i64; 2]>::new());
        assert_eq!(simplify(&[[1, 1]], 2.0), vec![[1, 1]]);
        assert_eq!(simplify(&[[0, 0], [9, 9]], 2.0), vec![[0, 0], [9, 9]]);
    }

    #[test]
    fn collapses_a_straight_run_to_its_ends() {
        let line: Vec<[i64; 2]> = (0..=10).map(|i| [i * 10, i * 5]).collect();
        assert_eq!(simplify(&line, 2.0), vec![[0, 0], [100, 50]]);
    }

    #[test]
    fn drops_jitter_within_the_tolerance() {
        let wobbly = [[0, 0], [10, 1], [20, -1], [30, 1], [40, 0]];
        assert_eq!(simplify(&wobbly, 2.0), vec![[0, 0], [40, 0]]);
    }

    #[test]
    fn keeps_corners() {
        let corner = [[0, 0], [5, 0], [10, 0], [10, 5], [10, 10]];
        assert_eq!(simplify(&corner, 2.0), vec![[0, 0], [10, 0], [10, 10]]);
    }

    #[test]
    fn keeps_a_closed_loop_open_at_its_start() {
        // A circle-ish loop that ends where it started must not collapse to one point.
        let lp = [[0, 0], [10, 0], [10, 10], [0, 10], [0, 0]];
        assert_eq!(simplify(&lp, 2.0), lp.to_vec());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SNAPSHOT: &str = include_str!("../../../contract/fixtures/board-snapshot.json");
    const OUTPUT: &str = include_str!("../../../contract/fixtures/capture-output.json");

    fn snapshot(scale: f64, strokes: Vec<Stroke>) -> BoardSnapshot {
        BoardSnapshot {
            contract_version: crate::board::CONTRACT_VERSION,
            scale,
            strokes,
        }
    }

    fn p(x: f64, y: f64) -> Point {
        Point { x, y }
    }

    #[test]
    fn matches_the_shared_contract_fixtures() {
        let annotations = to_annotations(&BoardSnapshot::parse(SNAPSHOT).unwrap());
        let expected: serde_json::Value = serde_json::from_str(OUTPUT).unwrap();
        assert_eq!(
            serde_json::to_value(&annotations).unwrap(),
            expected["annotations"]
        );
    }

    #[test]
    fn scales_and_rounds_freehand_points() {
        let s = snapshot(
            2.0,
            vec![Stroke::Freehand {
                author: Author::User,
                points: vec![p(1.2, 2.6), p(3.0, 4.0)],
            }],
        );
        assert_eq!(
            to_annotations(&s),
            vec![Annotation::Freehand {
                author: Author::User,
                points: vec![[2, 5], [6, 8]]
            }]
        );
    }

    #[test]
    fn keeps_a_single_point_stroke() {
        let s = snapshot(
            1.0,
            vec![Stroke::Freehand {
                author: Author::User,
                points: vec![p(5.0, 5.0), p(5.1, 4.9)],
            }],
        );
        assert_eq!(
            to_annotations(&s),
            vec![Annotation::Freehand {
                author: Author::User,
                points: vec![[5, 5]]
            }]
        );
    }

    #[test]
    fn normalizes_a_box_drawn_up_and_left() {
        let s = snapshot(
            1.0,
            vec![Stroke::Box {
                author: Author::User,
                from: p(100.0, 80.0),
                to: p(40.0, 20.0),
            }],
        );
        assert_eq!(
            to_annotations(&s),
            vec![Annotation::Box {
                author: Author::User,
                x: 40,
                y: 20,
                width: 60,
                height: 60
            }]
        );
    }
}
