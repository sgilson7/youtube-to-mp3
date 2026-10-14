// The page draws what the core returns and decides nothing.
//
// The core describes the screen (its panels, fields, actions and the
// glossary's definitions), and this file builds the DOM from that
// description without knowing what any panel means. Every button and field
// sends its action name and value back through `act`, the core's one door.
// After drawing, the page carries out the effects the core names: open the
// tab picker, close it, save the file. Sound from the shared tab goes to the
// core through `feed`, block by block, as bytes. (After Peter's Mandible.)
//
// For a video inside the page, the core names a player frame and the
// messages to post to it; every message the window receives goes back to the
// core as text with its origin, and the core decides what it means.
import init, { Tool } from "./pkg/tool_shim.js?v=1bc9a96e2c0f";

const el = (id) => document.getElementById(id);
const make = (tag, text) => {
  const node = document.createElement(tag);
  if (text !== undefined) node.textContent = text;
  return node;
};

let tool = null;
let ui = null;
let options = null;
let capture = null;
const drawn = {};

function send(name, value) {
  draw(JSON.parse(tool.act(name, value)));
}

// A label whose text is a glossary term shows the definition on hover.
function mark(node) {
  const hint = ui.hints.find((h) => h.term === node.textContent);
  if (hint) {
    node.dataset.term = hint.term;
    node.title = hint.definition;
  }
}

// A text box keeps its node across redraws, so typing keeps its place.
function field(f, kept) {
  const id = "field-" + f.name;
  const label = make("label", f.label);
  label.htmlFor = id;
  let input = kept[id];
  if (!input) {
    input = make("input");
    input.id = id;
    input.type = f.kind;
    input.autocomplete = "off";
    input.spellcheck = false;
    input.addEventListener("input", () => send(f.name, input.value));
  }
  if (input.value !== f.value) input.value = f.value;
  const row = make("div");
  row.className = "field";
  row.append(label, input);
  return row;
}

function meter(m) {
  const label = make("label", m.label);
  label.htmlFor = m.id;
  const bar = make("meter");
  bar.id = m.id;
  bar.min = 0;
  bar.max = m.max;
  const reading = make("span");
  reading.id = m.id + "-reading";
  const row = make("div");
  row.className = "meter";
  row.append(label, bar, reading);
  return row;
}

function panel(p, kept) {
  const box = make("section");
  box.id = p.id;
  if (p.heading) box.append(make("h2", p.heading));
  (p.lines || []).forEach((l) => {
    const node = make("p", l.text);
    if (l.id) node.id = l.id;
    box.append(node);
  });
  if (p.steps) {
    const list = make("ol");
    p.steps.forEach((s) => list.append(make("li", s)));
    box.append(list);
  }
  (p.fields || []).forEach((f) => box.append(field(f, kept)));
  if (p.link) {
    const a = make("a", p.link.label);
    a.href = p.link.href;
    a.target = "_blank";
    a.rel = "noopener noreferrer";
    box.append(a);
  }
  (p.meters || []).forEach((m) => box.append(meter(m)));
  if (p.actions) {
    const row = make("div");
    row.className = "actions";
    p.actions.forEach((a) => {
      const button = make("button", a.label);
      button.type = "button";
      button.disabled = !a.enabled;
      if (a.pressed !== undefined) button.setAttribute("aria-pressed", String(a.pressed));
      if (a.primary) button.className = "primary";
      button.addEventListener("click", () => send(a.name, a.value));
      row.append(button);
    });
    box.append(row);
  }
  return box;
}

// Only a panel whose description changed is rebuilt.
function drawPanels(panels) {
  // Moving a focused box blurs it, so the focus and caret are noted first.
  const active = document.activeElement;
  const focused = active && active.id;
  const caret = active && active.tagName === "INPUT" ? [active.selectionStart, active.selectionEnd] : null;
  const kept = {};
  document.querySelectorAll("#panels input").forEach((input) => {
    kept[input.id] = input;
  });
  const nodes = panels.map((p) => {
    const text = JSON.stringify(p);
    const old = el(p.id);
    if (old && drawn[p.id] === text) return old;
    drawn[p.id] = text;
    return panel(p, kept);
  });
  el("panels").replaceChildren(...nodes);
  const again = focused && el(focused);
  if (again && again !== document.activeElement) {
    again.focus();
    if (caret) again.setSelectionRange(...caret);
  }
  document.querySelectorAll("#panels h2, #panels button").forEach(mark);
}

