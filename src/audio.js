import audioProcessorUrl from './audio-processor?worker&url';

let audioContext;
let audioWorkletNode;

export const initAudio = async () => {
  audioContext = new AudioContext({ sampleRate: 44100 });

  await audioContext.audioWorklet.addModule(audioProcessorUrl);

  audioWorkletNode = new AudioWorkletNode(audioContext, 'game-audio', {
    outputChannelCount: [2],
  });

  audioWorkletNode.connect(audioContext.destination);
};

export const resumeAudio = () => {
  if (audioContext && audioContext.state === 'suspended') {
    audioContext.resume();
  }
};

export const appendSample = (sample) => {
  audioWorkletNode.port.postMessage(sample);
};
