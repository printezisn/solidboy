const buttonState = {
  'down-direction': { pressed: false, bit: 0x08, key: 'ArrowDown' },
  'up-direction': { pressed: false, bit: 0x04, key: 'ArrowUp' },
  'left-direction': { pressed: false, bit: 0x02, key: 'ArrowLeft' },
  'right-direction': { pressed: false, bit: 0x01, key: 'ArrowRight' },

  'start-button': { pressed: false, bit: 0x08, key: 'Enter' },
  'select-button': { pressed: false, bit: 0x04, key: ' ' },
  'b-button': { pressed: false, bit: 0x02, key: 'a' },
  'a-button': { pressed: false, bit: 0x01, key: 's' },
};

export const pressedDirections = () => {
  let result = 0;

  for (const name in buttonState) {
    if (name.includes('-direction') && buttonState[name].pressed) {
      result |= buttonState[name].bit;
    }
  }

  return ~result & 0x0f;
};

export const pressedButtons = () => {
  let result = 0;

  for (const name in buttonState) {
    if (name.includes('-button') && buttonState[name].pressed) {
      result |= buttonState[name].bit;
    }
  }

  return ~result & 0x0f;
};

export const initJoypad = () => {
  for (const name in buttonState) {
    const button = document.getElementById(name);

    button.addEventListener('pointerdown', (e) => {
      e.preventDefault();
      buttonState[name].pressed = true;
      button.classList.add('pressed');
    });

    button.addEventListener('pointerup', (e) => {
      e.preventDefault();
      buttonState[name].pressed = false;
      button.classList.remove('pressed');
    });
  }

  document.addEventListener('keydown', (e) => {
    for (const name in buttonState) {
      if (e.key === buttonState[name].key) {
        buttonState[name].pressed = true;
        const button = document.getElementById(name);
        button.classList.add('pressed');

        e.preventDefault();
        break;
      }
    }
  });

  document.addEventListener('keyup', (e) => {
    for (const name in buttonState) {
      if (e.key === buttonState[name].key) {
        buttonState[name].pressed = false;
        const button = document.getElementById(name);
        button.classList.remove('pressed');

        e.preventDefault();
        break;
      }
    }
  });
};
