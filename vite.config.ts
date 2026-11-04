import { fileURLToPath } from 'node:url'
import { svelte } from '@sveltejs/vite-plugin-svelte'
import { defineConfig } from 'vite'

const page = (path: string) => fileURLToPath(new URL(path, import.meta.url))

export default defineConfig({
  base: './',
  plugins: [svelte()],
  build: {
    rolldownOptions: {
      input: {
        main: page('index.html'),
        editor: page('editor/index.html'),
      },
    },
  },
})
