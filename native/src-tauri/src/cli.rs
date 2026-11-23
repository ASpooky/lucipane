//! `lucipane <command>`: the same executable doubles as a CLI for agents.
//!
//! `capture` runs entirely in this process: the overlay's strokes are on screen, so they are
//! in the screenshot, and their geometry comes from the board file the running app keeps
//! up to date. `wait` polls that board file until the user hands a drawing over, then
//! captures like `capture`. `clear` and `draw` must reach the running app; they are forwarded
//! by the single-instance plugin (see lib.rs).

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::Duration;

use serde::Serialize;

use crate::annotate::{Annotation, to_annotations};
use crate::board::{BoardSnapshot, CONTRACT_VERSION, board_file};
use crate::shape::{MAX_TEXT_CHARS, Shape, parse_svg_path};

pub const USAGE: &str = "\
usage:
  lucipane                              start the overlay
  lucipane capture [--out <file.png>]   capture the primary display and all annotations
  lucipane clear [--agent]              erase all annotations (or only the agent's)
  lucipane draw box --x X --y Y --w W --h H
  lucipane draw arrow --from X,Y --to X,Y
  lucipane draw text --x X --y Y --text TEXT
  lucipane draw path --d PATH_DATA      SVG path data, e.g. \"M 10 10 C 20 0, 40 0, 50 10 Z\"
                                        draw as the agent, in pixels of the capture image
  lucipane wait [--out <file.png>]      block until the user leaves draw mode after drawing,
                                        then print the same output as capture";

#[derive(Debug, Clone, PartialEq)]
pub enum Command {
    Capture {
        out: Option<PathBuf>,
    },
    Clear {
        agent_only: bool,
    },
    Draw(Shape),
    /// Block until the user hands a drawing over, then capture.
    Wait {
        out: Option<PathBuf>,
    },
    Help,
}

impl Command {
    /// Commands that act on the running overlay and must be forwarded to it.
    pub fn is_forwarded(&self) -> bool {
        matches!(self, Command::Clear { .. } | Command::Draw(_))
    }
}

/// Parse the arguments after the program name. `Ok(None)` means: start the overlay app.
pub fn parse(args: &[String]) -> Result<Option<Command>, String> {
    let args: Vec<&str> = args.iter().map(String::as_str).collect();
    let command = match args.as_slice() {
        [] => return Ok(None),
        ["capture"] => Command::Capture { out: None },
        ["capture", "--out", path] => Command::Capture {
            out: Some(PathBuf::from(path)),
        },
        ["clear"] => Command::Clear { agent_only: false },
        ["clear", "--agent"] => Command::Clear { agent_only: true },
        ["draw", kind, rest @ ..] => Command::Draw(parse_shape(kind, rest)?),
        ["wait"] => Command::Wait { out: None },
        ["wait", "--out", path] => Command::Wait {
            out: Some(PathBuf::from(path)),
        },
        ["help" | "--help" | "-h"] => Command::Help,
        _ => return Err(unrecognized(&args)),
    };
    Ok(Some(command))
}

fn parse_shape(kind: &str, rest: &[&str]) -> Result<Shape, String> {
    let shape = match kind {
        "box" => {
            let f = flags(rest, &["--x", "--y", "--w", "--h"])?;
            let (width, height) = (number(&f, "--w")?, number(&f, "--h")?);
            if width < 0.0 || height < 0.0 {
                return Err("--w and --h must not be negative".into());
            }
            Shape::Box {
                x: number(&f, "--x")?,
                y: number(&f, "--y")?,
                width,
                height,
            }
        }
        "arrow" => {
            let f = flags(rest, &["--from", "--to"])?;
            Shape::Arrow {
                from: pair(&f, "--from")?,
                to: pair(&f, "--to")?,
            }
        }
        "text" => {
            let f = flags(rest, &["--x", "--y", "--text"])?;
            let text = f["--text"].to_string();
            let chars = text.chars().count();
            if chars == 0 || chars > MAX_TEXT_CHARS {
                return Err(format!("--text must be 1 to {MAX_TEXT_CHARS} characters"));
            }
            Shape::Text {
                x: number(&f, "--x")?,
                y: number(&f, "--y")?,
                text,
            }
        }
        "path" => {
            let f = flags(rest, &["--d"])?;
            Shape::Path {
                subpaths: parse_svg_path(f["--d"])?,
            }
        }
        _ => {
            return Err(format!(
                "unknown shape `{kind}` (box, arrow, text or path)\n{USAGE}"
            ));
        }
    };
    Ok(shape)
}

/// Parse `--name value` pairs. Every flag in `required` must appear exactly once, and no
/// other flag is accepted.
fn flags<'a>(rest: &[&'a str], required: &[&str]) -> Result<HashMap<&'a str, &'a str>, String> {
    if !rest.len().is_multiple_of(2) {
        return Err(format!(
            "expected `--flag value` pairs, got: {}",
            rest.join(" ")
        ));
    }
    let mut found = HashMap::new();
    for pair in rest.chunks(2) {
        let (name, value) = (pair[0], pair[1]);
        if !required.contains(&name) {
            return Err(format!(
                "unknown flag {name} (expected {})",
                required.join(", ")
            ));
        }
        if found.insert(name, value).is_some() {
            return Err(format!("{name} given more than once"));
        }
    }
    if let Some(missing) = required.iter().find(|name| !found.contains_key(*name)) {
        return Err(format!("missing {missing}"));
    }
    Ok(found)
}

