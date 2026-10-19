// Plays interleaved stereo float chunks posted from the game in arrival order
class CrunchOutput extends AudioWorkletProcessor {
  constructor() {
    super();
    this.chunks = [];
    this.offset = 0;
    this.port.onmessage = (e) => { this.chunks.push(e.data); };
  }

  process(_inputs, outputs) {
    const [left, right] = outputs[0];
    for (let i = 0; i < left.length; i++) {
      while (this.chunks.length && this.offset >= this.chunks[0].length) {
        this.chunks.shift();
        this.offset = 0;
      }
      if (!this.chunks.length) {
        left[i] = 0;
        right[i] = 0;
        continue;
      }
      const chunk = this.chunks[0];
      left[i] = chunk[this.offset];
      right[i] = chunk[this.offset + 1];
      this.offset += 2;
    }
    return true;
  }
}

registerProcessor('crunch-output', CrunchOutput);
