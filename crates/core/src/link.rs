//! Reading a YouTube link. The page cannot fetch anything from YouTube, so a
//! link is used for one thing: to offer the video's own address in a new tab,
//! where the reader plays it while the page records.

/// The length of every YouTube video id.
const ID_LENGTH: usize = 11;

/// Hosts that serve YouTube watch pages.
const HOSTS: &[&str] = &[
    "youtube.com",
    "m.youtube.com",
    "music.youtube.com",
    "youtube-nocookie.com",
];

/// Path prefixes that are followed directly by a video id.
const ID_PATHS: &[&str] = &["/shorts/", "/embed/", "/live/", "/v/"];

/// Why a link was not read as a video.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LinkError {
    /// Not a YouTube address at all.
    NotYouTube,
    /// A YouTube address that names no single video (a channel, a playlist).
    NoVideo,
}

fn is_id_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '-' || c == '_'
}

/// The id if `text` is exactly a well-formed video id.
fn as_id(text: &str) -> Option<&str> {
    (text.len() == ID_LENGTH && text.chars().all(is_id_char)).then_some(text)
}

/// The value of `v` in a query string.
fn query_v(query: &str) -> Option<&str> {
    query.split('&').find_map(|pair| pair.strip_prefix("v="))
}

/// Reads a pasted link and returns the video id it names. Accepts the forms
/// people copy: watch pages, `youtu.be` short links, Shorts, embeds and live
/// pages, with or without the scheme and `www.`.
pub fn video_id(link: &str) -> Result<String, LinkError> {
    let text = link.trim();
    // The scheme and host are case-insensitive; the path and query are not,
    // because video ids mix cases.
    let rest = match text.find("://") {
        Some(at) if ["http", "https"].contains(&text[..at].to_ascii_lowercase().as_str()) => &text[at + 3..],
        _ => text,
    };
    let split = rest.find(['/', '?', '#']).unwrap_or(rest.len());
    let host = rest[..split].to_ascii_lowercase();
    let host = host.strip_prefix("www.").unwrap_or(&host).to_string();
    let after = &rest[split..];
    let after = after.split('#').next().unwrap_or("");
    let (path, query) = match after.split_once('?') {
        Some((p, q)) => (p, q),
        None => (after, ""),
    };

    if host == "youtu.be" {
        let id = path.trim_start_matches('/').split('/').next().unwrap_or("");
        return as_id(id).map(str::to_string).ok_or(LinkError::NoVideo);
    }
    if !HOSTS.contains(&host.as_str()) {
        return Err(LinkError::NotYouTube);
    }
    if path == "/watch" || path == "/watch/" {
        return query_v(query)
            .and_then(as_id)
            .map(str::to_string)
            .ok_or(LinkError::NoVideo);
    }
    ID_PATHS
        .iter()
        .find_map(|prefix| path.strip_prefix(prefix))
        .map(|tail| tail.split('/').next().unwrap_or(""))
        .and_then(as_id)
        .map(str::to_string)
        .ok_or(LinkError::NoVideo)
}

/// The watch page for a video id, the address the page offers in a new tab.
pub fn watch_address(id: &str) -> String {
    format!("https://www.youtube.com/watch?v={id}")
}

#[cfg(test)]
mod tests {
    use super::*;

    const ID: &str = "aqz-KE-bpKQ";

    fn id(link: &str) -> Result<String, LinkError> {
        video_id(link)
    }

    #[test]
    fn the_forms_people_copy_all_name_the_video() {
        for link in [
            "https://www.youtube.com/watch?v=aqz-KE-bpKQ",
            "http://youtube.com/watch?v=aqz-KE-bpKQ",
            "youtube.com/watch?v=aqz-KE-bpKQ",
            "www.youtube.com/watch?v=aqz-KE-bpKQ",
            "https://m.youtube.com/watch?v=aqz-KE-bpKQ",
            "https://music.youtube.com/watch?v=aqz-KE-bpKQ&si=abc",
            "https://www.youtube.com/watch?list=PL123&v=aqz-KE-bpKQ&t=42s",
            "https://youtu.be/aqz-KE-bpKQ",
            "https://youtu.be/aqz-KE-bpKQ?si=xyz&t=10",
            "https://www.youtube.com/shorts/aqz-KE-bpKQ",
            "https://www.youtube.com/embed/aqz-KE-bpKQ?start=3",
            "https://www.youtube-nocookie.com/embed/aqz-KE-bpKQ",
            "https://www.youtube.com/live/aqz-KE-bpKQ?feature=share",
            "  HTTPS://WWW.YouTube.COM/watch?v=aqz-KE-bpKQ#comments  ",
        ] {
            assert_eq!(id(link), Ok(ID.to_string()), "{link}");
        }
    }

    #[test]
    fn a_page_that_names_no_single_video_is_refused() {
        for link in [
            "https://www.youtube.com/playlist?list=PL123",
            "https://www.youtube.com/@somechannel",
            "https://www.youtube.com/watch?v=short",
            "https://www.youtube.com/watch?v=aqz-KE-bpKQX",
            "https://www.youtube.com/watch?v=aqz-KE-bp%51",
            "https://www.youtube.com/watch",
            "https://youtu.be/",
            "https://www.youtube.com/",
        ] {
            assert_eq!(id(link), Err(LinkError::NoVideo), "{link}");
        }
    }

    #[test]
    fn an_address_on_another_site_is_refused() {
        for link in [
            "https://vimeo.com/123456",
            "https://notyoutube.com/watch?v=aqz-KE-bpKQ",
            "https://youtube.com.example.org/watch?v=aqz-KE-bpKQ",
            "aqz-KE-bpKQ",
            "",
        ] {
            assert_eq!(id(link), Err(LinkError::NotYouTube), "{link}");
        }
    }

    #[test]
    fn the_watch_address_is_the_plain_watch_page() {
        assert_eq!(watch_address(ID), "https://www.youtube.com/watch?v=aqz-KE-bpKQ");
    }
}
