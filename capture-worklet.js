// Runs on the browser's audio thread. Hands each block of the shared tab's
// sound to the page as the bytes of its samples, unchanged. Decides nothing.
class Capture extends AudioWorkletProcessor {
  process(inputs) {
    const input = inputs[0];
    if (input.length) {
      const left = input[0].slice();
      const right = input[input.length - 1].slice();
      this.port.postMessage({ left: left.buffer, right: right.buffer }, [left.buffer, right.buffer]);
    }
    return true;
  }
}

registerProcessor("capture", Capture);
