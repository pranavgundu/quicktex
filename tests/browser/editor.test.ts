import { afterAll, beforeAll, beforeEach, describe, expect, test } from 'bun:test'
import { readFileSync } from 'node:fs'
import type { Browser } from 'playwright-core'
import { dist, launch, serve, session, type Session } from '../support/browser'
import { fixture } from '../support/engine'

const site = serve(dist)
let browser: Browser
let s: Session

const lab = fixture('lab')
const status = () => s.page.locator('[role=status]').first().innerText()
const sheets = () => s.page.locator('article svg')
const source = () => s.page.locator('.cm-content')

async function open(tex?: string) {
  await s.page.goto(`${site.url}editor/`)
  await s.page.waitForSelector('article svg')
  if (tex === undefined) return
  await s.page.evaluate((value) => localStorage.setItem('quicktex-document', value), tex)
  await s.page.reload()
  await s.page.waitForSelector('article svg')
  await s.page.waitForTimeout(250)
}

async function typeBeforeEnd(text: string) {
  await source().click()
  await s.page.keyboard.press('ControlOrMeta+End')
  await s.page.keyboard.press('ArrowUp')
  await s.page.keyboard.press('ArrowUp')
  await s.page.keyboard.type(text)
  await s.page.waitForTimeout(300)
}

beforeAll(async () => {
  browser = await launch()
})

afterAll(async () => {
  await browser.close()
  site.stop()
})

beforeEach(async () => {
  await s?.context.close()
  s = await session(browser, { viewport: { width: 1500, height: 1000 } })
})

