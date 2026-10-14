# Triage

Stage 7. One row per request after the person has used the build: a bug, a change, or a new idea, in the person's own words. Rows are numbered T1, T2 and so on, and each commit that answers one names it ("Fix the timer reset (T4)"). `C4 Verify against hand-computed cases`.

Compare at least one result with a case worked out by hand. **Severity** is *blocks use*, *wrong*, or *wanted*. **Cost** is the agent's estimate. **Disposition** is *done*, *open*, *declined* (with the reason), or *try* (built on a branch and published as a TRY commit that says how to undo it).

| # | Request, in the person's words | Expected | What happened | Severity | Cost | Disposition |
| --- | --- | --- | --- | --- | --- | --- |
| T1 | i cant choose the next tab where the youtube sound exists after pressing choose the tab, the share window screen to record the screen on mac  doesnt allow you to click the other tabs in the window | A list of tabs to pick from | The only browser on the Mac is Safari 26.3, whose picker shares windows and screens with no sound; the page let Safari open it and warned only in small grey text | blocks use | small | done: Safari and Firefox get a notice after the intro and the button is disabled; the Chrome steps say to pick from the list, not the tab bar (R2.1) |
