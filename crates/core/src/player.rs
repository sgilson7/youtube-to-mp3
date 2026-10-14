//! The video inside the page: YouTube's embedded player, in a frame from
//! `youtube-nocookie.com`, the address YouTube gives for embeds that set no
//! cookie until the video plays.
//!
//! The page and the frame talk by `postMessage` in the widget protocol that
//! YouTube's own embed script uses: the page says it is listening and sends
//! commands; the frame answers with `infoDelivery` reports of the position,
//! the length, the state and the title. The page loads no YouTube script; it
//! passes each message across as text, and this module reads it.
//!
//! The player reports times in seconds with a fraction. The core never names
//! a float type (reference default D2), so a time is read from its decimal
//! text into whole milliseconds.

use serde_json::Value;

/// The one origin the page may frame. The page's CSP lists exactly this in
/// `frame-src`, and `tests/player_fence.rs` holds the two together.
pub const ORIGIN: &str = "https://www.youtube-nocookie.com";

/// The player's state codes, as YouTube defines them.
pub const ENDED: i64 = 0;
pub const PLAYING: i64 = 1;

/// Error codes for a video whose owner does not allow it on other sites.
const NOT_EMBEDDABLE: &[u64] = &[101, 150];

/// Percent-encodes everything but the characters a query value may hold.
fn encode(text: &str) -> String {
    text.bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => (b as char).to_string(),
            _ => format!("%{b:02X}"),
        })
        .collect()
}

/// The frame's address for a video id. `enablejsapi` turns on the widget
/// protocol, and `origin` names the page that may talk to it.
pub fn embed_address(id: &str, page_origin: &str) -> String {
    format!("{ORIGIN}/embed/{id}?enablejsapi=1&origin={}", encode(page_origin))
}

/// The message that asks the player to start reporting.
pub fn listening() -> String {
    serde_json::json!({ "event": "listening", "id": 1, "channel": "widget" }).to_string()
}

/// A command to the player: `playVideo`, `pauseVideo`, `addEventListener`.
pub fn command(func: &str, args: &[&str]) -> String {
    serde_json::json!({ "event": "command", "func": func, "args": args, "id": 1, "channel": "widget" })
        .to_string()
}

/// Seconds written as decimal text, in whole milliseconds, rounded down.
/// Negative numbers, exponents and anything else that is not plain decimal
/// read as `None`.
pub fn millis(text: &str) -> Option<u64> {
    let (whole, fraction) = text.split_once('.').unwrap_or((text, ""));
    if whole.is_empty() || !whole.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    if !fraction.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let seconds: u64 = whole.parse().ok()?;
    let mut ms = 0;
    for k in 0..3 {
        let digit = fraction.as_bytes().get(k).map_or(0, |b| u64::from(b - b'0'));
        ms = ms * 10 + digit;
    }
    seconds.checked_mul(1000)?.checked_add(ms)
}

/// A time field of a report, read without a float.
fn seconds_field(info: &Value, key: &str) -> Option<u64> {
    match info.get(key)? {
        Value::Number(n) => millis(&n.to_string()),
        _ => None,
    }
}

/// What one message from the frame said. Each report carries only what
/// changed, so every part is optional.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Report {
    pub ready: bool,
    pub state: Option<i64>,
    pub time_ms: Option<u64>,
    pub length_ms: Option<u64>,
    pub title: Option<String>,
    pub error: Option<u64>,
}

/// Reads a message the page received. A message from any origin but the
/// player's, or one that is not the player's JSON, reads as `None` and
/// changes nothing.
pub fn read(origin: &str, data: &str) -> Option<Report> {
    if origin != ORIGIN {
        return None;
    }
    let message: Value = serde_json::from_str(data).ok()?;
    let event = message.get("event")?.as_str()?;
    let info = message.get("info").unwrap_or(&Value::Null);
    let mut report = Report::default();
    match event {
        "onReady" => report.ready = true,
        "onStateChange" => report.state = info.as_i64(),
        "onError" => report.error = Some(info.as_u64().unwrap_or(0)),
        "infoDelivery" | "initialDelivery" => {
            report.state = info.get("playerState").and_then(Value::as_i64);
            report.time_ms = seconds_field(info, "currentTime");
            report.length_ms = seconds_field(info, "duration");
            report.title = info
                .get("videoData")
                .and_then(|v| v.get("title"))
                .and_then(Value::as_str)
                .map(str::to_string);
        }
        _ => return None,
    }
    Some(report)
}

/// Whether an error code means the owner keeps the video off other sites.
pub fn not_embeddable(code: u64) -> bool {
    NOT_EMBEDDABLE.contains(&code)
}

/// The player as the page last described it.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Player {
    pub id: String,
    pub ready: bool,
    pub state: Option<i64>,
    pub time_ms: u64,
    pub length_ms: u64,
    pub title: String,
}

impl Player {
    pub fn new(id: &str) -> Player {
        Player {
            id: id.to_string(),
            ..Player::default()
        }
    }

    /// Takes in a report's changes.
    pub fn apply(&mut self, report: &Report) {
        self.ready |= report.ready;
        if let Some(state) = report.state {
            self.state = Some(state);
            // A report of any state means the player is answering.
            self.ready = true;
        }
        if let Some(t) = report.time_ms {
            self.time_ms = t;
        }
        if let Some(l) = report.length_ms {
            self.length_ms = l;
        }
        if let Some(title) = &report.title {
            self.title = title.trim().to_string();
        }
    }

    /// Milliseconds of the video still to play, once its length is known.
    pub fn left_ms(&self) -> Option<u64> {
        (self.length_ms > 0).then(|| self.length_ms.saturating_sub(self.time_ms))
    }

