//! Board strokes (CSS pixels) -> annotations in screenshot pixels
//! (contract: `Annotation` in `contract/src/capture.ts`).

use serde::Serialize;

use crate::board::{BoardSnapshot, Point, Stroke};

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum Annotation {
    Freehand {
        points: Vec<[i64; 2]>,
    },
    Box {
        x: i64,
        y: i64,
        width: i64,
        height: i64,
    },
}

/// Scale every stroke to image pixels. Freehand points are rounded and consecutive
/// duplicates dropped (they carry no information and cost tokens). Boxes are normalized
/// to a top-left corner and a non-negative size.
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
            Stroke::Freehand { points } => {
                let mut scaled: Vec<[i64; 2]> = points.iter().map(px).collect();
                scaled.dedup();
                Annotation::Freehand { points: scaled }
            }
            Stroke::Box { from, to } => {
                let [x1, y1] = px(from);
                let [x2, y2] = px(to);
                Annotation::Box {
                    x: x1.min(x2),
                    y: y1.min(y2),
                    width: (x1 - x2).abs(),
                    height: (y1 - y2).abs(),
                }
            }
        })
        .collect()
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
                points: vec![p(1.2, 2.6), p(3.0, 4.0)],
            }],
        );
        assert_eq!(
            to_annotations(&s),
            vec![Annotation::Freehand {
                points: vec![[2, 5], [6, 8]]
            }]
        );
    }

    #[test]
    fn keeps_a_single_point_stroke() {
        let s = snapshot(
            1.0,
            vec![Stroke::Freehand {
                points: vec![p(5.0, 5.0), p(5.1, 4.9)],
            }],
        );
        assert_eq!(
            to_annotations(&s),
            vec![Annotation::Freehand {
                points: vec![[5, 5]]
            }]
        );
    }

    #[test]
    fn normalizes_a_box_drawn_up_and_left() {
        let s = snapshot(
            1.0,
            vec![Stroke::Box {
                from: p(100.0, 80.0),
                to: p(40.0, 20.0),
            }],
        );
        assert_eq!(
            to_annotations(&s),
            vec![Annotation::Box {
                x: 40,
                y: 20,
                width: 60,
                height: 60
            }]
        );
    }
}
