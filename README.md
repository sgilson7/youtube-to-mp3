# YouTube to MP3

Record the sound of a YouTube video in a browser tab and save it as an MP3. The page records and encodes inside your browser. It has no server, no account and no adverts. The page requests nothing from any other address; the link to a video opens YouTube in a tab of its own.

## How it works

1. Open the video in a tab, or paste its link into the page and open it from there.
2. Press **Choose the tab**, pick the video's tab, and turn on the switch to share the tab's audio.
3. Play the video. Recording begins with the first sound.
4. Press **Stop and make the MP3**, or let the recording stop after three seconds of silence, then type a title and artist and save.

The page records what the tab plays, so a recording takes as long as the video. Silence before the first sound and after the last is left out. An advert that plays in the tab is recorded too.

Sharing a tab's sound works in Chrome and Edge on a computer. Firefox and Safari can share a tab's picture without its sound, and the page says so.

## Why it records instead of downloading

A web page cannot read YouTube's video streams. Browsers let a page read a response from another site only when that site allows it, and YouTube does not: a request from another origin to the API that hands out stream addresses is refused. Converter sites get around this with a server that downloads the video for you, and that server is what the adverts pay for. This page has no server, so it records the sound the tab plays, which the browser lets a page do once you choose the tab yourself.

## What happens to the sound

The browser hands the shared tab's sound to the page in blocks of samples. The page passes each block to a WebAssembly module, which keeps or drops it, encodes it as MP3, and adds the title and artist as an ID3 tag. When you save, the browser writes the file from memory. You can confirm that nothing leaves the tab by watching the Network panel of the browser's developer tools while you record.

Record only what you have the right to keep, such as your own videos or ones licensed for reuse.

## Build, test and serve

```bash
./scripts/setup.sh              # once: Python venv with Playwright and three browsers
cargo test --workspace          # core, boundary, copy and storage tests, in seconds
./scripts/build.sh              # release WebAssembly build into dist/
python3 scripts/serve.py        # http://127.0.0.1:8000/
./scripts/check.sh              # everything, before publishing
python3 tests/browser/capture.py   # records a test tone from a real tab in Chromium
python3 scripts/break_tests.py  # each row of tests/breaks.json must make its test fail
```

Needs Rust (pinned in `rust-toolchain.toml`), `wasm-bindgen-cli` 0.2.127, and Python 3.

## Layout

| Path | Holds |
| --- | --- |
| `crates/core` | every rule: reading samples, trimming silence, stopping, encoding, tags, file names, links, the screens; no input or output |
| `crates/shim` | the `wasm-bindgen` class the page holds; each method is one call into the core |
| `web/` | the static page: a generic renderer, the tab capture, and the audio worklet |
| `data/` | the strings the page shows, and the options it passes to the browser |
| `GLOSSARY.md`, `TONE.md` | what each term means; how every string is written |
| `tests/browser/` | the browser check, this tool's interactions, and the recording check |
| `tests/breaks.json` | one deliberate break per rule, for `scripts/break_tests.py` |

Built on [Builder](https://github.com/sgilson7/builder-setting). The MP3 encoder is [rusty_mp3](https://crates.io/crates/rusty_mp3) (Apache-2.0).
