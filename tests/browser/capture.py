#!/usr/bin/env python3
"""Recording check: the built page records a real tab in Chromium, both ways.

A person's click on the browser's share picker cannot be scripted, so this
check starts Chromium with --auto-select-tab-capture-source-by-title, which
answers the picker with the tab of that title. The tab is a fixture
(fixtures/tone.html) that plays 440 Hz on the left and 1000 Hz on the right
for TONE_SECONDS, then falls silent.

It drives the page as a person would: choose the tab, play the video, wait for
the recording to stop by itself, type a title, save. Then it reads the saved
file: the ID3 tag, the MP3 frame header, and, where macOS's afconvert is
present, the decoded sound's length and pitch on each channel.

What a pass shows: in Chromium on this machine, the path from a shared tab to
a saved MP3 works, and the file holds the fixture's sound, trimmed. It does
not show that Chrome or Edge as installed by a reader answer the picker the
same way, or that a YouTube tab behaves like the fixture; those are a
person's checks (HANDOFF.md).

The second half records the video inside the page. Chromium starts with
--auto-accept-this-tab-capture, which answers "share this tab" with yes. The
player's address is routed to a stand-in (fixtures/player.html) that speaks
YouTube's widget protocol and plays the same tone, so no request leaves the
machine: what this shows is that the page follows the protocol as the core
reads it, not that YouTube still speaks it (a person's check, HANDOFF.md).

    python3 tests/browser/capture.py
"""
import functools
import http.server
import pathlib
import shutil
import struct
import subprocess
import sys
import tempfile
import threading
import wave

from playwright.sync_api import sync_playwright

ROOT = pathlib.Path(__file__).resolve().parents[2]
DIST = ROOT / "dist"
FIXTURES = pathlib.Path(__file__).resolve().parent / "fixtures"
TONE_SECONDS = 2
TITLE = "Test tone: left/right"
PLAYER = "https://www.youtube-nocookie.com"
WANT_NAME = "Test tone leftright.mp3"  # ":" and "/" dropped, worked out by hand

failures = []


def check(ok, what):
    print(("  PASS " if ok else "  FAIL ") + what)
    if not ok:
        failures.append(what)


class Handler(http.server.SimpleHTTPRequestHandler):
    extensions_map = {
        **http.server.SimpleHTTPRequestHandler.extensions_map,
        ".wasm": "application/wasm",
        ".js": "text/javascript",
        ".json": "application/json",
    }

    def log_message(self, *args):
        pass


def serve(directory):
    server = http.server.ThreadingHTTPServer(("127.0.0.1", 0), functools.partial(Handler, directory=str(directory)))
    threading.Thread(target=server.serve_forever, daemon=True).start()
    return server, f"http://127.0.0.1:{server.server_address[1]}"


