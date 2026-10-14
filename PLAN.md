# Plan

Stage 2, written by the agent from `BRIEF.md`. `B2 Require step-by-step justification`.

The brief approves the plan in advance, so this records what was built and why rather than waiting to be read. Each decision is in build order, with the test that shows it holds.

| # | Decision | Reason | Test that shows it holds |
| --- | --- | --- | --- |
| 1 | Record the sound of a shared tab; do not fetch from YouTube. | A page on another origin cannot read YouTube's player API or streams: a cross-origin preflight to the player API was refused with 403, and the watch page sends no `Access-Control-Allow-Origin` (R1.1). A service worker is bound by the same rule. | `tests/browser/check.py`: no off-origin request in three engines |
| 2 | Capture with `getDisplayMedia` and an `AudioWorklet` that hands raw sample bytes to the core. | The browser's own recorder writes Opus or AAC, so encoding its output to MP3 would compress the sound twice. The worklet passes the samples unchanged. | `tests/browser/capture.py`: pitch and level on each channel survive |
| 3 | Encode MP3 in the core with `rusty_mp3` at 128, 192 or 256 kbps; 192 by default. | No browser encodes MP3. The crate is pure Rust with no dependencies; 320 kbps is left out because the crate's notes say it is garbled there. | `a_recording_becomes_mp3_frames_at_the_chosen_bitrate`; macOS `afconvert` decodes the file in `capture.py` |
| 4 | Read samples as IEEE 754 bit patterns with integer arithmetic; pass bytes to the encoder. | Keeps D2 (no float type in the core) without a second copy of the sound. | `pcm.rs` tests on hand-worked bit patterns |
| 5 | Leave out silence before the first sound; hold silence after sound, keep it if sound returns, drop it at the end. | The reader starts recording before pressing play and stops after the video ends. | `silence_before_the_first_sound_is_left_out`, `a_pause_between_sounds_is_kept`, `silence_at_the_end_is_left_out_of_the_file` |
| 6 | Stop by itself after 3 seconds of silence once sound has started (on by default, can be turned off); stop at 3 hours. | The reader is in another tab while the video plays. The limit bounds memory for a forgotten recording. | `enough_silence_after_sound_stops_the_recording`, `the_recording_stops_at_the_limit` |
| 7 | Title and artist in an ID3 v2.3 tag; the file named from the title. | Version 2.3 is the one players on every common system read. | `id3.rs` tests on hand-worked bytes; `capture.py` reads the tag and the name |
| 8 | An optional link box that offers the video's watch page in a new tab. | The person asked for "link in". The link cannot be fetched (decision 1), so it opens the video for the reader to play. | `link.rs` tests; `interactions.py` checks the address and that nothing is requested |
| 9 | The shim holds the core's `Handle`; the page's state is not JSON. | The encoder is a running object (R1.4). | `shim_decides_nothing` |
| 10 | The core names effects (`capture.open`, `capture.close`, `file.save`) and a follow-up action; the page carries them out. | Keeps every decision about when to open, close, encode and save in the core. | `api.rs` flow tests |

## Open questions for the person

1. May `rusty_mp3` stay on the core's allowlist (R1.3)?
2. Where should the tool be published: which repository name, and may the agent publish it (R1.9)?
3. Do the strings at the foot of `SECOND-ORDER.md` say what you want, in particular `rights` and `intro`?

## Status

- [ ] Read by the person on: 
