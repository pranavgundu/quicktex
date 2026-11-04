import { mount } from 'svelte'
import '../app.css'
import App from './App.svelte'
import { init } from '../../wasm-core/js/quicktex.js'

const target = document.getElementById('app')!

init()
  .then(() => mount(App, { target }))
  .catch(() => {
    target.textContent = 'The renderer did not load. Reload the page to try again.'
  })
