# Glossary

The source of truth for what each word in this tool means. Strings, tests, the plan and the README use a term in the sense given here. The page shows a definition when a reader hovers the term.

Write each entry as a `### Term` heading, then one or two plain sentences, then an optional `Not:` line listing words that must not be used for it. A number a rule decides is a `{placeholder}` the core fills, so the glossary cannot disagree with the rules. Keep entries in alphabetical order. `crates/core/tests/copy.rs` checks the format, the placeholders and the `Not:` words; `TONE.md` governs the prose.

<!--
An example entry. It sits inside this comment, so it does not count.

### Spout

Where slush comes out. Each line has up to {line.spouts} spouts.

Not: tap, dispenser, nozzle
-->

### Bitrate

How many kilobits of the saved file hold each second of sound. The page offers {bitrates} kbps and starts at {default}. A higher bitrate keeps more detail and makes a larger file.

Not: quality setting, kbps rate

### MP3

The sound file the page makes, in the compressed audio format that music players on phones and computers open.

### Record silently

A setting that has the browser record the tab's sound without playing it through your speakers or headphones. It takes effect when sharing starts.

Not: mute

### Silence

Sound whose loudest sample stays about sixty decibels below full volume. Silence before the first sound and after the last is left out of the saved file; with stopping on silence turned on, {seconds} seconds of it ends the recording.

Not: quiet part, gap

### Tab

A browser tab. The page records the sound of the one tab the reader shares with it, and hears nothing from other tabs or programs.

Not: window, screen

### Tags

The title and artist stored inside the saved file, which music players show in place of the file name.

Not: metadata, labels
