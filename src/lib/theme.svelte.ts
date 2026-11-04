const storageKey = 'quicktex-theme'
const root = document.documentElement
const system = matchMedia('(prefers-color-scheme: dark)')

function chosen() {
  try {
    return localStorage.getItem(storageKey)
  } catch {
    return null
  }
}

function paint(dark: boolean) {
  root.classList.toggle('dark', dark)
  const background = getComputedStyle(root).getPropertyValue('--color-paper').trim()
  document.querySelector('meta[name="theme-color"]')?.setAttribute('content', background)
}

class Theme {
  dark = $state(root.classList.contains('dark'))

  constructor() {
    paint(this.dark)
    system.addEventListener('change', (event) => {
      if (chosen() === null) this.set(event.matches)
    })
  }

  set(dark: boolean) {
    this.dark = dark
    paint(dark)
  }

  toggle() {
    this.set(!this.dark)
    try {
      localStorage.setItem(storageKey, this.dark ? 'dark' : 'light')
    } catch {
      return
    }
  }
}

export const theme = new Theme()
