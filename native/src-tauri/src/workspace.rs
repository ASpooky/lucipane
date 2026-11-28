//! Which virtual desktop the user is on, so each one gets its own pane: a macOS Space
//! (full-screen windows get one each) or a GlazeWM workspace on Windows. Without either, there
//! is a single workspace.

use std::time::Duration;

pub const DEFAULT: &str = "default";

const POLL: Duration = Duration::from_millis(250);

/// Call `on_change` with the new workspace whenever the user moves to another one. Spawns a
/// thread that runs for the life of the app.
// `Source` is a unit struct except on Windows, where it holds the GlazeWM connection.
#[allow(clippy::default_constructed_unit_structs)]
pub fn watch(on_change: impl Fn(String) + Send + 'static) {
    std::thread::spawn(move || {
        let mut source = Source::default();
        let mut last = source.current();
        loop {
            std::thread::sleep(POLL);
            let now = source.current();
            if now != last {
                on_change(now.clone());
                last = now;
            }
        }
    });
}

#[allow(clippy::default_constructed_unit_structs)]
pub fn current() -> String {
    Source::default().current()
}

#[cfg(target_os = "macos")]
#[derive(Default)]
struct Source;

#[cfg(target_os = "macos")]
impl Source {
    fn current(&mut self) -> String {
        // Private SkyLight calls (re-exported by CoreGraphics); there is no public API for the
        // active Space. yabai and Hammerspoon rely on the same pair.
        #[link(name = "CoreGraphics", kind = "framework")]
        unsafe extern "C" {
            fn CGSMainConnectionID() -> i32;
            fn CGSGetActiveSpace(connection: i32) -> u64;
        }
        // SAFETY: both only read window server state for this process's connection.
        let space = unsafe { CGSGetActiveSpace(CGSMainConnectionID()) };
        if space == 0 {
            DEFAULT.to_string()
        } else {
            format!("space-{space}")
        }
    }
}

#[cfg(windows)]
#[derive(Default)]
struct Source {
    glazewm: Option<glazewm::Client>,
}

#[cfg(windows)]
impl Source {
    fn current(&mut self) -> String {
        if self.glazewm.is_none() {
            self.glazewm = glazewm::Client::connect();
        }
        let Some(client) = self.glazewm.as_mut() else {
            return DEFAULT.to_string();
        };
        match client.focused_workspace() {
            Some(name) => format!("glazewm-{name}"),
            None => {
                // GlazeWM quit or restarted: reconnect on the next poll.
                self.glazewm = None;
                DEFAULT.to_string()
            }
        }
    }
}

#[cfg(not(any(windows, target_os = "macos")))]
#[derive(Default)]
struct Source;

#[cfg(not(any(windows, target_os = "macos")))]
impl Source {
    fn current(&mut self) -> String {
        DEFAULT.to_string()
    }
}

/// GlazeWM's IPC: a WebSocket on localhost:6123 answering text commands with JSON.
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

    #[cfg(windows)]
    pub struct Client(tungstenite::WebSocket<std::net::TcpStream>);

    #[cfg(windows)]
    impl Client {
        pub fn connect() -> Option<Self> {
            let stream = std::net::TcpStream::connect(("127.0.0.1", 6123)).ok()?;
            stream
                .set_read_timeout(Some(std::time::Duration::from_secs(1)))
                .ok()?;
            let (socket, _) = tungstenite::client("ws://localhost:6123", stream).ok()?;
            Some(Self(socket))
        }

        pub fn focused_workspace(&mut self) -> Option<String> {
            self.0.send("query workspaces".into()).ok()?;
            // Replies come in order; skip anything that is not ours.
            loop {
                let message = self.0.read().ok()?;
                let Ok(text) = message.to_text() else {
                    continue;
                };
                if let Some(name) = focused_workspace_name(text) {
                    return Some(name);
                }
                if text.contains("\"clientMessage\":\"query workspaces\"") {
                    return None;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::glazewm::focused_workspace_name;

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
