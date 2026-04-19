class GameBoyAudioProcessor extends AudioWorkletProcessor {
  constructor() {
    super();

    this.buffer = new Float32Array(0);
    this.readPos = 0;
    this.muted = false;

    this.port.onmessage = (e) => {
      if (e.data.sample != null) {
        const combined = new Float32Array(
          this.buffer.length - this.readPos + e.data.sample.length,
        );
        combined.set(this.buffer.subarray(this.readPos));
        combined.set(e.data.sample, this.buffer.length - this.readPos);
        this.buffer = combined;
        this.readPos = 0;
      } else if (e.data.muted != null) {
        this.muted = e.data.muted;
      }
    };
  }

  process(inputs, outputs) {
    const left = outputs[0][0];
    const right = outputs[0][1];
    const len = left.length;

    for (let i = 0; i < len; i++) {
      if (this.readPos + 1 < this.buffer.length) {
        left[i] = this.muted ? 0 : this.buffer[this.readPos];
        right[i] = this.muted ? 0 : this.buffer[this.readPos + 1];
        this.readPos += 2;
      } else {
        left[i] = 0;
        right[i] = 0;
      }
    }

    return true;
  }
}

registerProcessor('game-audio', GameBoyAudioProcessor);
