//! The door into the core. The shim holds one `Tool` and forwards each call.
//!
//! The core describes the screen as a view model (`Ui`): its name, title,
//! panels and their fields and actions, and the glossary's definitions. The
//! page builds the DOM from that description without knowing what a panel
//! means. Settings, screens and flags all come back through one function,
//! `act(name, value)`; an unknown action is an error. (After Peter's
//! Mandible.)
//!
//! The template keeps its state as JSON the page hands back on each call.
//! This tool holds a running MP3 encoder, which is not JSON, so the state
//! lives in `Tool` and the shim holds the `Tool` (SECOND-ORDER.md R1.4).
//! Sound comes in through `feed`, not `act`, because it arrives several
//! hundred times a second as bytes.
//!
//! The page carries out what the core asks for by name in `Ui::effects`
//! (open the tab picker, close it, save the file) and reports back through
//! `act`. It decides none of them.

use serde::{Deserialize, Serialize};

use crate::format;
use crate::glossary;
use crate::id3;
use crate::link::{self, LinkError};
use crate::page::{self, fill, Strings};
use crate::recorder::{self, Fed, Recorder, Stop};

fn ok<T: Serialize>(value: &T) -> String {
    serde_json::json!({ "ok": value }).to_string()
}

fn error(message: String) -> String {
    serde_json::json!({ "error": message }).to_string()
}

/// One button: the action name and value the page sends back through `act`.
/// `pressed` makes it a toggle the page marks with `aria-pressed`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Action {
    pub name: String,
    pub value: String,
    pub label: String,
    pub enabled: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pressed: Option<bool>,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub primary: bool,
}

/// A text box. The page sends `act(name, its text)` when the text changes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Field {
    pub name: String,
    pub label: String,
    pub value: String,
    pub kind: String,
}

/// A link the reader may follow, opened in a new tab.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Link {
    pub href: String,
    pub label: String,
}

/// A bar for a reading that changes while recording. `live` fills it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Meter {
    pub id: String,
    pub label: String,
    pub max: u32,
}

/// One line of text. A line with an id is updated by `live`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Line {
    #[serde(skip_serializing_if = "String::is_empty")]
    pub id: String,
    pub text: String,
}

/// One block of the screen. Empty fields are left out of the JSON.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct Panel {
    pub id: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub heading: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub lines: Vec<Line>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub steps: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub fields: Vec<Field>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub link: Option<Link>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meter: Option<Meter>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub actions: Vec<Action>,
}

/// A term and its definition, for the page's hover hints.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Hint {
    pub term: String,
    pub definition: String,
}

/// Everything the page draws, and what it should do after drawing.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Ui {
    pub screen: String,
    pub title: String,
    pub panels: Vec<Panel>,
    pub hints: Vec<Hint>,
    /// A sentence for the page's message line; empty for none.
    pub message: String,
    /// What the page carries out after drawing, in order: `capture.open`,
    /// `capture.close`, `file.save`.
    pub effects: Vec<String>,
    /// An action to send once this screen has been painted, or empty.
    pub follow: String,
    /// Whether the page should ask `live` for readings on each frame.
    pub live: bool,
    /// The name to save the file under.
    pub file_name: String,
}

/// One reading for `live`: text for a line, or a value for a meter.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Reading {
    pub id: String,
    pub text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<u32>,
}

/// What the page can do, reported once at start.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize)]
pub struct Abilities {
    /// The browser offers `getDisplayMedia`.
    pub display: bool,
    /// The browser offers `AudioWorkletNode`.
    pub worklet: bool,
    /// The browser is Chromium-based (it offers `navigator.userAgentData`).
    pub chromium: bool,
}

/// What the page reports when sharing starts.
#[derive(Debug, Clone, Copy, Deserialize)]
pub struct Shared {
    /// Sound tracks in the shared stream.
    pub tracks: u32,
    /// The sample rate of the page's audio context.
    pub rate: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Screen {
    Setup,
    Asking,
    Waiting,
    Recording,
    Encoding,
    Done,
}

impl Screen {
    fn name(self) -> &'static str {
        match self {
            Screen::Setup => "setup",
            Screen::Asking => "asking",
            Screen::Waiting => "waiting",
            Screen::Recording => "recording",
            Screen::Encoding => "encoding",
            Screen::Done => "done",
        }
    }
}

/// A finished recording.
struct Made {
    mp3: Vec<u8>,
    frames: u64,
    rate: u32,
    bitrate: u32,
    stopped: Option<String>,
}

/// The tool: the strings, the settings, and the recording in progress.
pub struct Tool {
    strings: Strings,
    hints: Vec<Hint>,
    abilities: Abilities,
    screen: Screen,
    link: String,
    bitrate: u32,
    stop_on_silence: bool,
    message: String,
    effects: Vec<String>,
    follow: String,
    recorder: Option<Recorder>,
    made: Option<Made>,
    title: String,
    artist: String,
}

fn hints_from(glossary_md: &str) -> Result<Vec<Hint>, String> {
    let entries = glossary::parse(glossary_md).map_err(|e| e.message())?;
    Ok(entries
        .into_iter()
        .map(|e| Hint {
            term: e.term,
            definition: e.definition,
        })
        .collect())
}

fn button(name: &str, value: &str, label: &str) -> Action {
    Action {
        name: name.into(),
        value: value.into(),
        label: label.into(),
        enabled: true,
        pressed: None,
        primary: false,
    }
}

fn line(text: String) -> Line {
    Line {
        id: String::new(),
        text,
    }
}

