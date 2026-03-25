Object.defineProperty(exports,Symbol.toStringTag,{value:`Module`});var e=`<header class="container">
  <h1 class="title1">Solidboy Emulator</h1>
  <p>A GameBoy/GameBoy Color emulator</p>
</header>
<main class="container">
  <div class="device">
    <div class="screen-container">
      <div class="insert-rom-container">
        <span class="upload-icon"></span>
        <label id="insert-rom-label">Insert your ROM</label>
        <input
          type="file"
          class="rom-file"
          id="rom-file"
          arial-labelledby="insert-rom-label"
        />
      </div>
    </div>
    <div class="controls">
      <div class="direction-container">
        <button
          type="button"
          aria-label="Up"
          class="up-direction"
          id="up-direction"
        ></button>
        <button
          type="button"
          aria-label="Right"
          class="right-direction"
          id="right-direction"
        ></button>
        <button
          type="button"
          aria-label="Down"
          class="down-direction"
          id="down-direction"
        ></button>
        <button
          type="button"
          aria-label="Left"
          class="left-direction"
          id="left-direction"
        ></button>
      </div>
      <div class="menu-container">
        <div class="select-container">
          <button
            type="button"
            aria-label="select"
            class="select-button"
            id="select-button"
          ></button>
          <span>Select</span>
        </div>
        <div class="start-container">
          <button
            type="button"
            aria-label="select"
            class="start-button"
            id="start-button"
          ></button>
          <span>Start</span>
        </div>
      </div>
      <div class="buttons-container">
        <div class="b-button-container">
          <button
            type="button"
            aria-label="B"
            class="b-button"
            id="b-button"
          ></button>
          <span>B</span>
        </div>
        <div class="a-button-container">
          <button
            type="button"
            aria-label="A"
            class="a-button"
            id="a-button"
          ></button>
          <span>A</span>
        </div>
      </div>
    </div>
  </div>
</main>
<footer class="container">
  <p>
    ©
    <span class="current-year" id="current-year"></span>
    Nikos Printezis. All Rights Reserved.
  </p>
  <nav>
    <ul>
      <li>
        <a href="/">Find more content</a>
      </li>
      <li>
        <a href="/privacy-policy">Privacy Policy</a>
      </li>
    </ul>
  </nav>
</footer>
`;exports.MainBodyHtml=e;