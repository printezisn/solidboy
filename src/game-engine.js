import { init_emulator, execute } from 'emulator-lib';

const CYCLES_PER_MILLI = 4194;
const MAX_FRAME_DIFF = 20;

let lastFrameTime = null;
let consoleMessage = '';

const debugConsole = document.getElementById('console');

const onFrame = () => {
  const now = performance.now();
  if (lastFrameTime == null) {
    lastFrameTime = now;
  }

  const diff = Math.max(MAX_FRAME_DIFF, now - lastFrameTime);
  let totalCycles = diff * CYCLES_PER_MILLI;
  while (totalCycles > 0) {
    totalCycles -= execute();
  }

  lastFrameTime = now;

  requestAnimationFrame(onFrame);
};

export const initGameEngine = () => {
  window.append_emulator_message = (str) => {
    consoleMessage += str || '';
    debugConsole.innerHTML = consoleMessage;
  };

  window.set_emulator_message = (str) => {
    consoleMessage = str || '';
    debugConsole.innerHTML = consoleMessage;
  };

  window.set_emulator_error = (str) => {
    consoleMessage = '<span class="error-message">' + (str || '') + '</span>';
    debugConsole.innerHTML = consoleMessage;
  };

  document.getElementById('rom-file').addEventListener('change', (e) => {
    const file = e.target.files[0];
    if (!file) return;

    const reader = new FileReader();
    reader.onload = () => {
      init_emulator(new Uint8Array(reader.result));
      document.getElementById('insert-rom-container').remove();
      onFrame();
    };

    reader.readAsArrayBuffer(file);
  });
};
