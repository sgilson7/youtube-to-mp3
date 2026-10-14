# Decisions

Dated entries. Each quotes the person's request word for word, then says what the agent decided and where it is recorded. Grants of autonomy go here too ("you may publish"), with their scope: this run only, or until withdrawn.

## 2026-10-13

> Use sgilson7.github.io builder setting, which can be found one directory up. Use the shared technical setting within to create the following tool for me: a youtube to mp3 converter that works like the pdf redactor, with no terrible ads or anything like using other open source youtube to mp3 converters

The agent decided: there is no `sgilson7.github.io` directory one level up; the Builder reference repository is at `~/Documents/Builder/builder-setting`, and this project is a copy of its `template/` (Builder 0.2.0, commit `6621ad2`) in `~/Documents/YoutubeToMp3Converter`.

> browser page, link in, mp3 out, i dont care about youtube changes and their terms, i just actively want a version of an open source youtube to mp3 converter and release it online so other people dont have to use the terrible ad filled virus filled versions that steal all the data online

The agent decided: a static page cannot fetch from YouTube (`SECOND-ORDER.md` R1.1), so it offered a local app with a link box, a hosted server, or a Firefox extension. The person chose the local app.

> wait so why cant the browser read youtubes stream directly? lets get a little creative and try and figure that out. can the host create a faux server all in the browser cache and use that to pull the youtube stream?

> lets try the creative route fist

The agent decided: build the tab recorder first, as a static page with no off-origin request (`BRIEF.md`). The local app with a link box is not built; it stays open as a second tool the person may ask for. Publishing is not granted: the person wants it released but has not named where (`SECOND-ORDER.md` R1.9).
