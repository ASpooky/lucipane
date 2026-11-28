//! Which virtual desktop the user is on, so each one gets its own pane: a macOS Space
//! (full-screen windows get one each) or a GlazeWM workspace on Windows. Without either, there
//! is a single workspace.
//!
//! The overlay is one window over every desktop, so after a switch it shows the pane of the
//! desktop just left until the switch is noticed. The switch must be noticed quickly.

pub const DEFAULT: &str = "default";

/// The workspace the user is on now.
pub fn current() -> String {
    probe().unwrap_or_else(|| DEFAULT.to_string())
}

/// Call `on_change` with the new workspace whenever the user moves to another one. Spawns a
/// thread that runs for the life of the app.
pub fn watch(on_change: impl Fn(String) + Send + 'static) {
    std::thread::spawn(move || {
        let mut last = current();
        follow(&mut |now: String| {
            if now != last {
                on_change(now.clone());
                last = now;
            }
        });
    });
}

#[cfg(target_os = "macos")]
fn probe() -> Option<String> {
    // Private SkyLight calls (re-exported by CoreGraphics); there is no public API for the
    // active Space. yabai and Hammerspoon rely on the same pair.
    #[link(name = "CoreGraphics", kind = "framework")]
    unsafe extern "C" {
        fn CGSMainConnectionID() -> i32;
        fn CGSGetActiveSpace(connection: i32) -> u64;
    }
    // SAFETY: both only read window server state for this process's connection.
    let space = unsafe { CGSGetActiveSpace(CGSMainConnectionID()) };
    // 0: no answer right now (seen mid-switch); keep the current pane rather than flash another.
    (space != 0).then(|| format!("space-{space}"))
}

/// macOS: poll. Asking for the active Space is cheap, and a short interval keeps the pane of
/// the Space just left from lingering.
#[cfg(target_os = "macos")]
fn follow(report: &mut impl FnMut(String)) {
    const POLL: std::time::Duration = std::time::Duration::from_millis(50);
    loop {
        std::thread::sleep(POLL);
        if let Some(now) = probe() {
            report(now);
        }
    }
}

#[cfg(windows)]
fn probe() -> Option<String> {
    let mut client = glazewm::Client::connect(Some(glazewm::QUERY_TIMEOUT))?;
    client
        .focused_workspace()
        .ok()
        .flatten()
        .map(|name| glazewm::pane_id(&name))
}

/// Windows: follow GlazeWM's events, so the pane changes as soon as the workspace does. While
/// GlazeWM is not running (or restarting), the current pane stays.
#[cfg(windows)]
fn follow(report: &mut impl FnMut(String)) {
    const RECONNECT: std::time::Duration = std::time::Duration::from_secs(1);
    loop {
        glazewm::follow(&mut |name| report(glazewm::pane_id(&name)));
        std::thread::sleep(RECONNECT);
    }
}

#[cfg(not(any(windows, target_os = "macos")))]
fn probe() -> Option<String> {
    None
}

#[cfg(not(any(windows, target_os = "macos")))]
fn follow(_report: &mut impl FnMut(String)) {}

/// GlazeWM's IPC: a WebSocket on localhost:6123 answering text commands with JSON, and
/// streaming events to subscribers.
#[cfg(any(windows, test))]
mod glazewm {
    use serde_json::Value;

    /// The `name` of the workspace with `hasFocus` in a reply to `query workspaces`.
    pub fn focused_workspace_name(reply: &str) -> Option<String> {
        let reply: Value = serde_json::from_str(reply).ok()?;
        if reply.get("messageType")?.as_str()? != "client_response" {
            return None;
        }
        reply
            .pointer("/data/workspaces")?
            .as_array()?
            .iter()
            .find(|w| w.get("hasFocus").and_then(Value::as_bool) == Some(true))?
            .get("name")?
            .as_str()
            .map(str::to_string)
    }

    /// Whether a message is an event after which the focused workspace may be another one.
    pub fn may_switch_workspace(message: &str) -> bool {
        serde_json::from_str::<Value>(message).is_ok_and(|m| {
            matches!(
                m.pointer("/data/eventType").and_then(Value::as_str),
                Some("workspace_activated" | "focus_changed")
            )
        })
    }

    #[cfg(windows)]
    pub use client::*;

    #[cfg(windows)]
    mod client {
        use std::net::TcpStream;
        use std::time::Duration;

        use super::{focused_workspace_name, may_switch_workspace};