impl Tool {
    /// Reads the strings, the glossary and what the browser can do, and
    /// shows the first screen.
    pub fn start(strings_json: &str, glossary_md: &str, abilities_json: &str) -> Result<Tool, String> {
        let strings = page::parse_strings(strings_json).map_err(|e| e.message())?;
        let mut hints = hints_from(glossary_md)?;
        // A definition's placeholders are filled from the same rules the
        // screens use, so the glossary cannot disagree with them.
        let numbers = numbers();
        for h in &mut hints {
            h.definition = fill(&h.definition, &numbers);
        }
        let abilities: Abilities = serde_json::from_str(abilities_json)
            .map_err(|e| format!("The browser's abilities could not be read: {e}"))?;
        Ok(Tool {
            strings,
            hints,
            abilities,
            screen: Screen::Setup,
            link: String::new(),
            bitrate: recorder::DEFAULT_BITRATE,
            stop_on_silence: true,
            message: String::new(),
            effects: Vec::new(),
            follow: String::new(),
            recorder: None,
            made: None,
            title: String::new(),
            artist: String::new(),
        })
    }

    /// A string with the names of the buttons it mentions filled in.
    fn say(&self, template: &str) -> String {
        fill(
            template,
            &[
                ("choose", self.strings.choose.clone()),
                ("stop", self.strings.stop.clone()),
            ],
        )
    }

    fn can_share(&self) -> bool {
        self.abilities.display && self.abilities.worklet
    }

    fn setup_panels(&self) -> Vec<Panel> {
        let s = &self.strings;
        let mut panels = vec![
            Panel {
                id: "intro".into(),
                lines: vec![line(s.intro.clone())],
                ..Panel::default()
            },
            Panel {
                id: "steps".into(),
                heading: s.steps_heading.clone(),
                steps: s.steps.iter().map(|t| self.say(t)).collect(),
                ..Panel::default()
            },
        ];

        let mut video = Panel {
            id: "video".into(),
            fields: vec![Field {
                name: "link.set".into(),
                label: s.link_label.clone(),
                value: self.link.clone(),
                kind: "url".into(),
            }],
            ..Panel::default()
        };
        if !self.link.trim().is_empty() {
            match link::video_id(&self.link) {
                Ok(id) => {
                    video.link = Some(Link {
                        href: link::watch_address(&id),
                        label: s.link_open.clone(),
                    })
                }
                Err(LinkError::NotYouTube) => video.lines.push(line(s.link_not_youtube.clone())),
                Err(LinkError::NoVideo) => video.lines.push(line(s.link_no_video.clone())),
            }
        }
        panels.push(video);

        let kbps_default = [
            ("default", recorder::DEFAULT_BITRATE.to_string()),
            ("seconds", recorder::SILENCE_SECONDS.to_string()),
        ];
        let mut options: Vec<Action> = recorder::BITRATES
            .iter()
            .map(|k| Action {
                pressed: Some(*k == self.bitrate),
                ..button(
                    "bitrate.set",
                    &k.to_string(),
                    &fill(&s.bitrate_option, &[("kbps", k.to_string())]),
                )
            })
            .collect();
        options.push(Action {
            pressed: Some(self.stop_on_silence),
            ..button("silence.toggle", "", &fill(&s.silence_option, &kbps_default))
        });
        panels.push(Panel {
            id: "settings".into(),
            heading: s.bitrate_heading.clone(),
            lines: vec![line(fill(&s.bitrate_note, &kbps_default))],
            actions: options,
            ..Panel::default()
        });

        let mut start = Panel {
            id: "start".into(),
            actions: vec![Action {
                enabled: self.can_share(),
                primary: true,
                ..button("capture.choose", "", &s.choose)
            }],
            ..Panel::default()
        };
        if !self.can_share() {
            start.lines.push(line(s.browser_cannot.clone()));
        } else if !self.abilities.chromium {
            start.lines.push(line(s.browser_maybe.clone()));
        }
        start.lines.push(line(s.rights.clone()));
        panels.push(start);
        panels
    }

    fn heard(&self) -> (u64, u32) {
        self.recorder
            .as_ref()
            .map(|r| (r.heard_frames(), r.rate()))
            .unwrap_or((0, 0))
    }

