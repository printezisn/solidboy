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
  audioWorkletNode.port.postMessage({ sample });
};

export const isMuted = () => localStorage.getItem('muted')?.trim() === 'true';

export const setMuted = (value) => {
  localStorage.setItem('muted', value ? 'true' : 'false');
  if (audioWorkletNode) {
    audioWorkletNode.port.postMessage({ muted: isMuted() });
  }
};
