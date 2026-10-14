//! Numbers and names as the reader sees them. Time comes from counting
//! samples, not from a clock, so it is the length of the sound kept.

/// Characters no common file system accepts in a name.
const NOT_IN_NAMES: &[char] = &['/', '\\', ':', '*', '?', '"', '<', '>', '|'];

/// The longest file name stem, in characters, before `.mp3`.
pub const NAME_LIMIT: usize = 120;

/// `frames` sample frames at `rate` per second as `m:ss`, or `h:mm:ss` from
/// an hour. Rounds down to the whole second.
pub fn clock(frames: u64, rate: u32) -> String {
    let seconds = frames.checked_div(u64::from(rate)).unwrap_or(0);
    let (h, m, s) = (seconds / 3600, seconds / 60 % 60, seconds % 60);
    if h > 0 {
        format!("{h}:{m:02}:{s:02}")
    } else {
        format!("{m}:{s:02}")
    }
}

/// A file size in kilobytes under a megabyte, else in megabytes to one
/// decimal place, rounded half up. A kilobyte is 1000 bytes.
pub fn size(bytes: usize) -> String {
    if bytes < 1_000_000 {
        let kb = (bytes + 500) / 1000;
        return format!("{} KB", kb.max(1));
    }
    let tenths = (bytes + 50_000) / 100_000;
    format!("{}.{} MB", tenths / 10, tenths % 10)
}

/// A file name for the MP3 from the title the reader typed: characters file
/// systems refuse are dropped, runs of space become one space, leading and
/// trailing dots and spaces go, and the stem is cut to `NAME_LIMIT`
/// characters. A title with nothing left uses `fallback`.
pub fn file_name(title: &str, fallback: &str) -> String {
    let kept: String = title
        .chars()
        .map(|c| if c.is_whitespace() { ' ' } else { c })
        .filter(|c| !c.is_control() && !NOT_IN_NAMES.contains(c))
        .collect();
    let spaced = kept.split_whitespace().collect::<Vec<_>>().join(" ");
    let cut: String = spaced.chars().take(NAME_LIMIT).collect();
    let stem = cut.trim_matches(|c: char| c == '.' || c == ' ');
    let stem = if stem.is_empty() { fallback } else { stem };
    format!("{stem}.mp3")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clock_counts_whole_seconds_of_samples() {
        assert_eq!(clock(0, 48_000), "0:00");
        assert_eq!(clock(47_999, 48_000), "0:00");
        assert_eq!(clock(48_000, 48_000), "0:01");
        assert_eq!(clock(48_000 * 83, 48_000), "1:23");
        assert_eq!(clock(44_100 * 600, 44_100), "10:00");
        assert_eq!(clock(48_000 * 3_725, 48_000), "1:02:05");
        assert_eq!(clock(5, 0), "0:00", "no rate yet");
    }

    #[test]
    fn size_uses_kilobytes_then_megabytes_to_a_tenth() {
        assert_eq!(size(0), "1 KB");
        assert_eq!(size(1_499), "1 KB");
        assert_eq!(size(1_500), "2 KB");
        assert_eq!(size(999_499), "999 KB");
        assert_eq!(size(1_000_000), "1.0 MB");
        assert_eq!(size(3_449_999), "3.4 MB");
        assert_eq!(size(3_450_000), "3.5 MB");
        assert_eq!(size(12_960_000), "13.0 MB");
    }

    #[test]
    fn file_name_drops_what_file_systems_refuse() {
        assert_eq!(file_name("AC/DC: Back in Black?", "x"), "ACDC Back in Black.mp3");
        assert_eq!(file_name("  a\tb\n  c  ", "x"), "a b c.mp3");
        assert_eq!(file_name("...hidden.", "x"), "hidden.mp3");
        assert_eq!(file_name("Café 東京", "x"), "Café 東京.mp3");
    }

    #[test]
    fn file_name_falls_back_when_nothing_is_left() {
        assert_eq!(file_name("", "recording"), "recording.mp3");
        assert_eq!(file_name(" ?*: ", "recording"), "recording.mp3");
    }

    #[test]
    fn file_name_cuts_a_long_title_by_characters() {
        let long = "é".repeat(NAME_LIMIT + 30);
        assert_eq!(file_name(&long, "x"), format!("{}.mp3", "é".repeat(NAME_LIMIT)));
    }
}
