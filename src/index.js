import './styles/main.scss';
import uploadIcon from 'feather-icons/dist/icons/upload.svg?raw';

export const initEmulator = () => {
  document.getElementById('current-year').innerHTML = new Date().getFullYear();
  Array.from(document.getElementsByClassName('upload-icon')).forEach(
    (container) => {
      container.innerHTML = uploadIcon;
    },
  );
};
