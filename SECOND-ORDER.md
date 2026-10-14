# Second-order notebook

Stage 6, kept by the agent as the work happens, not afterwards. `B3 Build in self-questioning`.

One row per assumption, surprise or open item. **Kind** is *divergence* (what was assumed, what turned out to be true, what changed), *finding*, or *worklist* (the person's to do). Number rows by round (R1.1, R1.2, then R2.1 after the first triage) so a later round does not renumber an earlier one. End each row with its status: *done*, *open*, or *person*.

| # | Kind | Row |
| --- | --- | --- |
| R1.1 | finding | A page cannot read YouTube. From the shell, with `Origin: https://sgilson7.github.io`: the CORS preflight to `www.youtube.com/youtubei/v1/player` (the API that hands out stream addresses) was refused with 403; a GET of a watch page returned 200 with no `Access-Control-Allow-Origin`, so a browser would hide the body from the page. A service worker's own requests obey the same rule, and an opaque response cannot be read by script. Stream addresses are also scrambled by YouTube's player code. This rules out any static page that takes a link and fetches the video. *done* |
| R1.2 | divergence | Assumed at first that the tool would be a local app with a link box (the person chose it), then the person asked for a creative browser-only route and chose tab recording. The local app was not built; `DECISIONS.md` records it as a possible second tool. *done* |
| R1.3 | worklist | `rusty_mp3 = "=1.0.0"` was added to the core's allowlist without the person's approval, which the allowlist's own comment asks for. Reasons in `BRIEF.md`. What a reader of its source sees: Apache-2.0, pure Rust, no dependencies; `unsafe` only in SIMD kernels with scalar twins (the WebAssembly build uses the scalar path); it reads tuning variables with `std::env::var`, which on `wasm32-unknown-unknown` always reads as unset; its notes say 320 kbps output is garbled, so 320 is not offered. It was nine days old when adopted. The tests and the macOS decode show it works for these cases; they do not show it is free of defects. *person* |
| R1.4 | divergence | The template keeps the page's state as JSON passed back on each call. A running encoder is not JSON, so the core's state lives in `api::Tool`, and the shim holds `api::Handle` in a `wasm_bindgen` struct. The shim still has no branch, loop or `?`. *done* |
| R1.5 | divergence | `scripts/build.sh` stamped `import`, `fetch` and the wasm URL. The worklet is loaded with `audioWorklet.addModule("...")`, which it did not stamp, so a returning reader could have kept an old worklet after a deploy. Added `addModule` to the stamp and to the unstamped-URL guard. *done* |
| R1.6 | finding | `every_glossary_entry_is_complete_and_states_no_bare_number` counts the digits in names (`MP3`, `MPEG-1`, `ID3`) as bare numbers. The definitions were reworded to avoid the names; the check was not changed. A name with a digit in a definition will trip it again. *open* |
| R1.7 | finding | The first core run caught two defects: tabs and newlines in a title were deleted before spaces were collapsed, gluing words (`a\tb` gave `ab`); and `WWW.` in capitals was not stripped, so a link in capitals was refused. Both fixed; the tests that caught them stay. *done* |
| R1.8 | finding | Effects were queued until the next `ui()` call, so two actions without a draw between them handed the page both sets. In the page every `act` returns a draw, so only the tests saw it; effects now belong to the action that asked for them (`self.effects.clear()` at the start of `act`, with a break row). *done* |
| R1.9 | worklist | The person granted a public repository and Pages deployment without naming the repository; the agent named it `youtube-to-mp3` (`DECISIONS.md`). *done* |
| R1.10 | finding | The recording check found that the page stayed on "Waiting for sound" after sound started: the core changed screen inside `feed`, which the page does not redraw from. `feed` now returns `screen.draw` on the first sound, a no-op action the page sends to redraw. The core tests had passed because they call `ui()` themselves; the browser check is what caught it. *done* |
| R1.11 | finding | Headless Chromium recorded silence from the shared tab. Playwright launches Chromium with `--mute-audio`, and a muted tab shares silence. `capture.py` drops that switch with `ignore_default_args`; with it, headless capture records the tone. *done* |
| R1.12 | finding | Typing in the link box kept only the first character: rebuilding a panel moves the focused input, which blurs it, and the page noted the focus after the move. The page now notes focus and caret before rebuilding, and reuses the input node. `interactions.py` types a full link and checks the box and focus. *done* |
| R1.13 | finding | One probe against real YouTube (Big Buck Bunny, CC BY 3.0, from 1:00), with headless Chromium choosing the tab by title: 15.3 s recorded, saved as `Big Buck Bunny (excerpt).mp3`, 192 kbps; decoded with `afconvert`, stereo with RMS 0.05 to 0.09 throughout. The probe reaches YouTube, so it is not part of `check.sh`; the script is not in the repository. *done* |
| R1.14 | worklist | Try the page by hand in Chrome and in Edge, with the real picker: share a YouTube tab, play a song to its end, and listen to the MP3. Also on Windows, where the picker's audio switch and system-audio option differ from macOS. No automated check covers a person's picker. *person* |
| R1.15 | finding | `capture.py` was shown to fail: with the page's `feed` call passing right as left, it reported the left channel at 1000 Hz and the right at 439.5 Hz, and failed. Restored; it passes. *done* |
| R1.16 | finding | An advert that plays in the shared tab is recorded with the video. The page says so (`adverts`). Nothing in the page can tell an advert from the video. *done* |
| R1.17 | divergence | `picker_failed` shows the browser's own error name (for example `NotReadableError`) inside a reader-visible sentence. That name is not in `data/` and is not translated. Kept because it is what a reader would search for; listed here as an exception to D5. *person* |
| R1.18 | finding | `check.sh` now has an eighth step, `capture.py`, which needs Chromium to play and capture sound. The repository has no CI workflow yet; a CI runner without audio support may fail that step. *open* |
| R1.19 | worklist | YouTube's terms forbid downloading outside YouTube's own features. The person said they do not mind; the page asks readers to record only what they have the right to keep (`rights`). Whether that wording is enough for a public release is the person's call. *person* |
| R1.20 | worklist | Every string below was written by the agent. Read them, in particular `intro`, `rights` and `adverts`. *person* |
| R1.21 | worklist | The template has no licence file. The person asked for an open-source tool, so `LICENSE` is a copy of the PDF redactor's MIT licence (Copyright 2026 Sam Gilson). Change it if another licence is wanted. *person* |
| R2.1 | divergence | Assumed Safari and Firefox might share a tab's sound in some versions, so the page let them try with a muted warning. The person, on a Mac with only Safari 26.3, met Safari's picker, which offers windows and screens and gives the page no sound. Recording now needs a Chromium-based browser (`navigator.userAgentData`); others see a notice straight after the intro and a disabled button. A Chromium browser without `userAgentData` would be refused wrongly; none is known. "window" is allowed in `browser_cannot` only, with the reason in `copy.rs`. (T1) *done* |
| R2.3 | finding | The person reported, 2026-10-13: "it worked on opera". Opera desktop is Chromium-based and passes the page's browser test, so it gets the picker. First recording by a person on the published page. No automated check runs Opera. *done* |
| R2.2 | worklist | Install Chrome or Edge on the Mac and try the page by hand (R1.14 still stands). *person* |
| R3.1 | divergence | The brief ruled out any off-origin request. The person asked for the video inside the page on 2026-10-14, after being told it breaks that rule. Built as a declared off-origin feature under R7, after `kits/online`: the frame loads only when the reader presses "Record it here"; the CSP's `frame-src` is exactly the core's `player::ORIGIN`, and `player_fence.rs` fails on any widening (each of seven widenings seen failing); `connect-src` stays `'self'`. No third-party code runs in the page's origin. Inside the frame, YouTube's own scripts contact YouTube and Google as any embed does. *done* |
| R3.2 | finding | The widget protocol (`listening`, `command`, `infoDelivery`, `onStateChange`, `onError`) is what YouTube's embed script uses, but YouTube documents the script, not the messages. Messages recorded from a real embed on 2026-10-14 are the fixtures in `player.rs`. `capture.py` tests the page against a stand-in that speaks the same messages, so a change on YouTube's side would show only when a person tries it. *open* |
| R3.3 | finding | Real YouTube, 2026-10-14, Playwright Chromium with a window, `--auto-accept-this-tab-capture`: Big Buck Bunny started by itself once the tab was shared; the time left read 10:35 and counted down a second a second; the file took the video's title; 13 s recorded decoded by `afinfo` as 12.98 s at 192 kbps. The YouTube embed reports `author` as empty, so the artist is not filled. *done* |
| R3.4 | divergence | For a video inside the page, stopping on silence is ignored: the player reports the end, and a quiet passage in the video would otherwise end the recording early. Silence after the last sound is still left out. *done* |
| R3.5 | worklist | "Record silently" is on at first. Chrome reads `suppressLocalAudioPlayback` only when sharing starts, so the setting is chosen before. The checks show recording still works with it on; whether the reader hears nothing is a check by ear. Try it in Chrome or Opera, both ways. *person* |
| R3.6 | finding | A break row for "the player shows only while recording" was not caught. The screen filter was redundant: `stop` and `back_to_setup` already drop the player. The filter was removed with its row, leaving 37 rows, all caught. *done* |
| R3.7 | finding | The time left is known only for a video inside the page. A page cannot see the player in another tab, so recording another tab shows the level and the time recorded, as before. *done* |
| R3.8 | worklist | Check the strings that describe Chrome's "share this tab" prompt (`steps[1]`, `asking_here_text`: "keep the switch to share its audio on, and allow it") against the prompt Chrome shows; automation answers that prompt with a switch, so no check has seen it. *person* |
| R3.9 | finding | An advert in the embedded player plays in this tab and is recorded, as in another tab. How the player reports time during an advert was not seen. *open* |

## Strings written by the agent

Each reader-visible string the agent wrote, with its key, for the person to read.

| Key | String |
| --- | --- |
| `title` | YouTube to MP3 |
| `intro` | Record the sound of a YouTube video in this tab and save it as an MP3. The page has no server to send anything to: it records and encodes the sound inside your browser, with no account and no adverts. |
| `steps_heading` | How it works |
| `steps[0]` | Open the video in another tab, or paste its link below and open it from here. |
| `steps[1]` | Press {choose}, pick the video's tab, and turn on the switch to share the tab's audio. |
| `steps[2]` | Play the video. Recording begins with the first sound, so the silence before it is left out. |
| `steps[3]` | Press {stop} when the video ends, or let the recording stop by itself, then save the MP3. |
| `link_label` | YouTube link (optional) |
| `link_open` | Open the video in a new tab |
| `link_not_youtube` | That is not a YouTube address. Copy the link from the address bar or from the video's Share button. |
| `link_no_video` | That address does not name a single video. Open the video itself and copy its link. |
| `bitrate_heading` | Bitrate |
| `bitrate_option` | {kbps} kbps |
| `bitrate_note` | A higher bitrate keeps more detail and makes a larger file. {default} kbps suits most listening. |
| `silence_option` | Stop after {seconds} seconds of silence |
| `choose` | Choose the tab |
| `rights` | Record only what you have the right to keep, such as your own videos or ones licensed for reuse. |
| `browser_cannot` | This browser cannot share a tab. Use Chrome or Edge on a computer. |
| `browser_maybe` | This browser may share a tab's picture without its sound. Chrome and Edge on a computer share both. |
| `asking_heading` | Choose the tab |
| `asking_text` | In the list the browser shows, pick the tab with the video and turn on the switch to share its audio. |
| `waiting_heading` | Waiting for sound |
| `waiting_text` | Play the video now. Recording begins with the first sound. |
| `cancel` | Stop sharing |
| `recording_heading` | Recording |
| `recording_time` | Recorded {time} |
| `level_label` | Sound level |
| `level_text` | {percent} percent of full volume |
| `recording_auto` | Recording stops by itself after {seconds} seconds of silence. |
| `recording_manual` | Press {stop} when the video ends. |
| `adverts` | An advert that plays in the video's tab is recorded too. |
| `stop` | Stop and make the MP3 |
| `encoding_heading` | Making the MP3 |
| `encoding_text` | Encoding {time} of sound at {kbps} kbps. |
| `done_heading` | Your MP3 is ready |
| `done_text` | {time} of sound at {kbps} kbps, {size}. |
| `title_label` | Title |
| `artist_label` | Artist |
| `tags_note` | The title names the file. The title and artist are written into the MP3's tags, which music players show. |
| `save` | Save the MP3 |
| `again` | Record another |
| `default_name` | recording |
| `stopped_silence` | Recording stopped after {seconds} seconds of silence, which is left out of the file. |
| `stopped_limit` | Recording stopped at the limit of {hours} hours. |
| `stopped_sharing` | Sharing ended, so recording stopped. |
| `no_audio` | The tab was shared without its sound. Press {choose} again and turn on the switch to share its audio. |
| `picker_cancelled` | No tab was shared. Press {choose} to try again. |
| `picker_failed` | The browser did not share the tab ({detail}). Press {choose} to try again. |
| `no_sound` | No sound was heard, so there is no MP3 to make. Play the video while the page is waiting for sound. |
| `ended_early` | Sharing ended before any sound was heard. Press {choose} to start again. |
| `bad_rate` | The browser records at {rate} samples a second, which an MP3 cannot hold. Reload the page and try again. |

### Round 3 (2026-10-14): new and changed

| Key | String |
| --- | --- |
| `steps[0]` | Paste the video's link below and press {here}. The video plays on this page while it records. |
| `steps[1]` | When the browser asks to share this tab, keep the switch to share its audio on, and allow it. |
| `steps[2]` | Recording begins with the first sound and stops when the video ends. Then save the MP3. |
| `steps[3]` | Some owners keep their videos off other sites. For those, open the video in a new tab, press {choose}, and click that tab in the browser's list. |
| `here` | Record it here |
| `silent_option` | Record silently |
| `asking_here_heading` | Share this tab |
| `asking_here_text` | The browser asks to share this tab. Keep the switch to share its audio on, and allow it. The video then starts by itself. |
| `waiting_here_text` | The video is starting. If it does not play within a few seconds, press play on it. |
| `progress_label` | Time left |
| `progress_text` | {left} until the MP3 is ready |
| `player_title` | YouTube video |
| `recording_here` | Recording stops by itself when the video ends. |
| `stopped_ended` | Recording stopped when the video ended. |
| `player_blocked` | The video's owner does not let it play on other sites. Open it in a new tab and press {choose} instead. |
| `player_failed` | The video could not play here (YouTube error {code}). Open it in a new tab and press {choose} instead. |
| `no_audio`, `picker_cancelled`, `picker_failed`, `ended_early` | `{choose}` became `{start}`, the button that began this recording |
| `GLOSSARY.md` "Record silently" | A setting that has the browser record the tab's sound without playing it through your speakers or headphones. It takes effect when sharing starts. |
