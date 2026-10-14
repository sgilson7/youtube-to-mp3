# Brief

Stage 1. The person's own words come first; the rest can be written by the agent from them and read by the person. `C1 Frame the problem and set the rules`, `B1 Establish context`.

## The description, word for word

> Use sgilson7.github.io builder setting, which can be found one directory up. Use the shared technical setting within to create the following tool for me: a youtube to mp3 converter that works like the pdf redactor, with no terrible ads or anything like using other open source youtube to mp3 converters

Asked whether the tool should be a page, a local app or both:

> browser page, link in, mp3 out, i dont care about youtube changes and their terms, i just actively want a version of an open source youtube to mp3 converter and release it online so other people dont have to use the terrible ad filled virus filled versions that steal all the data online

Told that a static page cannot read YouTube's streams, and asked about a way round it:

> wait so why cant the browser read youtubes stream directly? lets get a little creative and try and figure that out. can the host create a faux server all in the browser cache and use that to pull the youtube stream?

Offered a page that records a tab's sound, or a local app that takes a link:

> lets try the creative route fist

Told on 2026-10-14 why a page cannot fetch the audio, and offered silent recording, the video inside the page (which breaks this brief's "no off-origin request"), an extension, or the local app:

> add silent recording and video inside the page, and a little progress bar with a time left before its ready

## The tool

A static web page that turns the sound of a YouTube video into an MP3 file, built and kept entirely inside the reader's browser tab, like the PDF redactor. The reader opens the page, chooses the browser tab playing the video, shares that tab's sound, plays the video, and stops. The page encodes what it heard as an MP3 and offers it as a file to save. There is no server, no upload, no account, no advert and no request off the page's own origin.

The page records rather than downloads, because a page on another origin cannot read YouTube's streams (`SECOND-ORDER.md` R1.1). Recording takes as long as the video plays, and it works where the browser can share a tab's sound: Chrome and Edge on a desktop computer.

With a link, the video can instead play inside the page, in YouTube's embedded player, and the page records its own tab: a bar shows the time left, and recording ends with the video. The player is the tool's one declared off-origin feature (Builder R7): it loads only when the reader presses the button for it. By default the sound is recorded without playing aloud (`suppressLocalAudioPlayback`), so the reader can do other things meanwhile.

For anyone with a desktop browser; aimed at people who would otherwise use an advert-funded converter site.

## Done means

- In Chrome on macOS, sharing a tab that plays a test tone for a few seconds gives an MP3 that macOS decodes, at the chosen bitrate, with the tone's pitch.
- Silence before the sound starts and after it stops is left out of the file.
- With "stop when the sound stops" on, recording ends by itself once the tab has been silent for the stated time after sound was heard.
- The title and artist typed in are written into the file's tags, and the file is named from the title.
- Sharing without the tab's sound gives a message that says to tick the box for the tab's sound, not an empty file.
- A browser that cannot share a tab's sound is told which browsers can.
- With a link, the video plays inside the page, starts by itself once this tab is shared, shows the time left, and ends the recording when it ends; the file takes the video's title.
- A video whose owner keeps it off other sites sends the reader back with the other way to record it.
- No request leaves the page's origin until the reader asks for the video inside the page, and the CSP lets frames come from the player's one origin only.
- The browser check passes in Chromium, Firefox and WebKit with no off-origin request (it never asks for the player).
- `./scripts/check.sh` passes.

## Decisions that stay with the person

- Every reader-visible string (listed at the foot of `SECOND-ORDER.md`).
- The note on the page about what readers may record.
- Whether and where the tool is published (see Autonomy).
- Whether to build the local app with a link box later as a second tool (`DECISIONS.md`, 2026-10-13).

## Accessibility

Every state is text: the screen's heading names it, elapsed time is written as minutes and seconds, the sound level has a text reading beside the bar, and errors are sentences in a live region. Every control is a native button, input or link, so it works with a keyboard and a screen reader.

## Style

`default`.

## Words

YouTube, MP3, tab, bitrate. Entries in `GLOSSARY.md`.

## Autonomy

The plan is approved in advance. Where this brief is silent, the agent decides, records the decision in `SECOND-ORDER.md`, and continues. The agent may not loosen a Builder requirement or a boundary test, and may not add a server, an account, analytics or any off-origin request other than the video player the person asked for on 2026-10-14 (its fence is `crates/core/tests/player_fence.rs`). The person wrote: "i approve crestion of a public git repo and deployment to pages". The agent may create the public repository `sgilson7/youtube-to-mp3` and publish it to GitHub Pages with `./scripts/publish.sh` once `./scripts/check.sh` passes, and may republish after later changes that pass the check, until the person withdraws this.

## Allowed core dependencies

`serde`, `serde_json` (reference default), and:

- `rusty_mp3 = "=1.0.0"`: the MP3 encoder. Pure Rust, Apache-2.0, no dependencies of its own, no C. Browsers have no MP3 encoder (their built-in recorder writes Opus or AAC), and writing a Layer III encoder inside this project would take far longer than the tool. Pinned exactly because the crate was nine days old when adopted. It uses `f32` inside, which D2 bans in the core's own source but not in a dependency; the core passes it bytes and never names a float type. Its notes say its 320 kbps output is garbled, so the tool offers 128, 192 and 256 kbps only. It reads tuning variables from the process environment, which on `wasm32-unknown-unknown` always reads as unset. Recorded in `BUILDER-SPEC.md`'s change table.