    fn panels(&self) -> Vec<Panel> {
        let s = &self.strings;
        let seconds = recorder::SILENCE_SECONDS.to_string();
        match self.screen {
            Screen::Setup => self.setup_panels(),
            Screen::Asking => vec![Panel {
                id: "asking".into(),
                heading: s.asking_heading.clone(),
                lines: vec![line(s.asking_text.clone())],
                ..Panel::default()
            }],
            Screen::Waiting | Screen::Recording => {
                let (frames, rate) = self.heard();
                let waiting = self.screen == Screen::Waiting;
                let mut lines = vec![];
                if waiting {
                    lines.push(line(s.waiting_text.clone()));
                } else {
                    lines.push(Line {
                        id: "time".into(),
                        text: fill(&s.recording_time, &[("time", format::clock(frames, rate))]),
                    });
                    let note = if self.stop_on_silence {
                        fill(&s.recording_auto, &[("seconds", seconds)])
                    } else {
                        self.say(&s.recording_manual)
                    };
                    lines.push(line(note));
                }
                lines.push(line(s.adverts.clone()));
                let mut actions = Vec::new();
                if waiting {
                    actions.push(button("record.cancel", "", &s.cancel));
                } else {
                    actions.push(Action {
                        primary: true,
                        ..button("record.stop", "", &s.stop)
                    });
                }
                vec![Panel {
                    id: "recording".into(),
                    heading: if waiting {
                        s.waiting_heading.clone()
                    } else {
                        s.recording_heading.clone()
                    },
                    lines,
                    meter: Some(Meter {
                        id: "level".into(),
                        label: s.level_label.clone(),
                        max: 100,
                    }),
                    actions,
                    ..Panel::default()
                }]
            }
            Screen::Encoding => {
                let (frames, rate) = self.heard();
                let kept = self.recorder.as_ref().map(|r| r.kept_frames()).unwrap_or(frames);
                vec![Panel {
                    id: "encoding".into(),
                    heading: s.encoding_heading.clone(),
                    lines: vec![line(fill(
                        &s.encoding_text,
                        &[
                            ("time", format::clock(kept, rate)),
                            ("kbps", self.bitrate.to_string()),
                        ],
                    ))],
                    ..Panel::default()
                }]
            }
            Screen::Done => {
                let Some(made) = &self.made else {
                    return Vec::new();
                };
                let mut lines = vec![line(fill(
                    &s.done_text,
                    &[
                        ("time", format::clock(made.frames, made.rate)),
                        ("size", format::size(self.file_len())),
                        ("kbps", made.bitrate.to_string()),
                    ],
                ))];
                if let Some(stopped) = &made.stopped {
                    lines.push(line(stopped.clone()));
                }
                lines.push(line(s.tags_note.clone()));
                vec![Panel {
                    id: "done".into(),
                    heading: s.done_heading.clone(),
                    lines,
                    fields: vec![
                        Field {
                            name: "tag.title".into(),
                            label: s.title_label.clone(),
                            value: self.title.clone(),
                            kind: "text".into(),
                        },
                        Field {
                            name: "tag.artist".into(),
                            label: s.artist_label.clone(),
                            value: self.artist.clone(),
                            kind: "text".into(),
                        },
                    ],
                    actions: vec![
                        Action {
                            primary: true,
                            ..button("file.save", "", &s.save)
                        },
                        button("again", "", &s.again),
                    ],
                    ..Panel::default()
                }]
            }
        }
    }

    /// The screen as the page should draw it. Effects and the follow-up
    /// action are handed over once: the next `ui` has none until another
    /// action asks for them.
    pub fn ui(&mut self) -> Ui {
        Ui {
            screen: self.screen.name().into(),
            title: self.strings.title.clone(),
            panels: self.panels(),
            hints: self.hints.clone(),
            message: self.message.clone(),
            effects: std::mem::take(&mut self.effects),
            follow: std::mem::take(&mut self.follow),
            live: matches!(self.screen, Screen::Waiting | Screen::Recording),
            file_name: format::file_name(&self.title, &self.strings.default_name),
        }
    }

    fn back_to_setup(&mut self, message: String) {
        self.screen = Screen::Setup;
        self.recorder = None;
        self.message = message;
        self.effects.push("capture.close".into());
    }

    fn stopped_text(&self, reason: Option<Stop>, ended: bool) -> Option<String> {
        let s = &self.strings;
        match (reason, ended) {
            (Some(Stop::Silence), _) => Some(fill(
                &s.stopped_silence,
                &[("seconds", recorder::SILENCE_SECONDS.to_string())],
            )),
            (Some(Stop::Limit), _) => Some(fill(
                &s.stopped_limit,
                &[("hours", recorder::LIMIT_HOURS.to_string())],
            )),
            (None, true) => Some(s.stopped_sharing.clone()),
            (None, false) => None,
        }
    }

    /// Stops recording: with sound, on to encoding; without, back to setup.
    fn stop(&mut self, reason: Option<Stop>, ended: bool) {
        let started = self.recorder.as_ref().is_some_and(|r| r.started());
        if !started {
            let message = if ended {
                self.say(&self.strings.ended_early)
            } else {
                self.say(&self.strings.no_sound)
            };
            self.back_to_setup(message);
            return;
        }
        self.made = Some(Made {
            mp3: Vec::new(),
            frames: 0,
            rate: 0,
            bitrate: self.bitrate,
            stopped: self.stopped_text(reason, ended),
        });
        self.screen = Screen::Encoding;
        self.effects.push("capture.close".into());
        self.follow = "encode.finish".into();
    }

    fn finish(&mut self) -> Result<(), String> {
        if self.screen != Screen::Encoding {
            return Err("There is no recording to encode.".into());
        }
        let r = self.recorder.take().ok_or("There is no recording to encode.")?;
        let (frames, rate, bitrate) = (r.kept_frames(), r.rate(), r.bitrate());
        let mp3 = r.finish().ok_or("There is no recording to encode.")?;
        let stopped = self.made.take().and_then(|m| m.stopped);
        self.made = Some(Made {
            mp3,
            frames,
            rate,
            bitrate,
            stopped,
        });
        self.screen = Screen::Done;
        Ok(())
    }

