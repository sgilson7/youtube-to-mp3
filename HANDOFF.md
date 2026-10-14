# Handoff

Stage 9, written by the agent. `C5 Consolidate in your own words`. A fresh session starts from this file, not from memory.

## What this is

A static page that records the sound of a browser tab playing a YouTube video and saves it as an MP3, encoded inside the page by a Rust core compiled to WebAssembly. It has no server. With a link, the video plays inside the page in YouTube's embedded player, the page records its own tab, shows the time left, and stops when the video ends; that frame is the one off-origin request, loaded only on the reader's press (`SECOND-ORDER.md` R3.1). Without a link, the reader shares the tab where the video plays. "Record silently" keeps the sound off the speakers. It works where a browser shares a tab's sound: Chrome, Edge and Opera on a computer. Source: `https://github.com/sgilson7/youtube-to-mp3`; page: `https://sgilson7.github.io/youtube-to-mp3/`, published from the `gh-pages` branch by `./scripts/publish.sh`.

## Rules that hold it up

| Rule | Guard |
| --- | --- |
| The core never names a float type; samples are read as bit patterns | `core_source_has_no_io_and_no_nondeterminism`; `pcm.rs` tests |
| Silence before the first sound and after the last is left out; a pause in between is kept | `recorder.rs` tests; break rows 1 and 2 |
| `feed` returns `screen.draw` on the first sound, or the page never leaves "Waiting for sound" | `sharing_waits_for_sound_then_records`; `capture.py` |
| Effects belong to the action that asked for them | `saving_asks_the_page_to_save_once`; break row 19 |
| The shim holds the handle and decides nothing | `shim_decides_nothing` |
| The page asks nothing of any other origin until the reader asks for the player | `tests/browser/check.py`, `capture.py`, the CSP test |
| Frames come only from `player::ORIGIN`, and the CSP names no other address | `crates/core/tests/player_fence.rs` |
| Only the player's own messages are read, and play is sent once, after the tab is shared | `player.rs` and `api.rs` tests; break rows |
| `capture.py` launches Chromium without `--mute-audio`, or a shared tab records silence | `capture.py` itself fails |
| The worklet URL is stamped by the build | `scripts/build.sh` fails on an unstamped `addModule` |

## Shape of the code

| Where | What |
| --- | --- |
| `crates/core/src/pcm.rs` | sample bit patterns to integer levels; interleaving |
| `crates/core/src/recorder.rs` | trimming, stopping, the encoder |
| `crates/core/src/id3.rs`, `format.rs`, `link.rs` | tags; time, sizes and file names; YouTube links |
| `crates/core/src/player.rs` | the embed address, the widget protocol's messages, times read without floats |
| `crates/core/src/api.rs` | `Tool`: screens, `act`, `feed`, `live`, `file`; `Handle` for the shim |
| `crates/core/src/page.rs` | the strings file's shape and `fill` |
| `web/app.js` | renderer, player frame and its messages, capture, save |
| `web/capture-worklet.js` | hands sample blocks to the page unchanged |
| `data/strings.json`, `data/capture.json` | reader-visible strings; browser options and the two page timings |

## Commands

| Command | Prints when it passes |
| --- | --- |
| `cargo test --workspace` | `test result: ok` for each target |
| `./scripts/check.sh` | `BUILDER CHECK PASSED` |
| `python3 tests/browser/capture.py` | `CAPTURE CHECK PASSED in chromium` |
| `python3 scripts/break_tests.py` | `37 of 37 breaks caught` |
| `python3 scripts/serve.py` | serves `dist/` at `http://127.0.0.1:8000/` for trying by hand |

## What will bite within the hour

- `feed` runs hundreds of times a second; anything added to it must stay cheap, and the page only redraws when `feed` returns an action.
- The page's text boxes are kept across redraws by id (`field-<action>`). Renaming an action renames the box and loses the reader's focus while typing.
- The browser check never opens the picker or the player. Only `capture.py` exercises recording, and only in Chromium.
- The player frame must not be moved or rebuilt: that reloads the video. It sits in `#player`, outside the panels, and is redrawn only when the core's description of it changes.
- YouTube's widget messages are not documented by YouTube (R3.2). If the bar stops moving or the video never starts, compare a real embed's messages with the fixtures in `player.rs`.
- The glossary test counts digits in names such as MP3 as bare numbers (R1.6).

## Mistakes so far

| System | Mistake | Rule it produced |
| --- | --- | --- |
| core | tabs in titles glued words; capital `WWW.` refused (R1.7) | tests with whitespace and case variants stay |
| core | effects piled up across actions (R1.8) | effects are cleared at the start of `act` |
| page and core | first sound did not redraw (R1.10) | `feed` names the redraw; the recording check covers it |
| page | redraw blurred the text box (R1.12) | focus and caret are noted before a rebuild |
| test | headless Chromium was muted (R1.11) | `capture.py` drops `--mute-audio` |

## What passed

`./scripts/check.sh` on 2026-10-14, after the video inside the page and silent recording: `BROWSER CHECK PASSED in chromium, firefox, webkit`, `CAPTURE CHECK PASSED in chromium` (both ways), `BUILDER CHECK PASSED`. `./scripts/count-tests.sh`: 112 passed, 0 failed, 0 ignored. `scripts/break_tests.py`: 37 of 37 breaks caught. A real YouTube embed recorded by hand-run script (R3.3).

Earlier: `./scripts/check.sh` on 2026-10-13: `BROWSER CHECK PASSED in chromium, firefox, webkit`, `CAPTURE CHECK PASSED in chromium`, `BUILDER CHECK PASSED`. `./scripts/count-tests.sh`: 90 passed, 0 failed, 0 ignored. `scripts/break_tests.py`: 20 of 20 breaks caught. Published 2026-10-13 (build `2d90fecfbff1` from `44153d3`); the browser check against the live address passed in all three engines. The recording check has not been run against the live address, since it serves `dist/` itself; the live build hash matches the one it checked.

## The next action

The person tries "Record it here" in Chrome or Opera with a real video played to its end, listening for silence (R3.5) and reading Chrome's share prompt against the strings (R3.8); then reads the round 3 strings at the foot of `SECOND-ORDER.md`. The encoder's approval (R1.3) is still open.
