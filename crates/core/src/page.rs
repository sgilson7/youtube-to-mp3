//! The reader-visible strings, read from `data/strings.json`. Every key the
//! screens use is a field here, so a missing or misspelt key fails when the
//! file is read, not when a screen happens to need it.

use serde::Deserialize;

/// Every string the page shows. Numbers are `{placeholders}` the core fills.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Strings {
    pub title: String,
    pub intro: String,
    pub steps_heading: String,
    pub steps: Vec<String>,
    pub link_label: String,
    pub link_open: String,
    pub here: String,
    pub link_not_youtube: String,
    pub link_no_video: String,
    pub bitrate_heading: String,
    pub bitrate_option: String,
    pub bitrate_note: String,
    pub silence_option: String,
    pub silent_option: String,
    pub choose: String,
    pub rights: String,
    pub browser_heading: String,
    pub browser_cannot: String,
    pub asking_heading: String,
    pub asking_text: String,
    pub asking_here_heading: String,
    pub asking_here_text: String,
    pub waiting_heading: String,
    pub waiting_text: String,
    pub waiting_here_text: String,
    pub cancel: String,
    pub recording_heading: String,
    pub recording_time: String,
    pub level_label: String,
    pub level_text: String,
    pub progress_label: String,
    pub progress_text: String,
    pub player_title: String,
    pub recording_auto: String,
    pub recording_manual: String,
    pub recording_here: String,
    pub adverts: String,
    pub stop: String,
    pub encoding_heading: String,
    pub encoding_text: String,
    pub done_heading: String,
    pub done_text: String,
    pub title_label: String,
    pub artist_label: String,
    pub tags_note: String,
    pub save: String,
    pub again: String,
    pub default_name: String,
    pub stopped_silence: String,
    pub stopped_limit: String,
    pub stopped_sharing: String,
    pub stopped_ended: String,
    pub no_audio: String,
    pub picker_cancelled: String,
    pub picker_failed: String,
    pub no_sound: String,
    pub ended_early: String,
    pub player_blocked: String,
    pub player_failed: String,
    pub bad_rate: String,
}

/// Why a strings file cannot be used.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StringsError {
    NotJson(String),
    Empty(String),
}

impl StringsError {
    /// The message the page shows. These two are about the tool's own files,
    /// for whoever maintains it, so they stay beside the rule in English.
    pub fn message(&self) -> String {
        match self {
            StringsError::NotJson(detail) => {
                format!("The strings file could not be read: {detail}")
            }
            StringsError::Empty(field) => format!("The strings file has an empty \"{field}\"."),
        }
    }
}

/// Parses and checks the strings file. A string that is only whitespace counts
/// as empty, because the page would draw nothing.
pub fn parse_strings(json: &str) -> Result<Strings, StringsError> {
    let strings: Strings = serde_json::from_str(json).map_err(|e| StringsError::NotJson(e.to_string()))?;
    let value: serde_json::Value =
        serde_json::from_str(json).map_err(|e| StringsError::NotJson(e.to_string()))?;
    if let Some(map) = value.as_object() {
        for (key, v) in map {
            let blank = match v {
                serde_json::Value::String(s) => s.trim().is_empty(),
                serde_json::Value::Array(items) => {
                    items.is_empty()
                        || items
                            .iter()
                            .any(|i| i.as_str().is_none_or(|s| s.trim().is_empty()))
                }
                _ => false,
            };
            if blank {
                return Err(StringsError::Empty(key.clone()));
            }
        }
    }
    Ok(strings)
}

/// Replaces each `{name}` in `template` with its value.
pub fn fill(template: &str, values: &[(&str, String)]) -> String {
    values.iter().fold(template.to_string(), |text, (name, value)| {
        text.replace(&format!("{{{name}}}"), value)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn real() -> String {
        std::str::from_utf8(include_bytes!("../../../data/strings.json"))
            .unwrap()
            .to_string()
    }

    #[test]
    fn the_strings_file_is_complete() {
        let s = parse_strings(&real()).unwrap();
        assert!(!s.steps.is_empty());
    }

    #[test]
    fn an_empty_string_is_rejected_by_name() {
        let blanked = {
            let mut v: serde_json::Value = serde_json::from_str(&real()).unwrap();
            v["title"] = serde_json::Value::String("  ".into());
            v.to_string()
        };
        assert_eq!(parse_strings(&blanked), Err(StringsError::Empty("title".into())));
        assert_eq!(
            StringsError::Empty("title".into()).message(),
            "The strings file has an empty \"title\"."
        );
    }

    #[test]
    fn an_empty_step_is_rejected_by_name() {
        let mut v: serde_json::Value = serde_json::from_str(&real()).unwrap();
        v["steps"][0] = serde_json::Value::String(String::new());
        assert_eq!(
            parse_strings(&v.to_string()),
            Err(StringsError::Empty("steps".into()))
        );
    }

    #[test]
    fn a_missing_or_unknown_key_is_rejected() {
        let mut v: serde_json::Value = serde_json::from_str(&real()).unwrap();
        v.as_object_mut().unwrap().remove("save");
        assert!(matches!(
            parse_strings(&v.to_string()),
            Err(StringsError::NotJson(_))
        ));
        let mut v: serde_json::Value = serde_json::from_str(&real()).unwrap();
        v["extra"] = serde_json::Value::String("x".into());
        assert!(matches!(
            parse_strings(&v.to_string()),
            Err(StringsError::NotJson(_))
        ));
    }

    #[test]
    fn text_that_is_not_json_is_rejected() {
        assert!(matches!(parse_strings("title: T"), Err(StringsError::NotJson(_))));
    }

    #[test]
    fn fill_replaces_each_named_placeholder() {
        assert_eq!(
            fill("{a} and {b}, {a}", &[("a", "1".into()), ("b", "two".into())]),
            "1 and two, 1"
        );
        assert_eq!(fill("{c}", &[("a", "1".into())]), "{c}");
    }
}