fn number(flags: &HashMap<&str, &str>, name: &str) -> Result<f64, String> {
    parse_number(flags[name]).ok_or_else(|| format!("{name} must be a number"))
}

/// `X,Y`
fn pair(flags: &HashMap<&str, &str>, name: &str) -> Result<[f64; 2], String> {
    let invalid = || format!("{name} must be X,Y");
    let (x, y) = flags[name].split_once(',').ok_or_else(invalid)?;
    Ok([
        parse_number(x).ok_or_else(invalid)?,
        parse_number(y).ok_or_else(invalid)?,
    ])
}

fn parse_number(s: &str) -> Option<f64> {
    s.trim().parse::<f64>().ok().filter(|n| n.is_finite())
}

fn unrecognized(args: &[&str]) -> String {
    format!("unrecognized arguments: {}\n{USAGE}", args.join(" "))
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
        Command::Capture { out } => print_capture("capture", capture(out)),
        Command::Wait { out } => print_capture("wait", wait(out)),
        // Reaching here means no overlay was running to forward the command to.
        Command::Clear { .. } | Command::Draw(_) => {
            eprintln!("lucipane: the overlay is not running");
            1
        }
    }
}

/// Print a capture as one JSON line, or the error, and return the exit code.
fn print_capture(name: &str, result: Result<CaptureOutput, String>) -> i32 {
    match result {
        Ok(output) => {
            println!(
                "{}",
                serde_json::to_string(&output).expect("output serializes")
            );
            0
        }
        Err(e) => {
            eprintln!("lucipane {name}: {e}");
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

/// How often `wait` reads the board file. The file is tiny, so this is cheap.
const WAIT_POLL: Duration = Duration::from_millis(250);

fn wait(out: Option<PathBuf>) -> Result<CaptureOutput, String> {
    let file = board_file().ok_or("no data directory on this system")?;
    wait_for_submission(|| read_submission(&file), || std::thread::sleep(WAIT_POLL))?;
    capture(out)
}

/// Read the submission count from the board file. The overlay writes the file at launch and
/// removes it on exit, so a missing file means it is not running (or just stopped).
fn read_submission(file: &Path) -> Result<u64, String> {
    let json = std::fs::read_to_string(file)
        .map_err(|_| "the overlay is not running (no board file)".to_string())?;
    Ok(BoardSnapshot::parse(&json)?.submission)
}

/// Read the submission count, then keep reading (with `pause` in between) until it differs
/// from the first reading. Any difference counts, not just an increase: a restarted overlay
/// starts again from 0. A failed read ends the wait with that error.
fn wait_for_submission(
    mut read: impl FnMut() -> Result<u64, String>,
    mut pause: impl FnMut(),
) -> Result<(), String> {
    let start = read()?;
    loop {
        pause();
        if read()? != start {
            return Ok(());
        }
    }
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
    use crate::board::{Author, Point, Stroke};

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
        assert_eq!(
            parse(&args(&["clear"])),
            Ok(Some(Command::Clear { agent_only: false }))
        );
        assert_eq!(
            parse(&args(&["clear", "--agent"])),
            Ok(Some(Command::Clear { agent_only: true }))
        );
        assert_eq!(parse(&args(&["help"])), Ok(Some(Command::Help)));
        assert_eq!(parse(&args(&["--help"])), Ok(Some(Command::Help)));
    }

    #[test]
    fn parses_a_path_drawing() {
        assert_eq!(
            parse(&args(&["draw", "path", "--d", "M 0 0 L 10 0 M 5 5 L 5 9"])),
            Ok(Some(Command::Draw(Shape::Path {
                subpaths: vec![vec![[0.0, 0.0], [10.0, 0.0]], vec![[5.0, 5.0], [5.0, 9.0]]]
            })))
        );
        assert!(parse(&args(&["draw", "path", "--d", "nonsense"])).is_err());
        assert!(parse(&args(&["draw", "path"])).is_err());
    }

    #[test]
    fn parses_wait() {
        assert_eq!(
            parse(&args(&["wait"])),
            Ok(Some(Command::Wait { out: None }))
        );
        assert_eq!(
            parse(&args(&["wait", "--out", "shot.png"])),
            Ok(Some(Command::Wait {
                out: Some(PathBuf::from("shot.png"))
            }))
        );
        assert!(parse(&args(&["wait", "--out"])).is_err());
    }

    #[test]
    fn rejects_unknown_commands_and_flags() {
        assert!(parse(&args(&["draw"])).is_err());
        assert!(parse(&args(&["capture", "--verbose"])).is_err());
        assert!(parse(&args(&["capture", "--out"])).is_err());
        assert!(parse(&args(&["clear", "now"])).is_err());
    }

    #[test]
    fn parses_draw_box() {
        assert_eq!(
            parse(&args(&[
                "draw", "box", "--x", "10", "--y", "20.5", "--w", "30", "--h", "40"
            ])),
            Ok(Some(Command::Draw(Shape::Box {
                x: 10.0,
                y: 20.5,
                width: 30.0,
                height: 40.0
            })))
        );
    }

    #[test]
    fn parses_draw_arrow_in_any_flag_order() {
        assert_eq!(
            parse(&args(&[
                "draw", "arrow", "--to", "30,40", "--from", "10,20"
            ])),
            Ok(Some(Command::Draw(Shape::Arrow {
                from: [10.0, 20.0],
                to: [30.0, 40.0]
            })))
        );
    }

    #[test]
    fn parses_draw_text() {
        assert_eq!(
            parse(&args(&[
                "draw",
                "text",
                "--x",
                "5",
                "--y",
                "6",
                "--text",
                "is this it?"
            ])),
            Ok(Some(Command::Draw(Shape::Text {
                x: 5.0,
                y: 6.0,
                text: "is this it?".into()
            })))
        );
    }

    #[test]
    fn rejects_invalid_draw_arguments() {
        let bad: &[&[&str]] = &[
            &["draw", "circle", "--x", "1"],
            &["draw", "box", "--x", "1", "--y", "2", "--w", "3"],
            &[
                "draw", "box", "--x", "1", "--y", "2", "--w", "-3", "--h", "4",
            ],
            &[
                "draw", "box", "--x", "a", "--y", "2", "--w", "3", "--h", "4",
            ],
            &[
                "draw", "box", "--x", "1", "--x", "1", "--y", "2", "--w", "3", "--h", "4",
            ],
            &[
                "draw", "box", "--x", "1", "--y", "2", "--w", "3", "--h", "4", "--z", "5",
            ],
            &["draw", "arrow", "--from", "1", "--to", "2,3"],
            &["draw", "arrow", "--from", "1,2"],
            &["draw", "text", "--x", "1", "--y", "2", "--text", ""],
            &[
                "draw", "box", "--x", "1", "--y", "2", "--w", "NaN", "--h", "4",
            ],
        ];
        for case in bad {
            assert!(parse(&args(case)).is_err(), "should reject {case:?}");
        }
        let long = "x".repeat(MAX_TEXT_CHARS + 1);
        assert!(
            parse(&args(&[
                "draw", "text", "--x", "1", "--y", "2", "--text", &long
            ]))
            .is_err()
        );
    }

    #[test]
    fn forwards_only_commands_for_the_running_overlay() {
        assert!(Command::Clear { agent_only: false }.is_forwarded());
        assert!(
            Command::Draw(Shape::Text {
                x: 0.0,
                y: 0.0,
                text: "a".into()
            })
            .is_forwarded()
        );
        assert!(!Command::Capture { out: None }.is_forwarded());
        assert!(!Command::Help.is_forwarded());
        assert!(!Command::Wait { out: None }.is_forwarded());
    }

    /// Feeds `wait_for_submission` a scripted sequence of readings.
    fn run_wait(readings: &[Result<u64, &str>]) -> (Result<(), String>, usize) {
        let mut readings = readings.iter().map(|r| r.map_err(str::to_string));
        let mut pauses = 0;
        let result = wait_for_submission(
            || readings.next().expect("read past the scripted readings"),
            || pauses += 1,
        );
        (result, pauses)
    }

    #[test]
    fn wait_returns_once_the_submission_count_changes() {
        assert_eq!(run_wait(&[Ok(3), Ok(3), Ok(3), Ok(4)]), (Ok(()), 3));
    }

    #[test]
    fn wait_treats_a_reset_count_as_a_change() {
        // A restarted overlay starts counting from 0 again.
        assert_eq!(run_wait(&[Ok(5), Ok(0)]), (Ok(()), 1));
    }

    #[test]
    fn wait_fails_when_the_board_cannot_be_read() {
        assert_eq!(run_wait(&[Err("no board")]), (Err("no board".into()), 0));
        assert_eq!(
            run_wait(&[Ok(1), Ok(1), Err("gone")]),
            (Err("gone".into()), 2)
        );
    }

    #[test]
    fn builds_output_with_annotations_from_the_board() {
        let board = BoardSnapshot {
            contract_version: CONTRACT_VERSION,
            scale: 2.0,
            submission: 0,
            strokes: vec![Stroke::Box {
                author: Author::User,
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
                author: Author::User,
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
