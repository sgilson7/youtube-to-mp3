# Measurements

Stage 4, recorded by the agent before building and again at handoff. `B2 Require step-by-step justification`.

The state measured rather than recalled, so later changes can be compared with it. A probe prints numbers and asserts loose bounds at most; record its output here with the command that produced it.

| What | Command | Result | When |
| --- | --- | --- | --- |
| Toolchain | `rustc --version; wasm-bindgen --version` | rustc 1.99.0; wasm-bindgen 0.2.127 | 2026-10-13 |
| Encoder before adoption | a scratch crate encoding 5 s of 440 Hz left, 1000 Hz right at 48 kHz, decoded with `afconvert` | 128, 192 and 256 kbps each decoded at 440 Hz and 1000 Hz with peaks 0.50 and 0.30 (input 0.5 and 0.3); about 40 ms to encode 5 s natively; 200 KB as WebAssembly | 2026-10-13 |
| Core tests | `./scripts/count-tests.sh` | 90 passed, 0 failed, 0 ignored | 2026-10-13 |
| Full check | `./scripts/check.sh` | BUILDER CHECK PASSED (Chromium, Firefox, WebKit; recording check in Chromium) | 2026-10-13 |
| WebAssembly size | printed by `./scripts/build.sh` | 404 KB | 2026-10-13 |
| Breaks caught | `python3 scripts/break_tests.py` | 20 of 20 | 2026-10-13 |
| Recording check | `python3 tests/browser/capture.py` | a 2 s tone decoded as 2.016 s, 440.5 Hz left, 1000.0 Hz right, peak 0.400 | 2026-10-13 |
| Live page | `BUILDER_ORIGIN=https://sgilson7.github.io/youtube-to-mp3 python3 tests/browser/check.py` | BROWSER CHECK PASSED in chromium, firefox, webkit; build `2d90fecfbff1` from `44153d3` | 2026-10-13 |
| Real YouTube, once | a probe outside the repository (`SECOND-ORDER.md` R1.13) | 15.3 s at 192 kbps, stereo, RMS 0.05 to 0.09 | 2026-10-13 |

## Not measured

- Chrome and Edge as installed, with a person answering the real picker.
- Windows and Linux. The recording check ran on macOS only.
- Recordings longer than 15 seconds: memory and encoding time for a full song or an hour-long video.
- How long the encoder takes in WebAssembly on a slow computer.
- Safari and Firefox sharing a tab: the browser check loads the page in their engines but never opens the picker.