    /// Applies one named action. An unknown action, or one the current screen
    /// does not offer, is an error and changes nothing.
    pub fn act(&mut self, name: &str, value: &str) -> Result<(), String> {
        // Effects belong to the action that asked for them.
        self.effects.clear();
        self.follow.clear();
        let screen = self.screen;
        let on = |allowed: &[Screen]| -> Result<(), String> {
            if allowed.contains(&screen) {
                Ok(())
            } else {
                Err(format!(
                    "\"{name}\" is not available on the {} screen.",
                    screen.name()
                ))
            }
        };
        match name {
            "link.set" => {
                on(&[Screen::Setup])?;
                self.link = value.to_string();
            }
            "bitrate.set" => {
                on(&[Screen::Setup])?;
                let kbps: u32 = value
                    .parse()
                    .map_err(|_| format!("There is no bitrate \"{value}\"."))?;
                if !recorder::BITRATES.contains(&kbps) {
                    return Err(format!("There is no bitrate \"{value}\"."));
                }
                self.bitrate = kbps;
            }
            "silence.toggle" => {
                on(&[Screen::Setup])?;
                self.stop_on_silence = !self.stop_on_silence;
            }
            "capture.choose" => {
                on(&[Screen::Setup])?;
                if !self.can_share() {
                    return Err(self.strings.browser_cannot.clone());
                }
                self.message.clear();
                self.screen = Screen::Asking;
                self.effects.push("capture.open".into());
            }
            "capture.started" => {
                on(&[Screen::Asking])?;
                let shared: Shared = serde_json::from_str(value)
                    .map_err(|e| format!("The shared tab could not be read: {e}"))?;
                if shared.tracks == 0 {
                    self.back_to_setup(self.say(&self.strings.no_audio));
                    return Ok(());
                }
                match Recorder::new(shared.rate, self.bitrate, self.stop_on_silence) {
                    Some(r) => {
                        self.recorder = Some(r);
                        self.screen = Screen::Waiting;
                    }
                    None => {
                        let message = fill(&self.strings.bad_rate, &[("rate", shared.rate.to_string())]);
                        self.back_to_setup(message);
                    }
                }
            }
            "capture.failed" => {
                on(&[Screen::Asking])?;
                let template = if value == "NotAllowedError" || value == "AbortError" {
                    &self.strings.picker_cancelled
                } else {
                    &self.strings.picker_failed
                };
                let message = fill(&self.say(template), &[("detail", value.to_string())]);
                self.back_to_setup(message);
            }
            "capture.ended" => {
                // The reader pressed the browser's own button to stop sharing.
                // Any other screen has already let the capture go.
                if matches!(self.screen, Screen::Waiting | Screen::Recording) {
                    self.stop(None, true);
                }
            }
            "record.cancel" => {
                on(&[Screen::Waiting])?;
                self.back_to_setup(String::new());
            }
            "record.stop" => {
                on(&[Screen::Waiting, Screen::Recording])?;
                let reason = match value {
                    "" => None,
                    "silence" => Some(Stop::Silence),
                    "limit" => Some(Stop::Limit),
                    other => return Err(format!("There is no reason to stop called \"{other}\".")),
                };
                self.stop(reason, false);
            }
            "encode.finish" => self.finish()?,
            // Changes nothing; the page sends it to draw the screen again.
            "screen.draw" => {}
            "tag.title" => {
                on(&[Screen::Done])?;
                self.title = value.to_string();
            }
            "tag.artist" => {
                on(&[Screen::Done])?;
                self.artist = value.to_string();
            }
            "file.save" => {
                on(&[Screen::Done])?;
                self.effects.push("file.save".into());
            }
            "again" => {
                on(&[Screen::Done])?;
                self.screen = Screen::Setup;
                self.made = None;
                self.title.clear();
                self.artist.clear();
                self.message.clear();
            }
            other => return Err(format!("Unknown action \"{other}\".")),
        }
        Ok(())
    }

    /// Takes one block of sound from the shared tab. Returns the action the
    /// page should send next as `{"name", "value"}`, or an empty string:
    /// `screen.draw` when the first sound changes the screen, `record.stop`
    /// when the recording stops by itself.
    pub fn feed(&mut self, left: &[u8], right: &[u8]) -> String {
        if !matches!(self.screen, Screen::Waiting | Screen::Recording) {
            return String::new();
        }
        let Some(r) = self.recorder.as_mut() else {
            return String::new();
        };
        let next = |name: &str, value: &str| serde_json::json!({ "name": name, "value": value }).to_string();
        match r.feed(left, right) {
            Fed::Waiting => String::new(),
            Fed::Recording if self.screen == Screen::Waiting => {
                // The first sound: the screen changes, so the page redraws.
                self.screen = Screen::Recording;
                next("screen.draw", "")
            }
            Fed::Recording => String::new(),
            Fed::Stop(stop) => {
                self.screen = Screen::Recording;
                let value = match stop {
                    Stop::Silence => "silence",
                    Stop::Limit => "limit",
                };
                next("record.stop", value)
            }
        }
    }

    /// The readings that change while recording: the time line and the level
    /// meter. Taking them starts the next level reading.
    pub fn live(&mut self) -> Vec<Reading> {
        let s = &self.strings;
        let Some(r) = self.recorder.as_mut() else {
            return Vec::new();
        };
        let level = r.take_level();
        vec![
            Reading {
                id: "time".into(),
                text: fill(
                    &s.recording_time,
                    &[("time", format::clock(r.heard_frames(), r.rate()))],
                ),
                value: None,
            },
            Reading {
                id: "level".into(),
                text: fill(&s.level_text, &[("percent", level.to_string())]),
                value: Some(level),
            },
        ]
    }

    /// The length of `file()`, without building it.
    fn file_len(&self) -> usize {
        let frames = self.made.as_ref().map(|m| m.mp3.len()).unwrap_or(0);
        id3::tag(&self.title, &self.artist).len() + frames
    }

    /// The MP3 file: the tag for the title and artist, then the frames.
    pub fn file(&self) -> Vec<u8> {
        let Some(made) = &self.made else {
            return Vec::new();
        };
        let mut out = id3::tag(&self.title, &self.artist);
        out.extend_from_slice(&made.mp3);
        out
    }
}

