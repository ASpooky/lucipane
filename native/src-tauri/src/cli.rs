//! `lucipane <command>`: the same executable doubles as a CLI for agents.
//!
//!   lucipane capture [--out <file.png>]   screenshot + annotations as JSON on stdout
//!   lucipane clear                        erase the overlay (forwarded to the running app)
//!
//! `capture` runs entirely in this process: the overlay's strokes are on screen, so they are
//! in the screenshot, and their geometry comes from the board file the running app keeps
//! up to date. `clear` must reach the running app; it is forwarded by the single-instance
//! plugin (see lib.rs).

use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::annotate::{Annotation, to_annotations};
use crate::board::{BoardSnapshot, CONTRACT_VERSION, board_file};

pub const USAGE: &str = "\
usage:
  lucipane                              start the overlay
  lucipane capture [--out <file.png>]   capture the primary display and the user's annotations
  lucipane clear                        erase all annotations in the running overlay";

#[derive(Debug, Clone, PartialEq)]
pub enum Command {
    Capture { out: Option<PathBuf> },
    Clear,
    Help,
}

/// Parse the arguments after the program name. `Ok(None)` means: start the overlay app.
pub fn parse(args: &[String]) -> Result<Option<Command>, String> {
    let args: Vec<&str> = args.iter().map(String::as_str).collect();
    match args.as_slice() {
        [] => Ok(None),
        ["capture"] => Ok(Some(Command::Capture { out: None })),
        ["capture", "--out", path] => Ok(Some(Command::Capture {
            out: Some(PathBuf::from(path)),
        })),
        ["clear"] => Ok(Some(Command::Clear)),
        ["help" | "--help" | "-h"] => Ok(Some(Command::Help)),
        _ => Err(format!(
            "unrecognized arguments: {}\n{USAGE}",
            args.join(" ")
        )),
    }
}

/// Contract: `CaptureOutput` in `contract/src/capture.ts`.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CaptureOutput {
    pub contract_version: u32,
    pub image: ImageInfo,
    pub annotations: Vec<Annotation>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ImageInfo {
    pub path: String,
    pub width: u32,
    pub height: u32,
}

/// Build the output. Without a board (app not running, or nothing drawn yet) there are no
/// annotations.
pub fn capture_output(
    path: &Path,
    width: u32,
    height: u32,
    board: Option<&BoardSnapshot>,
) -> CaptureOutput {
    CaptureOutput {
        contract_version: CONTRACT_VERSION,
        image: ImageInfo {
            path: path.display().to_string(),
            width,
            height,
        },
        annotations: board.map(to_annotations).unwrap_or_default(),
    }
}

/// Run a CLI command and return the process exit code.
pub fn run(command: Command) -> i32 {
    match command {
        Command::Help => {
            println!("{USAGE}");
            0
        }
        Command::Capture { out } => match capture(out) {
            Ok(output) => {
                println!(
                    "{}",
                    serde_json::to_string(&output).expect("output serializes")
                );
                0
            }
            Err(e) => {
                eprintln!("lucipane capture: {e}");
                1
            }
        },
        // Reaching here means no overlay was running to forward the command to.
        Command::Clear => {
            eprintln!("lucipane clear: the overlay is not running");
            1
        }
    }
}

fn capture(out: Option<PathBuf>) -> Result<CaptureOutput, String> {
    let path = match out {
        Some(path) => path,
        None => default_capture_path()?,
    };
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)
            .map_err(|e| format!("cannot create {}: {e}", dir.display()))?;
    }

    let monitor = xcap::Monitor::all()
        .map_err(|e| format!("cannot list displays: {e}"))?
        .into_iter()
        .find(|m| m.is_primary().unwrap_or(false))
        .ok_or("no primary display found")?;
    let image = monitor
        .capture_image()
        .map_err(|e| format!("screen capture failed: {e}"))?;
    image
        .save(&path)
        .map_err(|e| format!("cannot write {}: {e}", path.display()))?;

    // A missing or unreadable board file just means no annotations.
    let board = board_file()
        .and_then(|file| std::fs::read_to_string(file).ok())
        .and_then(|json| BoardSnapshot::parse(&json).ok());
    let path = std::path::absolute(&path).unwrap_or(path);
    Ok(capture_output(
        &path,
        image.width(),
        image.height(),
        board.as_ref(),
    ))
}

/// `<temp dir>/lucipane/capture-<unix millis>.png`
fn default_capture_path() -> Result<PathBuf, String> {
    let millis = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_millis();
    Ok(std::env::temp_dir()
        .join("lucipane")
        .join(format!("capture-{millis}.png")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::board::{Point, Stroke};

    fn args(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn no_arguments_starts_the_app() {
        assert_eq!(parse(&args(&[])), Ok(None));
    }

    #[test]
    fn parses_capture() {
        assert_eq!(
            parse(&args(&["capture"])),
            Ok(Some(Command::Capture { out: None }))
        );
        assert_eq!(
            parse(&args(&["capture", "--out", "a.png"])),
            Ok(Some(Command::Capture {
                out: Some(PathBuf::from("a.png"))
            }))
        );
    }

    #[test]
    fn parses_clear_and_help() {
        assert_eq!(parse(&args(&["clear"])), Ok(Some(Command::Clear)));
        assert_eq!(parse(&args(&["help"])), Ok(Some(Command::Help)));
        assert_eq!(parse(&args(&["--help"])), Ok(Some(Command::Help)));
    }

    #[test]
    fn rejects_unknown_commands_and_flags() {
        assert!(parse(&args(&["draw"])).is_err());
        assert!(parse(&args(&["capture", "--verbose"])).is_err());
        assert!(parse(&args(&["capture", "--out"])).is_err());
        assert!(parse(&args(&["clear", "now"])).is_err());
    }

    #[test]
    fn builds_output_with_annotations_from_the_board() {
        let board = BoardSnapshot {
            contract_version: CONTRACT_VERSION,
            scale: 2.0,
            strokes: vec![Stroke::Box {
                from: Point { x: 0.0, y: 0.0 },
                to: Point { x: 5.0, y: 5.0 },
            }],
        };
        let output = capture_output(Path::new("/tmp/c.png"), 100, 50, Some(&board));
        assert_eq!(output.contract_version, CONTRACT_VERSION);
        assert_eq!(
            output.image,
            ImageInfo {
                path: "/tmp/c.png".into(),
                width: 100,
                height: 50
            }
        );
        assert_eq!(
            output.annotations,
            vec![Annotation::Box {
                x: 0,
                y: 0,
                width: 10,
                height: 10
            }]
        );
    }

    #[test]
    fn builds_output_without_a_board() {
        let output = capture_output(Path::new("c.png"), 1, 1, None);
        assert!(output.annotations.is_empty());
    }

    #[test]
    fn serializes_to_the_contract_shape() {
        let output = capture_output(Path::new("c.png"), 1, 1, None);
        let json = serde_json::to_value(&output).unwrap();
        assert_eq!(
            json,
            serde_json::json!({
                "contractVersion": CONTRACT_VERSION,
                "image": { "path": "c.png", "width": 1, "height": 1 },
                "annotations": []
            })
        );
    }
}