    /// How far through the video the player is, in thousandths.
    pub fn played_per_mille(&self) -> u32 {
        if self.length_ms == 0 {
            return 0;
        }
        (self.time_ms.min(self.length_ms) * 1000 / self.length_ms) as u32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Messages as the player sent them to a page on 2026-10-14, cut down to
    // the fields this module reads (SECOND-ORDER.md R3.2).
    const INFO: &str = r#"{"event":"infoDelivery","info":{"playerState":-1,"currentTime":0,"duration":635,"videoData":{"video_id":"aqz-KE-bpKQ","author":"","title":"Big Buck Bunny 60fps 4K - Official Blender Foundation Short Film"}},"channel":"widget","id":1}"#;
    const TICK: &str = r#"{"event":"infoDelivery","info":{"currentTime":3.884592,"playbackRate":1},"channel":"widget","id":1}"#;

    #[test]
    fn the_embed_address_names_the_video_and_the_page() {
        assert_eq!(
            embed_address("aqz-KE-bpKQ", "https://sgilson7.github.io"),
            "https://www.youtube-nocookie.com/embed/aqz-KE-bpKQ?enablejsapi=1&origin=https%3A%2F%2Fsgilson7.github.io"
        );
        assert_eq!(
            embed_address("aqz-KE-bpKQ", "http://127.0.0.1:8000"),
            "https://www.youtube-nocookie.com/embed/aqz-KE-bpKQ?enablejsapi=1&origin=http%3A%2F%2F127.0.0.1%3A8000"
        );
    }

    #[test]
    fn commands_are_the_widget_protocol() {
        assert_eq!(listening(), r#"{"channel":"widget","event":"listening","id":1}"#);
        assert_eq!(
            command("addEventListener", &["onError"]),
            r#"{"args":["onError"],"channel":"widget","event":"command","func":"addEventListener","id":1}"#
        );
        assert_eq!(
            command("playVideo", &[]),
            r#"{"args":[],"channel":"widget","event":"command","func":"playVideo","id":1}"#
        );
    }

    #[test]
    fn decimal_seconds_become_whole_milliseconds() {
        assert_eq!(millis("0"), Some(0));
        assert_eq!(millis("635"), Some(635_000));
        assert_eq!(millis("3.884592"), Some(3_884));
        assert_eq!(millis("3.8"), Some(3_800));
        assert_eq!(millis("12.05"), Some(12_050));
        assert_eq!(millis("634.5666666666667"), Some(634_566));
        assert_eq!(millis("-1"), None);
        assert_eq!(millis("1e21"), None);
        assert_eq!(millis(".5"), None);
        assert_eq!(millis("2.x"), None);
        assert_eq!(millis("99999999999999999999"), None);
    }

    #[test]
    fn a_report_gives_the_state_time_length_and_title() {
        let r = read(ORIGIN, INFO).unwrap();
        assert_eq!(r.state, Some(-1));
        assert_eq!(r.time_ms, Some(0));
        assert_eq!(r.length_ms, Some(635_000));
        assert_eq!(
            r.title.as_deref(),
            Some("Big Buck Bunny 60fps 4K - Official Blender Foundation Short Film")
        );
        let tick = read(ORIGIN, TICK).unwrap();
        assert_eq!(tick.time_ms, Some(3_884));
        assert_eq!(tick.state, None);
        assert_eq!(tick.length_ms, None);
    }

    #[test]
    fn events_and_errors_are_read() {
        let ready = read(
            ORIGIN,
            r#"{"event":"onReady","info":null,"channel":"widget","id":1}"#,
        );
        assert!(ready.unwrap().ready);
        let ended = read(ORIGIN, r#"{"event":"onStateChange","info":0}"#).unwrap();
        assert_eq!(ended.state, Some(ENDED));
        let error = read(ORIGIN, r#"{"event":"onError","info":150}"#).unwrap();
        assert_eq!(error.error, Some(150));
        assert!(not_embeddable(150) && not_embeddable(101) && !not_embeddable(100));
    }

    #[test]
    fn a_message_from_another_origin_or_not_the_player_s_is_ignored() {
        assert_eq!(read("https://evil.example", INFO), None);
        assert_eq!(read("https://www.youtube.com", INFO), None);
        assert_eq!(read(ORIGIN, "[object Object]"), None);
        assert_eq!(read(ORIGIN, r#"{"event":"somethingElse"}"#), None);
        assert_eq!(read(ORIGIN, r#"{"info":1}"#), None);
    }

    #[test]
    fn reports_add_up_and_give_the_time_left() {
        let mut p = Player::new("aqz-KE-bpKQ");
        assert_eq!(p.left_ms(), None, "no length yet");
        assert_eq!(p.played_per_mille(), 0);
        p.apply(&read(ORIGIN, INFO).unwrap());
        assert!(p.ready);
        p.apply(&read(ORIGIN, TICK).unwrap());
        assert_eq!(p.state, Some(-1), "a report without a state keeps the last one");
        assert_eq!(
            p.length_ms, 635_000,
            "a report without a length keeps the last one"
        );
        // 635000 - 3884 = 631116; 3884 * 1000 / 635000 = 6.1, so 6.
        assert_eq!(p.left_ms(), Some(631_116));
        assert_eq!(p.played_per_mille(), 6);
        p.apply(&Report {
            time_ms: Some(700_000),
            ..Report::default()
        });
        assert_eq!(p.left_ms(), Some(0), "a time past the end leaves nothing");
        assert_eq!(p.played_per_mille(), 1000);
    }
}
