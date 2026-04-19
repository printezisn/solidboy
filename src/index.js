import 'invokers-polyfill';
import { initGameEngine } from './game-engine';
import './styles/main.scss';
import uploadIcon from 'feather-icons/dist/icons/upload.svg?raw';
import terminalIcon from 'feather-icons/dist/icons/terminal.svg?raw';
import volumeIcon from 'feather-icons/dist/icons/volume.svg?raw';
import volumeOffIcon from 'feather-icons/dist/icons/volume-x.svg?raw';
import xIcon from 'feather-icons/dist/icons/x.svg?raw';
import tableIcon from 'feather-icons/dist/icons/table.svg?raw';
import { isMuted, setMuted } from './audio';

const updateVolumeButton = (volumeButton) => {
  volumeButton.innerHTML = isMuted()
    ? `<span class="volume-icon" aria-hidden="true">${volumeOffIcon}</span> Unmute`
    : `<span class="volume-icon" aria-hidden="true">${volumeIcon}</span> Mute`;
};

export const initEmulator = () => {
  const volumeButton = document.getElementById('volume-button');
  updateVolumeButton(volumeButton);

  document.getElementById('current-year').innerHTML = new Date().getFullYear();

  Array.from(document.getElementsByClassName('upload-icon')).forEach(
    (container) => {
      container.innerHTML = uploadIcon;
    },
  );

  Array.from(document.getElementsByClassName('terminal-icon')).forEach(
    (container) => {
      container.innerHTML = terminalIcon;
    },
  );

  Array.from(document.getElementsByClassName('x-icon')).forEach((container) => {
    container.innerHTML = xIcon;
  });

  Array.from(document.getElementsByClassName('table-icon')).forEach(
    (container) => {
      container.innerHTML = tableIcon;
    },
  );

  volumeButton.addEventListener('click', () => {
    setMuted(!isMuted());
    updateVolumeButton(volumeButton);
  });

  initGameEngine();
};