        pub const QUERY_TIMEOUT: Duration = Duration::from_secs(1);
        const SUBSCRIBE: &str = "sub -e workspace_activated focus_changed";

        pub fn pane_id(workspace: &str) -> String {
            format!("glazewm-{workspace}")
        }

        pub struct Client(tungstenite::WebSocket<TcpStream>);

        impl Client {
            /// `None` when GlazeWM is not running. Without a timeout, reads wait for as long as
            /// it takes (for events).
            pub fn connect(read_timeout: Option<Duration>) -> Option<Self> {
                let stream = TcpStream::connect(("127.0.0.1", 6123)).ok()?;
                stream.set_read_timeout(read_timeout).ok()?;
                let (socket, _) = tungstenite::client("ws://localhost:6123", stream).ok()?;
                Some(Self(socket))
            }

            /// `Ok(None)` when GlazeWM answers but no workspace has focus; `Err` when the
            /// connection is gone.
            pub fn focused_workspace(&mut self) -> Result<Option<String>, ()> {
                self.0.send("query workspaces".into()).map_err(drop)?;
                // Replies come in order; skip anything that is not ours.
                loop {
                    let message = self.0.read().map_err(drop)?;
                    let Ok(text) = message.to_text() else {
                        continue;
                    };
                    if text.contains("\"clientMessage\":\"query workspaces\"") {
                        return Ok(focused_workspace_name(text));
                    }
                }
            }
        }

        /// Report the focused workspace now and after every event that may change it. Returns
        /// when GlazeWM is not running or goes away.
        pub fn follow(report: &mut impl FnMut(String)) {
            let (Some(mut events), Some(mut queries)) =
                (Client::connect(None), Client::connect(Some(QUERY_TIMEOUT)))
            else {
                return;
            };
            if events.0.send(SUBSCRIBE.into()).is_err() {
                return;
            }
            // With no workspace focused (focus on a window GlazeWM does not manage), stay put.
            let mut check = || match queries.focused_workspace() {
                Ok(name) => {
                    name.map(&mut *report);
                    true
                }
                Err(()) => false,
            };
            if !check() {
                return;
            }
            while let Ok(message) = events.0.read() {
                if message.to_text().is_ok_and(may_switch_workspace) && !check() {
                    return;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::glazewm::{focused_workspace_name, may_switch_workspace};

    #[test]
    fn workspace_and_focus_events_may_switch_the_workspace() {
        // Shapes as GlazeWM 3.10 sends them on a workspace switch.
        let activated = r#"{"messageType":"event_subscription","data":{"eventType":"workspace_activated",
            "activatedWorkspace":{"type":"workspace","name":"2"}},"subscriptionId":"s","success":true}"#;
        let focus = r#"{"messageType":"event_subscription","data":{"eventType":"focus_changed",
            "focusedContainer":{"type":"window","processName":"chrome"}},"subscriptionId":"s","success":true}"#;
        assert!(may_switch_workspace(activated));
        assert!(may_switch_workspace(focus));
    }

    #[test]
    fn other_messages_do_not_switch_the_workspace() {
        let ack = r#"{"messageType":"client_response","clientMessage":"sub -e focus_changed",
            "data":{"subscriptionId":"s"},"success":true,"error":null}"#;
        let deactivated = r#"{"messageType":"event_subscription","data":{"eventType":"workspace_deactivated",
            "deactivatedName":"2"},"subscriptionId":"s","success":true}"#;
        assert!(!may_switch_workspace(ack));
        assert!(!may_switch_workspace(deactivated));
        assert!(!may_switch_workspace("not json"));
    }

    #[test]
    fn picks_the_focused_glazewm_workspace() {
        let reply = r#"{"messageType":"client_response","clientMessage":"query workspaces",
            "data":{"workspaces":[
                {"type":"workspace","name":"1","hasFocus":false,"isDisplayed":true},
                {"type":"workspace","name":"2","hasFocus":true,"isDisplayed":true}]},
            "success":true,"error":null}"#;
        assert_eq!(focused_workspace_name(reply), Some("2".to_string()));
    }

    #[test]
    fn ignores_events_and_failed_queries() {
        let event = r#"{"messageType":"event_subscription","data":{"eventType":"focus_changed"}}"#;
        assert_eq!(focused_workspace_name(event), None);
        let failed = r#"{"messageType":"client_response","clientMessage":"query workspaces",
            "data":null,"success":false,"error":"boom"}"#;
        assert_eq!(focused_workspace_name(failed), None);
    }
}
