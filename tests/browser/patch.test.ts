import { afterAll, beforeAll, describe, expect, test } from 'bun:test'
import { copyFileSync, mkdtempSync, rmSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import type { Browser, Page } from 'playwright-core'
import { launch, serve } from '../support/browser'
import { fixture } from '../support/engine'

const folder = mkdtempSync(join(tmpdir(), 'quicktex-patch-'))
let site: ReturnType<typeof serve>
let browser: Browser
let page: Page

beforeAll(async () => {
  const built = await Bun.build({ entrypoints: [new URL('../../src/editor/patch.ts', import.meta.url).pathname], outdir: folder, target: 'browser' })
  expect(built.success).toBe(true)
  for (const name of ['quicktex.js', 'quicktex.wasm']) copyFileSync(new URL(`../../wasm-core/js/${name}`, import.meta.url).pathname, join(folder, name))
  await Bun.write(join(folder, 'index.html'), '<!doctype html><div id="a"></div><div id="b" hidden></div>')
  site = serve(folder)
  browser = await launch()
  page = await browser.newPage()
  await page.goto(site.url)
  await page.evaluate(async () => {
    const engine = await import(new URL('quicktex.js', location.href).href)
    const { showPage } = await import(new URL('patch.js', location.href).href)
    await engine.init(await (await fetch('quicktex.wasm')).arrayBuffer())
    Object.assign(window, { engine, showPage })
  })
})

afterAll(async () => {
  await browser.close()
  site.stop()
  rmSync(folder, { recursive: true })
})

const compare = (body: string, args: unknown) =>
  page.evaluate(({ body, args }) => new (Object.getPrototypeOf(async function () {}).constructor)('args', body)(args), { body, args })

const walk = `
  const { engine, showPage } = window
  const { steps, tex } = args
  const a = document.getElementById('a'), b = document.getElementById('b')
  engine.renderDocument(tex)
  a.replaceChildren()
  const shown = showPage(a, engine.documentPage(0))
  let failed = 0
  for (const next of steps) {
    let text = next
    try { engine.renderDocument(text) } catch { text = tex; engine.renderDocument(text) }
    const svg = engine.documentPage(0)
    shown.update(svg)
    b.innerHTML = svg.split('\\n').join('')
    if (a.innerHTML !== b.innerHTML) failed++
  }
  return { failed, children: a.firstElementChild?.children.length ?? 0 }
`

function run(tex: string, steps: string[]) {
  return compare(walk, { tex, steps }) as Promise<{ failed: number; children: number }>
}

function edits(base: string, count: number, seed: number) {
  let state = seed
  const next = (limit: number) => {
    state = (Math.imul(state, 1664525) + 1013904223) >>> 0
    return state % limit
  }
  const start = base.indexOf('\\begin{document}') + 20
  const steps: string[] = []
  let text = base
  for (let i = 0; i < count; i++) {
    const at = start + next(text.length - start - 20)
    const kind = next(6)
    if (kind === 0) text = text.slice(0, at) + 'word ' + text.slice(at)
    else if (kind === 1) text = text.slice(0, at) + text.slice(at + 1 + next(30))
    else if (kind === 2) text = text.slice(0, at) + '\n\n' + text.slice(at)
    else if (kind === 3) text = base
    else if (kind === 4) text = text.slice(0, at) + 'x' + text.slice(at)
    else text = text.slice(0, at) + ' lots of new text to push the following lines along here ' + text.slice(at)
    steps.push(text)
  }
  return steps
}

describe('page patching', () => {
  for (const name of ['lab', 'shapes', 'sample']) {
    test(`${name} matches a fresh render after 1500 random edits`, async () => {
      const base = fixture(name)
      const result = await run(base, edits(base, 1500, name.length * 31))
      expect(result.failed).toBe(0)
      expect(result.children).toBeGreaterThan(0)
    })
  }

  const doc = (body: string) => `\\documentclass{article}\\usepackage{tikz}\\begin{document}${body}\\end{document}`

  const cases: [string, string, string][] = [
    ['a line removed', 'one\n\ntwo\n\nthree', 'one\n\nthree'],
    ['a line added', 'one\n\nthree', 'one\n\ntwo\n\nthree'],
    ['lines swapped', 'one\n\ntwo', 'two\n\none'],
    ['everything replaced', 'one', 'completely different'],
    ['everything removed', 'one\n\ntwo', ''],
    ['a drawing added', 'x', '\\begin{tikzpicture}\\draw (0,0) -- (1,0);\\end{tikzpicture}x'],
    ['a drawing removed', '\\begin{tikzpicture}\\draw (0,0) -- (1,0);\\end{tikzpicture}x', 'x'],
    ['two identical drawings', '\\begin{tikzpicture}\\draw (0,0) -- (1,0);\\draw (0,0) -- (1,0);\\end{tikzpicture}', '\\begin{tikzpicture}\\draw (0,0) -- (1,0);\\end{tikzpicture}'],
    ['duplicates appear', '\\begin{tikzpicture}\\draw (0,0) -- (1,0);\\end{tikzpicture}', '\\begin{tikzpicture}\\draw (0,0) -- (1,0);\\draw (0,0) -- (1,0);\\draw (0,0) -- (1,0);\\end{tikzpicture}'],
    ['paint order changes', '\\begin{tikzpicture}\\fill[red] (0,0) rectangle (1,1);\\fill[blue] (0,0) rectangle (1,1);\\end{tikzpicture}', '\\begin{tikzpicture}\\fill[blue] (0,0) rectangle (1,1);\\fill[red] (0,0) rectangle (1,1);\\end{tikzpicture}'],
    ['the last element goes', 'x$y_1$', 'x'],
    ['the last two elements go', 'x$y_1^{\\textcolor{red}{2}}$', 'x'],
    ['the last element changes', 'x$y_1$', 'x$y_2$'],
    ['a trailing color goes', 'x$\\textcolor{red}{y}$', 'x'],
    ['elements appear at the end', 'x', 'x$y_1$'],
    ['turned text changes', '\\begin{tikzpicture}\\node[rotate=30] at (0,0) {a};\\end{tikzpicture}', '\\begin{tikzpicture}\\node[rotate=60] at (0,0) {a};\\end{tikzpicture}'],
  ]
  for (const [name, before, after] of cases) {
    test(`stays exact when ${name}`, async () => {
      const result = await run(doc(before), [doc(after), doc(before), doc(after)])
      expect(result.failed).toBe(0)
    })
  }

  test('reuses untouched elements', async () => {
    const reused = await compare(
      `
      const { engine, showPage } = window
      const a = document.getElementById('a')
      engine.renderDocument(args.before)
      a.replaceChildren()
      const shown = showPage(a, engine.documentPage(0))
      const old = new Set(a.firstElementChild.children)
      engine.renderDocument(args.after)
      shown.update(engine.documentPage(0))
      const now = [...a.firstElementChild.children]
      return { kept: now.filter((e) => old.has(e)).length, total: now.length }
    `,
      { before: fixture('lab'), after: fixture('lab').replace('Does the relationship', 'Does a relationship') },
    )
    const { kept, total } = reused as { kept: number; total: number }
    expect(total).toBeGreaterThan(50)
    expect(kept).toBeGreaterThan(total - 8)
  })

  test('an unchanged page touches nothing', async () => {
    const same = await compare(
      `
      const { engine, showPage } = window
      const a = document.getElementById('a')
      engine.renderDocument(args.tex)
      a.replaceChildren()
      const svg = engine.documentPage(0)
      const shown = showPage(a, svg)
      let records = 0
      const watcher = new MutationObserver((list) => { records += list.length })
      watcher.observe(a, { subtree: true, childList: true, attributes: true })
      shown.update(svg)
      await new Promise((done) => setTimeout(done))
      watcher.disconnect()
      return records
    `,
      { tex: fixture('lab') },
    )
    expect(same).toBe(0)
  })

  test('a new page size redraws the whole page', async () => {
    const a4 = '\\documentclass[a4paper]{article}\\begin{document}x\\end{document}'
    const letter = '\\documentclass{article}\\begin{document}x\\end{document}'
    const result = await run(letter, [a4, letter])
    expect(result.failed).toBe(0)
  })

  test('an empty page clears the host', async () => {
    const empty = await compare(
      `
      const { engine, showPage } = window
      const a = document.getElementById('a')
      engine.renderDocument(args.tex)
      a.replaceChildren()
      const shown = showPage(a, engine.documentPage(0))
      shown.update('')
      const cleared = a.children.length
      shown.update(engine.documentPage(0))
      return [cleared, a.children.length]
    `,
      { tex: fixture('shapes') },
    )
    expect(empty).toEqual([0, 1])
  })
})