// The frame is drawn only when the core's description of it changes, because
// moving or rebuilding a frame reloads the video.
function drawPlayer(p) {
  const text = JSON.stringify(p);
  if (drawn.player === text) return;
  drawn.player = text;
  const box = el("player");
  box.replaceChildren();
  if (!p) return;
  const frame = make("iframe");
  frame.src = p.src;
  frame.title = p.title;
  frame.allow = "autoplay; encrypted-media";
  frame.addEventListener("load", () => send("player.loaded", ""));
  box.append(frame);
}

function post(message) {
  el("player").firstChild.contentWindow.postMessage(message, ui.player.origin);
}

function draw(result) {
  if (result.error) {
    el("message").textContent = result.error;
    return;
  }
  ui = result.ok;
  document.title = ui.title;
  el("title").textContent = ui.title;
  drawPlayer(ui.player);
  drawPanels(ui.panels);
  el("message").textContent = ui.message;
  document.body.dataset.screen = ui.screen;
  ui.effects.forEach((name) => effects[name]());
  ui.posts.forEach(post);
  const follow = ui.follow;
  if (follow) setTimeout(() => send(follow, ""), options.paint_ms);
}

async function open() {
  // Made inside the click, so the browser lets it play.
  const context = new AudioContext(options.context);
  const current = { context, stream: null, node: null };
  capture = current;
  const display = {
    ...options.display[ui.capture.surface],
    audio: { ...options.audio, suppressLocalAudioPlayback: ui.capture.silent },
  };
  try {
    const stream = await navigator.mediaDevices.getDisplayMedia(display);
    current.stream = stream;
    const tracks = stream.getAudioTracks();
    if (tracks.length) {
      await context.audioWorklet.addModule("capture-worklet.js?v=1bc9a96e2c0f");
      const node = new AudioWorkletNode(context, "capture", options.worklet);
      node.port.onmessage = (e) => {
        const next = tool.feed(new Uint8Array(e.data.left), new Uint8Array(e.data.right));
        if (next) {
          const action = JSON.parse(next);
          send(action.name, action.value);
        }
      };
      context.createMediaStreamSource(stream).connect(node);
      node.connect(context.destination);
      current.node = node;
    }
    stream.getTracks().forEach((t) => t.addEventListener("ended", () => send("capture.ended", "")));
    await context.resume();
    send("capture.started", JSON.stringify({ tracks: tracks.length, rate: context.sampleRate }));
  } catch (error) {
    send("capture.failed", error.name);
  }
}

function close() {
  if (!capture) return;
  if (capture.node) capture.node.port.onmessage = null;
  if (capture.stream) capture.stream.getTracks().forEach((t) => t.stop());
  capture.context.close();
  capture = null;
}

function save() {
  const url = URL.createObjectURL(new Blob([tool.file()], { type: "audio/mpeg" }));
  const a = make("a");
  a.href = url;
  a.download = ui.file_name;
  document.body.append(a);
  a.click();
  a.remove();
  setTimeout(() => URL.revokeObjectURL(url), options.revoke_ms);
}

const effects = { "capture.open": open, "capture.close": close, "file.save": save };

// The time and the level change many times a second, so they are read on
// each frame and written into the nodes that show them.
function tick() {
  if (ui && ui.live) {
    JSON.parse(tool.live()).forEach((r) => {
      const node = el(r.id);
      if (!node) return;
      if (r.value === undefined) {
        node.textContent = r.text;
      } else {
        node.value = r.value;
        el(r.id + "-reading").textContent = r.text;
      }
    });
  }
  requestAnimationFrame(tick);
}

async function main() {
  await init();
  const strings = await (await fetch("data/strings.json?v=1bc9a96e2c0f")).text();
  const glossary = await (await fetch("data/glossary.md?v=1bc9a96e2c0f")).text();
  options = await (await fetch("data/capture.json?v=1bc9a96e2c0f")).json();
  const abilities = {
    display: Boolean(navigator.mediaDevices && navigator.mediaDevices.getDisplayMedia),
    worklet: "AudioWorkletNode" in window,
    chromium: "userAgentData" in navigator,
    origin: location.origin,
  };
  tool = new Tool(strings, glossary, JSON.stringify(abilities));
  draw(JSON.parse(tool.ui()));
  window.addEventListener("message", (e) => send("player.message", JSON.stringify({ origin: e.origin, data: String(e.data) })));
  requestAnimationFrame(tick);
  document.body.dataset.ready = "true";
}

main();
