"""The project's own browser interactions, run by check.py in every engine.

check.py has already loaded the page, waited for it to report ready, and let
it idle. These drive the setup screen the way a person would. They never open
the tab picker: a picker needs a person's click on the browser's own window,
so recording is checked by tests/browser/capture.py in Chromium, where a
command-line switch can choose the tab.

Expected values are worked out by hand from the brief (C4), never copied from
the tool's own output. Waits are on selectors and attributes, never on time.
"""

import json
import pathlib

STRINGS = json.loads((pathlib.Path(__file__).resolve().parents[2] / "data" / "strings.json").read_text())


def run(page, check, engine):
    check(page.inner_text("#title") == "YouTube to MP3", "the title is drawn")
    check(page.get_attribute("body", "data-screen") == "setup", "the setup screen is shown")
    check(page.inner_text("#message") == "", "no error message is shown")
    steps = page.locator("#steps li")
    check(steps.count() == 4, "four steps are listed")
    check(steps.nth(1).inner_text().startswith("Press Choose the tab,"), "a step names the button it means")

    choose = page.get_by_role("button", name="Choose the tab")
    check(choose.is_enabled(), "the tab picker can be opened")
    if engine.startswith("chromium"):
        check(STRINGS["browser_maybe"] not in page.inner_text("#start"), "Chromium is not warned about sound")
    else:
        check(STRINGS["browser_maybe"] in page.inner_text("#start"), f"{engine} is warned it may share no sound")

    check(page.get_attribute("[aria-pressed='true'][type=button]:has-text('192 kbps')", "aria-pressed") == "true",
          "192 kbps is chosen at first")
    page.get_by_role("button", name="256 kbps").click()
    page.wait_for_selector("button[aria-pressed='true']:has-text('256 kbps')")
    check(page.get_attribute("button:has-text('192 kbps')", "aria-pressed") == "false", "choosing 256 kbps unchooses 192")

    silence = page.get_by_role("button", name="Stop after 3 seconds of silence")
    check(silence.get_attribute("aria-pressed") == "true", "stopping on silence is on at first")
    silence.click()
    page.wait_for_selector("button[aria-pressed='false']:has-text('seconds of silence')")
    check(True, "stopping on silence can be turned off")

    box = page.locator("#field-link\\.set")
    box.click()
    box.press_sequentially("https://youtu.be/aqz-KE-bpKQ?si=x")
    page.wait_for_selector("#video a")
    link = page.locator("#video a")
    check(link.get_attribute("href") == "https://www.youtube.com/watch?v=aqz-KE-bpKQ", "the link opens the watch page")
    check(link.get_attribute("target") == "_blank", "the video opens in a new tab")
    check(box.input_value() == "https://youtu.be/aqz-KE-bpKQ?si=x", "typing kept every character in the box")
    check(page.evaluate("document.activeElement.id") == "field-link.set", "the box kept the focus while the page redrew")

    box.fill("https://vimeo.com/1")
    page.wait_for_selector(f"#video p:has-text('{STRINGS['link_not_youtube'][:20]}')")
    check(page.locator("#video a").count() == 0, "another site's address is not offered")
