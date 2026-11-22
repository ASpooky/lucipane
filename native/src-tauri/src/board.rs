//! The board file (contract: `contract/src/snapshot.ts`).
//! The overlay's WebView sends a snapshot whenever the finished strokes change; we write it
//! to disk so the `lucipane capture` CLI, a separate process, can read it.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// Must match `CONTRACT_VERSION` in `contract/src/version.ts`.
pub const CONTRACT_VERSION: u32 = 2;

/// Must match `identifier` in tauri.conf.json.
const APP_IDENTIFIER: &str = "dev.aspooky.lucipane";

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

/// Who drew a stroke.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Author {
    User,
    Agent,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum Stroke {
    Freehand {
        author: Author,
        points: Vec<Point>,
    },
    Box {
        author: Author,
        from: Point,
        to: Point,
    },
    /// The head is at `to`.
    Arrow {
        author: Author,
        from: Point,
        to: Point,
    },
    /// `at` is the top-left corner of the text.
    Text {
        author: Author,
        at: Point,
        text: String,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BoardSnapshot {
    pub contract_version: u32,
    /// Physical (screenshot) pixels per CSS pixel.
    pub scale: f64,
    /// Finished strokes in CSS pixels.
    pub strokes: Vec<Stroke>,
}

impl BoardSnapshot {
    pub fn empty(scale: f64) -> Self {
        Self {
            contract_version: CONTRACT_VERSION,
            scale,
            strokes: Vec::new(),
        }
    }

    /// Parse a board file, rejecting other contract versions and invalid scales.
    pub fn parse(json: &str) -> Result<Self, String> {
        let snapshot: Self = serde_json::from_str(json).map_err(|e| e.to_string())?;
        if snapshot.contract_version != CONTRACT_VERSION {
            return Err(format!(
                "board file has contract version {}, expected {CONTRACT_VERSION}",
                snapshot.contract_version
            ));
        }
        let scale_is_valid = snapshot.scale.is_finite() && snapshot.scale > 0.0;
        if !scale_is_valid {
            return Err(format!("invalid scale {}", snapshot.scale));
        }
        Ok(snapshot)
    }
}

/// `<OS data dir>/dev.aspooky.lucipane/board.json`, e.g. `%APPDATA%\dev.aspooky.lucipane\board.json`.
pub fn board_file() -> Option<PathBuf> {
    dirs::data_dir().map(|dir| dir.join(APP_IDENTIFIER).join("board.json"))
}

/// Write via a temporary file and rename, so a reader never sees a half-written file.
pub fn write_atomically(path: &Path, contents: &str) -> std::io::Result<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, contents)?;
    std::fs::rename(&tmp, path)
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE: &str = include_str!("../../../contract/fixtures/board-snapshot.json");

    #[test]
    fn parses_the_shared_contract_fixture() {
        let snapshot = BoardSnapshot::parse(FIXTURE).unwrap();
        assert_eq!(snapshot.scale, 1.5);
        assert_eq!(snapshot.strokes.len(), 4);
        assert_eq!(
            snapshot.strokes[1],
            Stroke::Box {
                author: Author::User,
                from: Point { x: 30.0, y: 40.0 },
                to: Point { x: 10.0, y: 0.0 }
            }
        );
        assert_eq!(
            snapshot.strokes[3],
            Stroke::Text {
                author: Author::Agent,
                at: Point { x: 10.4, y: 20.2 },
                text: "is this it?".into()
            }
        );
    }

    #[test]
    fn round_trips_through_json() {
        let snapshot = BoardSnapshot::parse(FIXTURE).unwrap();
        let json = serde_json::to_string(&snapshot).unwrap();
        assert_eq!(BoardSnapshot::parse(&json).unwrap(), snapshot);
    }

    #[test]
    fn rejects_another_contract_version() {
        let json = r#"{"contractVersion":999,"scale":1,"strokes":[]}"#;
        assert!(BoardSnapshot::parse(json).is_err());
    }

    #[test]
    fn rejects_a_non_positive_scale() {
        let json = format!(r#"{{"contractVersion":{CONTRACT_VERSION},"scale":0,"strokes":[]}}"#);
        let json = json.as_str();
        assert!(BoardSnapshot::parse(json).is_err());
    }

    #[test]
    fn empty_has_no_strokes_and_the_current_version() {
        let snapshot = BoardSnapshot::empty(2.0);
        assert_eq!(snapshot.contract_version, CONTRACT_VERSION);
        assert_eq!(snapshot.scale, 2.0);
        assert!(snapshot.strokes.is_empty());
    }

    #[test]
    fn writes_atomically_and_replaces_existing_content() {
        let dir = std::env::temp_dir().join(format!("lucipane-test-{}", std::process::id()));
        let path = dir.join("board.json");
        write_atomically(&path, "first").unwrap();
        write_atomically(&path, "second").unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "second");
        assert!(!path.with_extension("json.tmp").exists());
        std::fs::remove_dir_all(dir).unwrap();
    }
}
