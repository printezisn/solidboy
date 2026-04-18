import { init_emulator, execute, emulator_memory } from 'solidboy-emulator';
import { initJoypad, pressedButtons, pressedDirections } from './joypad';
import { fetchGameData, saveGameData } from './storage';
import { appendSample, initAudio, resumeAudio } from './audio';

const CYCLES_PER_MILLI = 4194;
const MAX_FRAME_DIFF = 20;

let canvas = null;
let framebuffer = new Uint8ClampedArray(160 * 144 * 4);
let lastFrameTime = null;
let gameName = null;
let dataToBeSaved = null;

const debugConsole = document.getElementById('console');

const renderFrameBuffer = () => {
  const imageData = new ImageData(framebuffer, 160, 144);
  const ctx = canvas.getContext('2d');
  ctx.putImageData(imageData, 0, 0);
};

const onFrame = () => {
  const now = performance.now();
  if (lastFrameTime == null) {
    lastFrameTime = now;
  }

  const diff = Math.min(MAX_FRAME_DIFF, now - lastFrameTime);
  let totalCycles = diff * CYCLES_PER_MILLI;
  execute(totalCycles, pressedDirections(), pressedButtons());

  lastFrameTime = now;

  resumeAudio();
  renderFrameBuffer();

  if (dataToBeSaved) {
    saveGameData(gameName, dataToBeSaved);
    dataToBeSaved = null;
  }

  requestAnimationFrame(onFrame);
};

export const initGameEngine = () => {
  const memory = emulator_memory();

  window.emulator_console_log = (str) => {
    debugConsole.innerHTML += str;
  };

  window.emulator_console_error = (str) => {
    debugConsole.innerHTML += `<span class="error-message">${str}</span>`;
  };

  window.render_frame_buffer = (frame_buffer_ptr, length) => {
    framebuffer = new Uint8ClampedArray(
      memory.buffer,
      frame_buffer_ptr,
      length,
    );
  };

  window.save_data = (data_ptr, length) => {
    dataToBeSaved = new Uint8ClampedArray(memory.buffer, data_ptr, length);
  };

  window.append_audio_sample = (data_ptr, length) => {
    const buffer = new Float32Array(data_ptr, length);
    appendSample(buffer);
  };

  document.getElementById('rom-file').addEventListener('change', (e) => {
    const file = e.target.files[0];
    if (!file) return;

    gameName = file.name;

    const reader = new FileReader();
    reader.onload = async () => {
      canvas = document.createElement('canvas');
      canvas.width = 160;
      canvas.height = 144;
      document.getElementById('insert-rom-container').remove();
      document.getElementById('screen-container').appendChild(canvas);

      const rom = new Uint8Array(reader.result);
      const gameData = await fetchGameData(gameName);

      await initAudio();
      init_emulator(rom, gameData);
      initJoypad();

      onFrame();
    };

    reader.readAsArrayBuffer(file);
  });
};
