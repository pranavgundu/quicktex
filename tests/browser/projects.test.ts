import { afterAll, beforeAll, expect, test } from 'bun:test'
import { readFileSync } from 'node:fs'
import type { Browser } from 'playwright-core'
import { dist, launch, serve, session } from '../support/browser'

const site = serve(dist)
let browser: Browser
beforeAll(async () => { browser = await launch() })
afterAll(async () => { await browser.close(); site.stop() })

test('uploaded TeX and image files compile together in the editor', async () => {
  const s = await session(browser)
  try {
    await s.page.goto(`${site.url}editor/`)
    await s.page.waitForSelector('article svg')
    await s.page.locator('input[type=file]').setInputFiles([
      { name: 'main.tex', mimeType: 'text/plain', buffer: Buffer.from(String.raw`\documentclass{article}\begin{document}\input{part}\includegraphics[width=2cm]{image}\end{document}`) },
      { name: 'part.tex', mimeType: 'text/plain', buffer: Buffer.from('Imported chapter text.') },
      { name: 'image.png', mimeType: 'image/png', buffer: readFileSync(new URL('../fixtures/image.png', import.meta.url)) },
    ])
    await s.page.waitForSelector('article svg image')
    expect(await s.page.locator('header').innerText()).toContain('3 project files')
    expect(await s.page.locator('.cm-content').innerText()).toContain(String.raw`\input{part}`)
    expect(await s.page.locator('[role=status]').first().innerText()).not.toContain('Ignored')
    await s.page.getByRole('button', { name: 'Clear files', exact: true }).click()
    await s.page.waitForFunction(() => !document.querySelector('article svg image'))
    expect(s.problems).toEqual([])
  } finally { await s.context.close() }
})

test('the editor loads Unicode fonts and their parser only when required', async () => {
  const s = await session(browser)
  const requested: string[] = []
  s.page.on('request', (request) => requested.push(request.url()))
  try {
    await s.page.goto(`${site.url}editor/`)
    await s.page.waitForSelector('article svg')
    expect(requested.some((url) => /NotoSerif|quicktex-fonts/.test(url))).toBe(false)
    const previous = await s.page.locator('article').first().innerHTML()
    await s.page.locator('input[type=file]').setInputFiles({ name: 'main.tex', mimeType: 'text/plain', buffer: Buffer.from(String.raw`\documentclass{article}\begin{document}` + 'Ελληνικά Русский 中文 日本語 한국어' + String.raw`\end{document}`) })
    await s.page.waitForFunction((html) => document.querySelector('article')?.innerHTML !== html, previous)
    expect(await s.page.locator('[role=status]').first().innerText()).not.toContain('Ignored')
    expect(await s.page.locator('.cm-content').innerText()).toContain('Ελληνικά')
    expect(requested.some((url) => /quicktex-fonts.*\.wasm/.test(url))).toBe(true)
    expect(requested.some((url) => /NotoSerifCJKsc/.test(url))).toBe(true)
    expect(s.problems).toEqual([])
  } finally { await s.context.close() }
})
