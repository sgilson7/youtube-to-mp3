//! The fence around the video player, the tool's one declared off-origin
//! feature (Builder requirement R7, after `kits/online`).
//!
//! The player loads from one origin, named once in the core as
//! `player::ORIGIN`. Each page's Content-Security-Policy lets frames come from
//! that origin and no other, names no other address in any directive, and
//! still keeps connections on the page's own origin. `boundary.rs` lets the
//! CSP line hold an address because these tests hold it to this list.
//!
//! When the player seems to need another origin, stop and say so in a
//! worklist row; do not widen the list to make the check pass.

use std::fs;
use std::path::{Path, PathBuf};

use tool_core::player;

fn web_pages() -> Vec<PathBuf> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../web");
    let mut out = Vec::new();
    let mut stack = vec![root];
    while let Some(d) = stack.pop() {
        for entry in fs::read_dir(&d).expect("read web/") {
            let p = entry.expect("dir entry").path();
            if p.is_dir() {
                stack.push(p);
            } else if p.extension().is_some_and(|x| x == "html") {
                out.push(p);
            }
        }
    }
    out.sort();
    out
}

/// The content of the first CSP meta element.
fn csp_of(html: &str) -> Option<String> {
    let lower = html.to_ascii_lowercase();
    let at = lower.find("http-equiv=\"content-security-policy\"")?;
    let tag_start = lower[..at].rfind('<')?;
    let tag_end = at + lower[at..].find('>')?;
    let tag = &html[tag_start..tag_end];
    let c = tag.find("content=\"")? + "content=\"".len();
    let rest = &tag[c..];
    Some(rest[..rest.find('"')?].to_string())
}

/// Each directive's name and its sources.
fn directives(csp: &str) -> Vec<(String, Vec<String>)> {
    csp.split(';')
        .map(str::trim)
        .filter(|d| !d.is_empty())
        .map(|d| {
            let mut words = d.split_whitespace().map(str::to_string);
            let name = words.next().unwrap_or_default();
            (name, words.collect())
        })
        .collect()
}

/// A source that reaches another origin: an address, a network scheme, or
/// anything at all. `data:` and `blob:` hold content made on the page.
fn names_another_origin(source: &str) -> bool {
    let local = ["data:", "blob:"].contains(&source);
    !local && (source.contains("://") || source.ends_with(':') || source == "*")
}

/// What is wrong with a page's CSP, as sentences; empty when it is right.
fn fence_problems(csp: &str) -> Vec<String> {
    let mut problems = Vec::new();
    let dirs = directives(csp);
    match dirs.iter().find(|(name, _)| name == "frame-src") {
        Some((_, sources)) if sources == &[player::ORIGIN.to_string()] => {}
        Some((_, sources)) => problems.push(format!(
            "frame-src is {sources:?}, not exactly the player's origin {}",
            player::ORIGIN
        )),
        None => problems.push("no frame-src: the player could not load".into()),
    }
    for (name, sources) in &dirs {
        if name == "frame-src" {
            continue;
        }
        for s in sources.iter().filter(|s| names_another_origin(s)) {
            problems.push(format!("{name} names another origin: {s}"));
        }
    }
    match dirs.iter().find(|(name, _)| name == "connect-src") {
        Some((_, sources)) if sources == &["'self'".to_string()] => {}
        _ => problems.push("connect-src is not exactly 'self'".into()),
    }
    problems
}

#[test]
fn every_page_frames_only_the_player_s_origin() {
    let pages = web_pages();
    assert!(!pages.is_empty(), "no HTML page found in web/");
    for path in pages {
        let text = fs::read_to_string(&path).expect("read html");
        let csp = csp_of(&text).unwrap_or_else(|| panic!("{}: no CSP", path.display()));
        let problems = fence_problems(&csp);
        assert!(problems.is_empty(), "{}: {}", path.display(), problems.join("; "));
    }
}

#[test]
fn the_player_origin_is_one_https_origin() {
    let o = player::ORIGIN;
    assert!(o.starts_with("https://"), "{o}");
    assert!(!o["https://".len()..].contains(['/', '*', '?', '#']), "{o}");
}

#[test]
fn the_fence_catches_each_kind_of_widening() {
    let good = format!(
        "default-src 'self'; img-src 'self' data:; connect-src 'self'; frame-src {}",
        player::ORIGIN
    );
    assert!(fence_problems(&good).is_empty(), "{:?}", fence_problems(&good));
    for bad in [
        "default-src 'self'; connect-src 'self'".to_string(),
        format!(
            "default-src 'self'; connect-src 'self'; frame-src {} https://evil.example",
            player::ORIGIN
        ),
        "default-src 'self'; connect-src 'self'; frame-src https:".to_string(),
        "default-src 'self'; connect-src 'self'; frame-src *".to_string(),
        format!(
            "default-src 'self'; connect-src 'self' https://evil.example; frame-src {}",
            player::ORIGIN
        ),
        format!(
            "default-src 'self'; img-src https://i.ytimg.com; connect-src 'self'; frame-src {}",
            player::ORIGIN
        ),
        format!("default-src 'self'; frame-src {}", player::ORIGIN),
    ] {
        assert!(!fence_problems(&bad).is_empty(), "passed: {bad}");
    }
}