describe('editor', () => {
  test('opens on the starter document with no console problems', async () => {
    await open()
    expect(await status()).toMatch(/^\d+ pages?, /)
    expect(s.problems).toEqual([])
  })

  test('renders the lab document on two pages', async () => {
    await open(lab)
    expect(await status()).toStartWith('2 pages, ')
    expect(await s.page.locator('article').count()).toBe(2)
  })

  test('status shows layout and pdf time', async () => {
    await open(lab)
    await s.page.waitForTimeout(300)
    expect(await status()).toMatch(/^2 pages, \d+ words, [\d.]+ (µs|ms), PDF [\d.]+ (µs|ms)$/)
  })

  test('the first page holds drawings and turned text', async () => {
    await open(lab)
    const first = await sheets().first().innerHTML()
    expect(first).toContain('stroke=')
    expect(first).toContain('matrix(')
  })

  test('page svgs split into many elements', async () => {
    await open(lab)
    expect(await sheets().first().evaluate((svg) => svg.children.length)).toBeGreaterThan(50)
  })

  test('typing updates the page and keeps its elements', async () => {
    await open(lab)
    await s.page.evaluate(() => {
      const scroller = document.querySelector('[data-scroller]') as HTMLElement
      scroller.scrollTop = scroller.scrollHeight
    })
    await s.page.waitForTimeout(250)
    const marked = await sheets().last().evaluate((svg) => {
      for (const child of svg.children) child.setAttribute('data-old', '')
      return svg.children.length
    })
    await typeBeforeEnd('x')
    const kept = await sheets().last().evaluate((svg) => svg.querySelectorAll('[data-old]').length)
    expect(kept).toBeGreaterThan(marked / 2)
    expect(kept).toBeLessThanOrEqual(marked)
  })

  test('a mistake shows the line and keeps the last good pages', async () => {
    await open(lab)
    await typeBeforeEnd('\\begin{tikzpicture}\\end{quote}')
    expect(await status()).toContain('on line')
    expect(await status()).toContain('Pages show the last good version')
    expect(await s.page.locator('article').count()).toBe(2)
  })

  test('clicking an error moves the cursor to it', async () => {
    await s.page.goto(`${site.url}editor/`)
    await s.page.evaluate(() => localStorage.setItem('quicktex-document', '\\documentclass{article}\n\\begin{document}\nfirst\n\n\\nope\n\\end{document}\n'))
    await s.page.reload()
    await s.page.waitForSelector('.cm-content')
    await s.page.waitForTimeout(200)
    await s.page.locator('[role=status] button').click()
    await s.page.waitForTimeout(100)
    expect(await s.page.locator('.cm-activeLine').innerText()).toContain('\\nope')
  })

  test('a broken saved document still reports its line', async () => {
    await s.page.goto(`${site.url}editor/`)
    await s.page.evaluate(() => localStorage.setItem('quicktex-document', '\\begin{document}\n\n\\nope\n\\end{document}'))
    await s.page.reload()
    await s.page.waitForSelector('.cm-content')
    await s.page.waitForTimeout(200)
    expect(await status()).toContain('Ignored \\nope on line 3')
  })

  test('fixing a mistake brings the pages back', async () => {
    await open(lab)
    await typeBeforeEnd('\\nope')
    expect(await status()).toContain('Ignored \\nope')
    for (let i = 0; i < 5; i++) await s.page.keyboard.press('Backspace')
    await s.page.waitForTimeout(300)
    expect(await status()).toStartWith('2 pages, ')
  })

  test('downloads a two page pdf', async () => {
    await open(lab)
    const [download] = await Promise.all([s.page.waitForEvent('download'), s.page.getByRole('button', { name: 'Download PDF' }).click()])
    expect(download.suggestedFilename()).toBe('document.pdf')
    const bytes = readFileSync(await download.path())
    expect(bytes.subarray(0, 8).toString()).toBe('%PDF-1.4')
    expect(bytes.toString('latin1').match(/\/Count (\d+)/)?.[1]).toBe('2')
  })

  test('keeps the document across a reload', async () => {
    await open(lab)
    await typeBeforeEnd('keptword')
    await s.page.waitForTimeout(500)
    expect(await s.page.evaluate(() => localStorage.getItem('quicktex-document'))).toContain('keptword')
    await s.page.reload()
    await s.page.waitForSelector('article svg')
    await source().click()
    await s.page.keyboard.press('ControlOrMeta+End')
    await s.page.waitForTimeout(100)
    expect(await source().innerText()).toContain('keptword')
  })

  test('zoom buttons change the page size', async () => {
    await open(lab)
    const width = async () => (await s.page.locator('article').first().boundingBox())!.width
    const before = await width()
    await s.page.getByRole('button', { name: 'Zoom in' }).click()
    await s.page.waitForTimeout(150)
    expect(await width()).toBeGreaterThan(before)
    await s.page.getByRole('button', { name: 'Reset zoom' }).click()
    await s.page.waitForTimeout(150)
    expect(Math.round(await width())).toBe(Math.round(before))
    await s.page.getByRole('button', { name: 'Zoom out' }).click()
    await s.page.waitForTimeout(150)
    expect(await width()).toBeLessThan(before)
  })

  test('zooming does not reparse the page', async () => {
    await open(lab)
    await sheets().first().evaluate((svg) => svg.setAttribute('data-same', ''))
    await s.page.getByRole('button', { name: 'Zoom in' }).click()
    await s.page.waitForTimeout(150)
    expect(await sheets().first().getAttribute('data-same')).toBe('')
  })

  test('only pages near the view are drawn', async () => {
    const long = `\\documentclass{article}\n\\begin{document}\n${'page\\newpage\n'.repeat(20)}\\end{document}`
    await open(long)
    expect(await sheets().count()).toBeLessThan(21)
    await s.page.evaluate(() => {
      const scroller = document.querySelector('[data-scroller]') as HTMLElement
      scroller.scrollTop = scroller.scrollHeight
    })
    await s.page.waitForTimeout(300)
    expect(await s.page.locator('article').last().locator('svg').count()).toBe(1)
  })

  test('fewer pages after an edit removes the extra sheets', async () => {
    await open('\\documentclass{article}\n\\begin{document}\na\\newpage b\\newpage c\n\\end{document}\n')
    expect(await s.page.locator('article').count()).toBe(3)
    await open('\\documentclass{article}\n\\begin{document}\na\n\\end{document}\n')
    expect(await s.page.locator('article').count()).toBe(1)
  })

  test('pane split can be dragged and is remembered', async () => {
    await open(lab)
    const handle = s.page.getByRole('separator').or(s.page.locator('button.cursor-col-resize')).first()
    const box = (await handle.boundingBox())!
    await s.page.mouse.move(box.x + 4, box.y + 200)
    await s.page.mouse.down()
    await s.page.mouse.move(box.x - 200, box.y + 200, { steps: 4 })
    await s.page.mouse.up()
    await s.page.waitForTimeout(100)
    const saved = await s.page.evaluate(() => localStorage.getItem('quicktex-split'))
    expect(Number(saved)).toBeLessThan(50)
  })

  test('arrow keys nudge the split', async () => {
    await open(lab)
    await s.page.locator('button.cursor-col-resize').focus()
    await s.page.keyboard.press('ArrowLeft')
    expect(Number(await s.page.evaluate(() => localStorage.getItem('quicktex-split')))).toBe(48)
  })

  test('dark mode leaves the pages white', async () => {
    await open(lab)
    await s.page.getByRole('button', { name: 'Switch to dark mode' }).click()
    await s.page.waitForTimeout(100)
    const colors = await s.page.locator('article').first().evaluate((node) => {
      const style = getComputedStyle(node)
      return [style.backgroundColor, style.color]
    })
    expect(colors).toEqual(['rgb(255, 255, 255)', 'rgb(0, 0, 0)'])
  })

  test('back link returns to the formula page', async () => {
    await open()
    await s.page.getByRole('link', { name: 'Back to the formula page' }).click()
    await s.page.waitForSelector('textarea')
    expect(s.page.url()).not.toContain('/editor/')
  })

  test('a phone shows one pane at a time without sideways scroll', async () => {
    await s.page.setViewportSize({ width: 390, height: 780 })
    await s.page.goto(`${site.url}editor/`)
    await s.page.waitForSelector('.cm-content')
    await s.page.waitForTimeout(200)
    expect(await s.page.evaluate(() => document.documentElement.scrollWidth - innerWidth)).toBeLessThanOrEqual(0)
    expect(await s.page.locator('.cm-content').isVisible()).toBe(true)
    expect(await s.page.locator('article').first().isVisible()).toBe(false)
  })

  test('finishes the whole session with no console problems', async () => {
    await open(lab)
    await typeBeforeEnd('more text ')
    await s.page.getByRole('button', { name: 'Zoom in' }).click()
    await s.page.getByRole('button', { name: 'Switch to dark mode' }).click()
    await s.page.waitForTimeout(250)
    expect(s.problems).toEqual([])
  })
})
