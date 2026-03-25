import { initGameEngine } from './game-engine';
import './styles/main.scss';
import uploadIcon from 'feather-icons/dist/icons/upload.svg?raw';
import terminalIcon from 'feather-icons/dist/icons/terminal.svg?raw';
import volumeIcon from 'feather-icons/dist/icons/volume.svg?raw';
import xIcon from 'feather-icons/dist/icons/x.svg?raw';
import { initConsole } from './console';

export const initEmulator = () => {
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

  Array.from(document.getElementsByClassName('volume-icon')).forEach(
    (container) => {
      container.innerHTML = volumeIcon;
    },
  );

  Array.from(document.getElementsByClassName('x-icon')).forEach((container) => {
    container.innerHTML = xIcon;
  });

  initConsole();
  initGameEngine();
};