def frequency(samples, rate):
    """Rising zero crossings a second over the middle of the samples."""
    middle = samples[len(samples) // 4: 3 * len(samples) // 4]
    rising = sum(1 for a, b in zip(middle, middle[1:]) if a < 0 <= b)
    return rising * rate / len(middle)


def read_file(path, title=TITLE):
    data = path.read_bytes()
    check(data[:3] == b"ID3", "the file starts with an ID3 tag")
    size = (data[6] << 21) | (data[7] << 14) | (data[8] << 7) | data[9]
    check(title.encode("latin-1") in data[: 10 + size], f"the tag holds the title {title!r}")
    frame = data[10 + size:]
    check(frame[:2] == b"\xff\xfb", "an MPEG-1 Layer III frame follows the tag")
    # Third header byte: bitrate index 1011 (192 kbps), rate index 01 (48 kHz).
    check(frame[2] & 0xFC == 0xB4, "the frame is 192 kbps at 48 kHz")

    if not shutil.which("afconvert"):
        print("  (afconvert not found: the decoded sound is not checked here)")
        return
    wav = path.with_suffix(".wav")
    subprocess.run(["afconvert", "-f", "WAVE", "-d", "LEI16", str(path), str(wav)], check=True)
    with wave.open(str(wav)) as w:
        rate, n = w.getframerate(), w.getnframes()
        samples = struct.unpack(f"<{n * 2}h", w.readframes(n))
    seconds = n / rate
    print(f"  decoded: {n} frames at {rate} Hz = {seconds:.3f} s")
    # The tone is TONE_SECONDS long; silence before and after is trimmed, so
    # the file is that long plus at most one MP3 frame of padding (24 ms) and
    # the encoder's start-up delay.
    check(TONE_SECONDS - 0.05 <= seconds <= TONE_SECONDS + 0.1, f"the file is the tone's length ({seconds:.3f} s)")
    left, right = samples[0::2], samples[1::2]
    fl, fr = frequency(left, rate), frequency(right, rate)
    check(abs(fl - 440) < 5, f"the left channel is 440 Hz ({fl:.1f})")
    check(abs(fr - 1000) < 5, f"the right channel is 1000 Hz ({fr:.1f})")
    peak = max(abs(s) for s in left) / 32768
    check(0.3 < peak < 0.5, f"the left channel's level is the tone's 0.4 ({peak:.3f})")


def record_here(p, tool, out):
    print("\n[chromium] the video inside the page")
    fixture = (FIXTURES / "player.html").read_text()
    seen = {"unrouted": [], "errors": [], "console": []}
    browser = p.chromium.launch(
        channel="chromium",
        ignore_default_args=["--mute-audio"],
        args=["--auto-accept-this-tab-capture"],
    )
    context = browser.new_context(accept_downloads=True)

    def route(r):
        if r.request.url.startswith(PLAYER + "/embed/"):
            r.fulfill(status=200, content_type="text/html", body=fixture)
        elif r.request.url.startswith((tool + "/", "blob:", "data:")):
            r.continue_()
        else:
            seen["unrouted"].append(r.request.url)
            r.abort()

    context.route("**/*", route)
    page = context.new_page()
    page.on("pageerror", lambda e: seen["errors"].append(str(e)))
    page.on("console", lambda m: seen["console"].append(m.text) if m.type == "error" else None)
    page.goto(tool + "/index.html")
    page.wait_for_selector("body[data-ready='true']", state="attached")
    check(page.locator("#player iframe").count() == 0, "no player is loaded before the reader asks")

    page.locator("#field-link\\.set").fill("https://youtu.be/aqz-KE-bpKQ")
    page.get_by_role("button", name="Record it here").click()
    page.wait_for_selector("body[data-screen='recording']", timeout=15000)
    check(True, "this tab was shared, the player started, and its sound started the recording")
    src = page.get_attribute("#player iframe", "src")
    check(src.startswith(PLAYER + "/embed/aqz-KE-bpKQ?enablejsapi=1&origin=http%3A%2F%2F127.0.0.1%3A"), f"the player is the video's embed ({src})")
    page.wait_for_selector("#progress-reading:has-text('until the MP3 is ready')", timeout=5000)
    check(True, f"the time left is shown ({page.inner_text('#progress-reading')})")
    page.wait_for_selector("body[data-screen='done']", timeout=15000)
    done = page.inner_text("#done")
    check("Recording stopped when the video ended." in done, "the recording stopped when the video ended")
    check(page.locator("#player iframe").count() == 0, "the player is gone once the recording ends")
    check(page.input_value("#field-tag\\.title") == "Fixture tone", "the title is the video's")
    with page.expect_download() as info:
        page.get_by_role("button", name="Save the MP3").click()
    download = info.value
    check(download.suggested_filename == "Fixture tone.mp3", f"the file is named from the video ({download.suggested_filename})")
    saved = out / "here.mp3"
    download.save_as(saved)
    read_file(saved, "Fixture tone")

    page.get_by_role("button", name="Record another").click()
    page.locator("#field-link\\.set").fill("https://youtu.be/blockedvid0")
    page.get_by_role("button", name="Record it here").click()
    page.wait_for_selector("#message:has-text('does not let it play on other sites')", timeout=15000)
    check(page.get_attribute("body", "data-screen") == "setup", "a video kept off other sites goes back to the start with the way round")
    check(page.locator("#player iframe").count() == 0, "and its player is removed")

    check(not seen["unrouted"], f"no request left the machine {seen['unrouted']}")
    check(not seen["errors"], f"no uncaught page error {seen['errors']}")
    check(not seen["console"], f"no console error {seen['console']}")
    browser.close()


def main():
    if not (DIST / "index.html").exists():
        sys.exit("dist/ is missing: run scripts/build.sh first")
    tool_server, tool = serve(DIST)
    tone_server, tone = serve(FIXTURES)
    out = pathlib.Path(tempfile.mkdtemp(prefix="capture-check-"))
    seen = {"foreign": [], "errors": [], "console": []}
    with sync_playwright() as p:
        # Playwright mutes Chromium by default, and a muted tab shares
        # silence, so the switch is left out.
        browser = p.chromium.launch(
            channel="chromium",
            ignore_default_args=["--mute-audio"],
            args=[
                "--auto-select-tab-capture-source-by-title=Tone fixture",
                "--autoplay-policy=no-user-gesture-required",
            ],
        )
        context = browser.new_context(accept_downloads=True)
        tone_page = context.new_page()
        tone_page.goto(tone + "/tone.html")
        page = context.new_page()
        page.on("request", lambda r: None if r.url.startswith((tool + "/", "blob:", "data:")) else seen["foreign"].append(r.url))
        page.on("pageerror", lambda e: seen["errors"].append(str(e)))
        page.on("console", lambda m: seen["console"].append(m.text) if m.type == "error" else None)
        page.goto(tool + "/index.html")
        page.wait_for_selector("body[data-ready='true']", state="attached")

        page.get_by_role("button", name="Choose the tab").click()
        page.wait_for_selector("body[data-screen='waiting']", timeout=15000)
        check(True, "the picker shared the fixture tab and the page waits for sound")
        check(tone_page.evaluate(f"play({TONE_SECONDS})") == "running", "the fixture tab plays its tone")
        page.wait_for_selector("body[data-screen='recording']", timeout=10000)
        check(True, "the first sound started the recording")
        page.wait_for_selector("body[data-screen='done']", timeout=(TONE_SECONDS + 15) * 1000)
        done = page.inner_text("#done")
        check("Recording stopped after 3 seconds of silence" in done, "the recording stopped by itself on silence")
        check("0:02 of sound at 192 kbps" in done, "the done screen gives the tone's length")

        page.locator("#field-tag\\.title").fill(TITLE)
        with page.expect_download() as info:
            page.get_by_role("button", name="Save the MP3").click()
        download = info.value
        check(download.suggested_filename == WANT_NAME, f"the file is named from the title ({download.suggested_filename})")
        saved = out / "saved.mp3"
        download.save_as(saved)
        read_file(saved)

        check(not seen["foreign"], f"the page made no off-origin request {seen['foreign']}")
        check(not seen["errors"], f"no uncaught page error {seen['errors']}")
        check(not seen["console"], f"no console error {seen['console']}")
        browser.close()

        record_here(p, tool, out)
    tool_server.shutdown()
    tone_server.shutdown()
    if failures:
        print(f"\nCAPTURE CHECK FAILED: {len(failures)} problem(s)")
        for f in failures:
            print("  - " + f)
        sys.exit(1)
    print("\nCAPTURE CHECK PASSED in chromium")


if __name__ == "__main__":
    main()
