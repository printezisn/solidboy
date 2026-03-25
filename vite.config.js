import { defineConfig } from 'vite';
import fs from 'fs';
import path from 'path';
import wasm from 'vite-plugin-wasm';
import { createHtmlPlugin } from 'vite-plugin-html';

const mainBody = fs
  .readFileSync(path.join(import.meta.dirname, 'templates', 'main-body.html'))
  .toString();

export default defineConfig({
  build: {
    lib: {
      entry: ['src/index.js', 'src/templates.js'],
      name: 'solidboy-emulator',
    },
    copyPublicDir: true,
  },
  plugins: [
    wasm(),
    createHtmlPlugin({
      inject: {
        data: {
          mainBody,
        },
      },
    }),
  ],
});