/// The numbers the glossary's placeholders name, from the rules.
fn numbers() -> Vec<(&'static str, String)> {
    let list = |v: &[u32]| v.iter().map(u32::to_string).collect::<Vec<_>>().join(", ");
    vec![
        ("bitrates", list(recorder::BITRATES)),
        ("default", recorder::DEFAULT_BITRATE.to_string()),
        ("seconds", recorder::SILENCE_SECONDS.to_string()),
        ("hours", recorder::LIMIT_HOURS.to_string()),
    ]
}

/// The handle the shim holds. Each method returns JSON: the screen as
/// `{"ok": ...}`, or `{"error": "..."}` with a sentence the page shows as it
/// is. A tool whose files could not be read answers every call with the
/// reason.
pub struct Handle(Result<Tool, String>);

impl Handle {
    pub fn new(strings_json: &str, glossary_md: &str, abilities_json: &str) -> Handle {
        Handle(Tool::start(strings_json, glossary_md, abilities_json))
    }

    /// The current screen.
    pub fn ui(&mut self) -> String {
        match &mut self.0 {
            Ok(t) => ok(&t.ui()),
            Err(e) => error(e.clone()),
        }
    }

    /// Applies an action and returns the screen after it. A refused action
    /// changes nothing and carries the reason in `error`.
    pub fn act(&mut self, name: &str, value: &str) -> String {
        match &mut self.0 {
            Ok(t) => match t.act(name, value) {
                Ok(()) => ok(&t.ui()),
                Err(e) => error(e),
            },
            Err(e) => error(e.clone()),
        }
    }

    pub fn feed(&mut self, left: &[u8], right: &[u8]) -> String {
        match &mut self.0 {
            Ok(t) => t.feed(left, right),
            Err(_) => String::new(),
        }
    }

    pub fn live(&mut self) -> String {
        match &mut self.0 {
            Ok(t) => serde_json::to_string(&t.live()).unwrap_or_default(),
            Err(_) => "[]".into(),
        }
    }

    pub fn file(&self) -> Vec<u8> {
        match &self.0 {
            Ok(t) => t.file(),
            Err(_) => Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const STRINGS: &str = include_str!("../../../data/strings.json");
    const GLOSSARY: &str = include_str!("../../../GLOSSARY.md");
    const ALL: &str = r#"{"display": true, "worklet": true, "chromium": true}"#;
    const LOUD: u32 = 0x3f00_0000; // 0.5
    const QUIET: u32 = 0;
    // 128 frames is the block an AudioWorklet delivers.
    const BLOCK: usize = 128;

    fn tool() -> Tool {
        Tool::start(STRINGS, GLOSSARY, ALL).unwrap()
    }

    fn strings() -> Strings {
        page::parse_strings(STRINGS).unwrap()
    }

    fn block(sample: u32) -> Vec<u8> {
        sample.to_le_bytes().repeat(BLOCK)
    }

    fn feed(t: &mut Tool, sample: u32, blocks: usize) -> String {
        let b = block(sample);
        let mut last = String::new();
        for _ in 0..blocks {
            let out = t.feed(&b, &b);
            if !out.is_empty() {
                last = out;
            }
        }
        last
    }

    fn texts(ui: &Ui) -> Vec<String> {
        let mut out = vec![ui.title.clone(), ui.message.clone()];
        for p in &ui.panels {
            out.push(p.heading.clone());
            out.extend(p.lines.iter().map(|l| l.text.clone()));
            out.extend(p.steps.iter().cloned());
            out.extend(p.fields.iter().map(|f| f.label.clone()));
            out.extend(p.actions.iter().map(|a| a.label.clone()));
            out.extend(p.link.iter().map(|l| l.label.clone()));
            out.extend(p.meter.iter().map(|m| m.label.clone()));
        }
        out.extend(ui.hints.iter().map(|h| h.definition.clone()));
        out
    }

    fn panel<'a>(ui: &'a Ui, id: &str) -> &'a Panel {
        ui.panels
            .iter()
            .find(|p| p.id == id)
            .unwrap_or_else(|| panic!("no panel {id}"))
    }

