# Handoff

Stage 9, written by the agent. `C5 Consolidate in your own words`. A fresh session starts from this file, not from memory.

## What this is

A static page that records the sound of a browser tab playing a YouTube video and saves it as an MP3, encoded inside the page by a Rust core compiled to WebAssembly. It has no server and makes no off-origin request. It works where a browser shares a tab's sound: Chrome and Edge on a computer. It lives in `~/Documents/YoutubeToMp3Converter` and is not published (`SECOND-ORDER.md` R1.9).

## Rules that hold it up

| Rule | Guard |
| --- | --- |
| The core never names a float type; samples are read as bit patterns | `core_source_has_no_io_and_no_nondeterminism`; `pcm.rs` tests |
| Silence before the first sound and after the last is left out; a pause in between is kept | `recorder.rs` tests; break rows 1 and 2 |
| `feed` returns `screen.draw` on the first sound, or the page never leaves "Waiting for sound" | `sharing_waits_for_sound_then_records`; `capture.py` |
| Effects belong to the action that asked for them | `saving_asks_the_page_to_save_once`; break row 19 |
| The shim holds the handle and decides nothing | `shim_decides_nothing` |
| The page asks nothing of any other origin | `tests/browser/check.py`, `capture.py`, the CSP test |
| `capture.py` launches Chromium without `--mute-audio`, or a shared tab records silence | `capture.py` itself fails |
| The worklet URL is stamped by the build | `scripts/build.sh` fails on an unstamped `addModule` |

## Shape of the code

| Where | What |
| --- | --- |
| `crates/core/src/pcm.rs` | sample bit patterns to integer levels; interleaving |
| `crates/core/src/recorder.rs` | trimming, stopping, the encoder |
| `crates/core/src/id3.rs`, `format.rs`, `link.rs` | tags; time, sizes and file names; YouTube links |
| `crates/core/src/api.rs` | `Tool`: screens, `act`, `feed`, `live`, `file`; `Handle` for the shim |
| `crates/core/src/page.rs` | the strings file's shape and `fill` |
| `web/app.js` | renderer, capture, save |
| `web/capture-worklet.js` | hands sample blocks to the page unchanged |
| `data/strings.json`, `data/capture.json` | reader-visible strings; browser options and the two page timings |

## Commands

| Command | Prints when it passes |
| --- | --- |
| `cargo test --workspace` | `test result: ok` for each target |
| `./scripts/check.sh` | `BUILDER CHECK PASSED` |
| `python3 tests/browser/capture.py` | `CAPTURE CHECK PASSED in chromium` |
| `python3 scripts/break_tests.py` | `20 of 20 breaks caught` |
| `python3 scripts/serve.py` | serves `dist/` at `http://127.0.0.1:8000/` for trying by hand |

## What will bite within the hour

- `feed` runs hundreds of times a second; anything added to it must stay cheap, and the page only redraws when `feed` returns an action.
- The page's text boxes are kept across redraws by id (`field-<action>`). Renaming an action renames the box and loses the reader's focus while typing.
- The browser check never opens the picker. Only `capture.py` exercises recording, and only in Chromium.
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

`./scripts/check.sh` on 2026-10-13: `BROWSER CHECK PASSED in chromium, firefox, webkit`, `CAPTURE CHECK PASSED in chromium`, `BUILDER CHECK PASSED`. `./scripts/count-tests.sh`: 90 passed, 0 failed, 0 ignored. `scripts/break_tests.py`: 20 of 20 breaks caught.

## The next action

The person tries the page by hand in Chrome with a real YouTube video played to its end (R1.14), then answers the open questions in `PLAN.md`: the encoder's approval, where to publish, and the strings.
