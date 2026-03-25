const TRANSITION_TIME = 500;
const modal = document.getElementById('console-modal');

let isOpen = false;
let transitioning = false;

const openModal = () => {
  if (isOpen || transitioning) return;

  isOpen = true;
  transitioning = true;
  modal.classList.add('show-start');
  setTimeout(() => {
    modal.classList.add('show');
    modal.classList.remove('show-start');

    setTimeout(() => {
      transitioning = false;
    }, TRANSITION_TIME);
  }, 0);
};

const closeModal = () => {
  if (!isOpen || transitioning) return;

  isOpen = false;
  transitioning = true;
  modal.classList.add('show-end');
  modal.classList.remove('show');
  setTimeout(() => {
    modal.classList.remove('show-end');
    transitioning = false;
  }, TRANSITION_TIME);
};

export const initConsole = () => {
  document
    .getElementById('console-modal-overlay')
    .addEventListener('click', () => {
      closeModal();
    });

  document.getElementById('console-button').addEventListener('click', () => {
    openModal();
  });

  document
    .getElementById('console-close-button')
    .addEventListener('click', () => {
      closeModal();
    });

  document.addEventListener('keyup', (e) => {
    if (e.key === 'Escape') {
      closeModal();
    }
  });
};
