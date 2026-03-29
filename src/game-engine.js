import { init_emulator, execute, emulator_memory } from 'emulator-lib';

const CYCLES_PER_MILLI = 4194;
const MAX_FRAME_DIFF = 20;

let canvas = null;
let framebuffer = new Uint8ClampedArray(160 * 144 * 4);

let lastFrameTime = null;
let consoleMessage = '';

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

  const diff = Math.max(MAX_FRAME_DIFF, now - lastFrameTime);
  let totalCycles = diff * CYCLES_PER_MILLI;
  execute(totalCycles);

  lastFrameTime = now;

  renderFrameBuffer();

  requestAnimationFrame(onFrame);
};

export const initGameEngine = () => {
  const memory = emulator_memory();

  window.emulator_console_log = (str) => {
    consoleMessage += str || '';
    debugConsole.innerHTML = consoleMessage;
  };

  window.emulator_console_error = (str) => {
    consoleMessage = '<span class="error-message">' + (str || '') + '</span>';
    debugConsole.innerHTML = consoleMessage;
  };

  window.render_frame_buffer = (frame_buffer_ptr, length) => {
    framebuffer = new Uint8ClampedArray(
      memory.buffer,
      frame_buffer_ptr,
      length,
    );
  };

  document.getElementById('rom-file').addEventListener('change', (e) => {
    const file = e.target.files[0];
    if (!file) return;

    const reader = new FileReader();
    reader.onload = () => {
      init_emulator(new Uint8Array(reader.result));

      canvas = document.createElement('canvas');
      canvas.width = 160;
      canvas.height = 144;
      document.getElementById('insert-rom-container').remove();
      document.getElementById('screen-container').appendChild(canvas);

      onFrame();
    };

    reader.readAsArrayBuffer(file);
  });
};