    fn action<'a>(ui: &'a Ui, name: &str, value: &str) -> &'a Action {
        ui.panels
            .iter()
            .flat_map(|p| &p.actions)
            .find(|a| a.name == name && a.value == value)
            .unwrap_or_else(|| panic!("no action {name} {value}"))
    }

    /// A tool that has heard one second of sound at 48 kHz.
    fn recording() -> Tool {
        let mut t = tool();
        t.act("capture.choose", "").unwrap();
        t.act("capture.started", r#"{"tracks": 1, "rate": 48000}"#)
            .unwrap();
        feed(&mut t, LOUD, 375);
        t
    }

    fn done() -> Tool {
        let mut t = recording();
        t.act("record.stop", "").unwrap();
        t.act("encode.finish", "").unwrap();
        t
    }

    #[test]
    fn the_setup_screen_offers_the_tab_picker_and_the_settings() {
        let ui = tool().ui();
        assert_eq!(ui.screen, "setup");
        assert_eq!(ui.title, "YouTube to MP3");
        let choose = action(&ui, "capture.choose", "");
        assert!(choose.enabled && choose.primary);
        assert_eq!(action(&ui, "bitrate.set", "192").pressed, Some(true));
        assert_eq!(action(&ui, "bitrate.set", "128").pressed, Some(false));
        assert_eq!(action(&ui, "silence.toggle", "").pressed, Some(true));
        assert!(ui.effects.is_empty() && ui.follow.is_empty() && !ui.live);
        assert_eq!(ui.file_name, "recording.mp3");
    }

    #[test]
    fn the_steps_name_the_buttons_they_mean() {
        let ui = tool().ui();
        let steps = &panel(&ui, "steps").steps;
        assert!(steps[1].starts_with("Press Choose the tab,"), "{}", steps[1]);
        assert!(
            steps[3].starts_with("Press Stop and make the MP3 "),
            "{}",
            steps[3]
        );
    }

    #[test]
    fn every_placeholder_on_every_screen_is_filled() {
        let mut seen = Vec::new();
        let mut t = tool();
        t.act("link.set", "https://youtu.be/aqz-KE-bpKQ").unwrap();
        seen.push(t.ui());
        t.act("capture.choose", "").unwrap();
        seen.push(t.ui());
        t.act("capture.started", r#"{"tracks": 1, "rate": 48000}"#)
            .unwrap();
        seen.push(t.ui());
        feed(&mut t, LOUD, 10);
        seen.push(t.ui());
        t.act("silence.toggle", "").unwrap_err();
        t.act("record.stop", "").unwrap();
        seen.push(t.ui());
        t.act("encode.finish", "").unwrap();
        seen.push(t.ui());
        let mut manual = tool();
        manual.act("silence.toggle", "").unwrap();
        manual.act("capture.choose", "").unwrap();
        manual
            .act("capture.started", r#"{"tracks": 1, "rate": 48000}"#)
            .unwrap();
        feed(&mut manual, LOUD, 10);
        seen.push(manual.ui());
        let mut failed = tool();
        failed.act("capture.choose", "").unwrap();
        failed.act("capture.failed", "NotReadableError").unwrap();
        seen.push(failed.ui());
        for ui in &seen {
            for text in texts(ui) {
                assert!(
                    !text.contains('{') && !text.contains('}'),
                    "{}: {text}",
                    ui.screen
                );
            }
        }
        let mut r = recording();
        for reading in r.live() {
            assert!(!reading.text.contains('{'), "{}", reading.text);
        }
    }

    #[test]
    fn a_browser_that_cannot_share_a_tab_is_told_so_and_cannot_start() {
        let mut t = Tool::start(
            STRINGS,
            GLOSSARY,
            r#"{"display": false, "worklet": true, "chromium": false}"#,
        )
        .unwrap();
        let ui = t.ui();
        assert!(!action(&ui, "capture.choose", "").enabled);
        assert!(panel(&ui, "start")
            .lines
            .iter()
            .any(|l| l.text == strings().browser_cannot));
        assert_eq!(t.act("capture.choose", ""), Err(strings().browser_cannot));
        assert_eq!(t.ui().screen, "setup");
    }

    #[test]
    fn a_browser_other_than_chrome_or_edge_is_warned_and_may_still_try() {
        let mut t = Tool::start(
            STRINGS,
            GLOSSARY,
            r#"{"display": true, "worklet": true, "chromium": false}"#,
        )
        .unwrap();
        let ui = t.ui();
        assert!(action(&ui, "capture.choose", "").enabled);
        assert!(panel(&ui, "start")
            .lines
            .iter()
            .any(|l| l.text == strings().browser_maybe));
        assert!(!texts(&tool().ui()).contains(&strings().browser_maybe));
    }

    #[test]
    fn a_youtube_link_is_offered_as_the_watch_page() {
        let mut t = tool();
        t.act("link.set", "https://youtu.be/aqz-KE-bpKQ?si=x").unwrap();
        let ui = t.ui();
        let link = panel(&ui, "video").link.clone().unwrap();
        assert_eq!(link.href, "https://www.youtube.com/watch?v=aqz-KE-bpKQ");
        assert_eq!(link.label, strings().link_open);
        assert_eq!(
            panel(&ui, "video").fields[0].value,
            "https://youtu.be/aqz-KE-bpKQ?si=x"
        );
    }

    #[test]
    fn a_link_that_is_not_a_video_says_why() {
        let mut t = tool();
        t.act("link.set", "https://vimeo.com/1").unwrap();
        let ui = t.ui();
        assert!(panel(&ui, "video").link.is_none());
        assert_eq!(panel(&ui, "video").lines[0].text, strings().link_not_youtube);
        t.act("link.set", "https://www.youtube.com/@channel").unwrap();
        assert_eq!(panel(&t.ui(), "video").lines[0].text, strings().link_no_video);
        t.act("link.set", "  ").unwrap();
        assert!(panel(&t.ui(), "video").lines.is_empty());
    }

    #[test]
    fn the_bitrate_can_be_one_of_those_offered_and_no_other() {
        let mut t = tool();
        t.act("bitrate.set", "256").unwrap();
        let ui = t.ui();
        assert_eq!(action(&ui, "bitrate.set", "256").pressed, Some(true));
        assert_eq!(action(&ui, "bitrate.set", "192").pressed, Some(false));
        assert_eq!(
            t.act("bitrate.set", "320"),
            Err("There is no bitrate \"320\".".into())
        );
        assert_eq!(
            t.act("bitrate.set", "lots"),
            Err("There is no bitrate \"lots\".".into())
        );
        assert_eq!(action(&t.ui(), "bitrate.set", "256").pressed, Some(true));
    }

    #[test]
    fn stopping_on_silence_can_be_turned_off_and_on() {
        let mut t = tool();
        t.act("silence.toggle", "").unwrap();
        assert_eq!(action(&t.ui(), "silence.toggle", "").pressed, Some(false));
        t.act("silence.toggle", "").unwrap();
        assert_eq!(action(&t.ui(), "silence.toggle", "").pressed, Some(true));
    }

    #[test]
    fn choosing_the_tab_asks_the_page_to_open_the_picker_once() {
        let mut t = tool();
        t.act("capture.choose", "").unwrap();
        let ui = t.ui();
        assert_eq!(ui.screen, "asking");
        assert_eq!(ui.effects, vec!["capture.open"]);
        assert!(t.ui().effects.is_empty(), "an effect is handed over once");
    }

    #[test]
    fn a_tab_shared_without_sound_goes_back_with_the_reason() {
        let mut t = tool();
        t.act("capture.choose", "").unwrap();
        t.act("capture.started", r#"{"tracks": 0, "rate": 48000}"#)
            .unwrap();
        let ui = t.ui();
        assert_eq!(ui.screen, "setup");
        assert_eq!(ui.effects, vec!["capture.close"]);
        assert_eq!(
            ui.message,
            "The tab was shared without its sound. Press Choose the tab again and turn on the switch to share its audio."
        );
    }

    #[test]
    fn a_rate_the_format_cannot_hold_goes_back_with_the_rate() {
        let mut t = tool();
        t.act("capture.choose", "").unwrap();
        t.act("capture.started", r#"{"tracks": 1, "rate": 96000}"#)
            .unwrap();
        let ui = t.ui();
        assert_eq!(ui.screen, "setup");
        assert!(
            ui.message
                .starts_with("The browser records at 96000 samples a second"),
            "{}",
            ui.message
        );
    }

    #[test]
    fn a_cancelled_picker_and_a_failed_one_say_different_things() {
        let mut t = tool();
        t.act("capture.choose", "").unwrap();
        t.act("capture.failed", "NotAllowedError").unwrap();
        assert_eq!(
            t.ui().message,
            "No tab was shared. Press Choose the tab to try again."
        );
        t.act("capture.choose", "").unwrap();
        t.act("capture.failed", "NotReadableError").unwrap();
        assert_eq!(
            t.ui().message,
            "The browser did not share the tab (NotReadableError). Press Choose the tab to try again."
        );
    }

    #[test]
    fn sharing_waits_for_sound_then_records() {
        let mut t = tool();
        t.act("capture.choose", "").unwrap();
        t.act("capture.started", r#"{"tracks": 1, "rate": 48000}"#)
            .unwrap();
        let ui = t.ui();
        assert_eq!(ui.screen, "waiting");
        assert!(ui.live);
        assert_eq!(feed(&mut t, QUIET, 100), "");
        assert_eq!(t.ui().screen, "waiting");
        assert_eq!(feed(&mut t, LOUD, 1), r#"{"name":"screen.draw","value":""}"#);
        assert_eq!(
            feed(&mut t, LOUD, 374),
            "",
            "only the first sound asks for a redraw"
        );
        t.act("screen.draw", "").unwrap();
        let ui = t.ui();
        assert_eq!(ui.screen, "recording");
        assert_eq!(panel(&ui, "recording").lines[0].text, "Recorded 0:01");
        assert_eq!(action(&ui, "record.stop", "").label, strings().stop);
    }

    #[test]
    fn live_readings_give_the_time_and_the_level() {
        let mut t = recording();
        let live = t.live();
        assert_eq!(live[0].id, "time");
        assert_eq!(live[0].text, "Recorded 0:01");
        assert_eq!(live[1].id, "level");
        assert_eq!(live[1].value, Some(50));
        assert_eq!(live[1].text, "50 percent of full volume");
        assert_eq!(
            t.live()[1].value,
            Some(0),
            "the level starts again after each reading"
        );
    }

    #[test]
    fn stopping_encodes_after_the_screen_is_drawn_then_offers_the_file() {
        let mut t = recording();
        t.act("record.stop", "").unwrap();
        let ui = t.ui();
        assert_eq!(ui.screen, "encoding");
        assert_eq!(ui.effects, vec!["capture.close"]);
        assert_eq!(ui.follow, "encode.finish");
        assert_eq!(
            panel(&ui, "encoding").lines[0].text,
            "Encoding 0:01 of sound at 192 kbps."
        );
        t.act("encode.finish", "").unwrap();
        let ui = t.ui();
        assert_eq!(ui.screen, "done");
        // One second at 192 kbps and 48 kHz: 43 frames of 576 bytes (see
        // recorder.rs), no tag yet.
        assert_eq!(
            panel(&ui, "done").lines[0].text,
            "0:01 of sound at 192 kbps, 25 KB."
        );
        let file = t.file();
        assert_eq!(file.len(), 24_768);
        assert_eq!(&file[..2], &[0xff, 0xfb]);
    }

    #[test]
    fn the_title_and_artist_tag_the_file_and_name_it() {
        let mut t = done();
        let bare = t.file();
        t.act("tag.title", "Song: Live?").unwrap();
        t.act("tag.artist", "B").unwrap();
        let ui = t.ui();
        assert_eq!(ui.file_name, "Song Live.mp3");
        let file = t.file();
        let tag = id3::tag("Song: Live?", "B");
        assert_eq!(&file[..tag.len()], &tag[..]);
        assert_eq!(&file[tag.len()..], &bare[..]);
        assert_eq!(panel(&ui, "done").fields[0].value, "Song: Live?");
    }

    #[test]
    fn saving_asks_the_page_to_save_once() {
        let mut t = done();
        t.act("file.save", "").unwrap();
        assert_eq!(t.ui().effects, vec!["file.save"]);
        assert!(t.ui().effects.is_empty());
    }

    #[test]
    fn record_another_starts_over_and_keeps_the_settings() {
        let mut t = tool();
        t.act("bitrate.set", "128").unwrap();
        t.act("link.set", "https://youtu.be/aqz-KE-bpKQ").unwrap();
        t.act("capture.choose", "").unwrap();
        t.act("capture.started", r#"{"tracks": 1, "rate": 48000}"#)
            .unwrap();
        feed(&mut t, LOUD, 10);
        t.act("record.stop", "").unwrap();
        t.act("encode.finish", "").unwrap();
        t.act("tag.title", "Song").unwrap();
        t.act("again", "").unwrap();
        let ui = t.ui();
        assert_eq!(ui.screen, "setup");
        assert_eq!(ui.file_name, "recording.mp3");
        assert_eq!(action(&ui, "bitrate.set", "128").pressed, Some(true));
        assert!(panel(&ui, "video").link.is_some());
        assert!(t.file().is_empty());
    }

    #[test]
    fn silence_after_the_sound_stops_the_recording_by_itself() {
        let mut t = recording();
        // Three seconds at 48 kHz is 144000 frames: 1125 blocks of 128.
        assert_eq!(feed(&mut t, QUIET, 1124), "");
        assert_eq!(
            feed(&mut t, QUIET, 1),
            r#"{"name":"record.stop","value":"silence"}"#
        );
        t.act("record.stop", "silence").unwrap();
        t.act("encode.finish", "").unwrap();
        let ui = t.ui();
        assert_eq!(
            panel(&ui, "done").lines[1].text,
            "Recording stopped after 3 seconds of silence, which is left out of the file."
        );
        assert_eq!(
            panel(&ui, "done").lines[0].text,
            "0:01 of sound at 192 kbps, 25 KB."
        );
    }

    #[test]
    fn stopping_before_any_sound_goes_back_with_the_reason() {
        let mut t = tool();
        t.act("capture.choose", "").unwrap();
        t.act("capture.started", r#"{"tracks": 1, "rate": 48000}"#)
            .unwrap();
        t.act("record.stop", "").unwrap();
        let ui = t.ui();
        assert_eq!(ui.screen, "setup");
        assert_eq!(ui.message, strings().no_sound);
        assert_eq!(ui.effects, vec!["capture.close"]);
    }

    #[test]
    fn ending_the_share_from_the_browser_stops_or_goes_back() {
        let mut t = recording();
        t.act("capture.ended", "").unwrap();
        assert_eq!(t.ui().screen, "encoding");
        t.act("encode.finish", "").unwrap();
        assert_eq!(panel(&t.ui(), "done").lines[1].text, strings().stopped_sharing);

        let mut early = tool();
        early.act("capture.choose", "").unwrap();
        early
            .act("capture.started", r#"{"tracks": 1, "rate": 48000}"#)
            .unwrap();
        early.act("capture.ended", "").unwrap();
        let ui = early.ui();
        assert_eq!(ui.screen, "setup");
        assert_eq!(
            ui.message,
            "Sharing ended before any sound was heard. Press Choose the tab to start again."
        );

        let mut after = done();
        after.act("capture.ended", "").unwrap();
        assert_eq!(
            after.ui().screen,
            "done",
            "a share that already ended changes nothing"
        );
    }

    #[test]
    fn sound_after_the_recording_stopped_is_ignored() {
        let mut t = recording();
        t.act("record.stop", "").unwrap();
        assert_eq!(feed(&mut t, LOUD, 10), "");
        t.act("encode.finish", "").unwrap();
        assert_eq!(t.file().len(), 24_768);
    }

    #[test]
    fn an_unknown_action_and_one_the_screen_does_not_offer_are_refused() {
        let mut t = tool();
        assert_eq!(
            t.act("screen.jump", ""),
            Err("Unknown action \"screen.jump\".".into())
        );
        assert_eq!(
            t.act("record.stop", ""),
            Err("\"record.stop\" is not available on the setup screen.".into())
        );
        assert_eq!(
            t.act("encode.finish", ""),
            Err("There is no recording to encode.".into())
        );
        assert_eq!(t.ui().screen, "setup");
        let mut r = recording();
        assert!(r.act("bitrate.set", "128").is_err());
        assert!(r.act("record.stop", "bored").is_err());
        assert_eq!(r.ui().screen, "recording");
    }

    #[test]
    fn the_glossary_definitions_carry_the_rules_numbers() {
        let ui = tool().ui();
        let bitrate = ui.hints.iter().find(|h| h.term == "Bitrate").unwrap();
        assert!(
            bitrate
                .definition
                .contains("The page offers 128, 192, 256 kbps and starts at 192."),
            "{}",
            bitrate.definition
        );
        let silence = ui.hints.iter().find(|h| h.term == "Silence").unwrap();
        assert!(
            silence.definition.contains(", 3 seconds of it ends"),
            "{}",
            silence.definition
        );
    }

    #[test]
    fn the_handle_answers_in_json_and_reports_broken_files() {
        let mut h = Handle::new(STRINGS, GLOSSARY, ALL);
        let v: serde_json::Value = serde_json::from_str(&h.ui()).unwrap();
        assert_eq!(v["ok"]["screen"], "setup");
        let v: serde_json::Value = serde_json::from_str(&h.act("nope", "")).unwrap();
        assert_eq!(v["error"], "Unknown action \"nope\".");
        let mut broken = Handle::new("{}", GLOSSARY, ALL);
        let v: serde_json::Value = serde_json::from_str(&broken.ui()).unwrap();
        assert!(v["error"]
            .as_str()
            .unwrap()
            .starts_with("The strings file could not be read"));
        assert_eq!(broken.live(), "[]");
        assert!(broken.file().is_empty());
        let mut bad = Handle::new(STRINGS, GLOSSARY, "not json");
        let v: serde_json::Value = serde_json::from_str(&bad.act("again", "")).unwrap();
        assert!(v["error"]
            .as_str()
            .unwrap()
            .starts_with("The browser's abilities could not be read"));
    }
}
