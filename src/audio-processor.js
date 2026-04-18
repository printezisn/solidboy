class GameBoyAudioProcessor extends AudioWorkletProcessor {
  constructor() {
    super();

    this.buffer = new Float32Array(0);
    this.readPos = 0;

    this.port.onmessage = (e) => {
      const newSamples = e.data;
      const combined = new Float32Array(
        this.buffer.length - this.readPos + newSamples.length,
      );
      combined.set(this.buffer.subarray(this.readPos));
      combined.set(newSamples, this.buffer.length - this.readPos);
      this.buffer = combined;
      this.readPos = 0;
    };
  }

  process(inputs, outputs) {
    const left = outputs[0][0];
    const right = outputs[0][1];
    const len = left.length;

    for (let i = 0; i < len; i++) {
      if (this.readPos + 1 < this.buffer.length) {
        left[i] = this.buffer[this.readPos++];
        right[i] = this.buffer[this.readPos++];
      } else {
        left[i] = 0;
        right[i] = 0;
      }
    }

    return true;
  }
}

registerProcessor('game-audio', GameBoyAudioProcessor);
