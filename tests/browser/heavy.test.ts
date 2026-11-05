import { afterAll, beforeAll, beforeEach, describe, expect, test } from 'bun:test'
import type { Browser } from 'playwright-core'
import { dist, launch, serve, session, type Session } from '../support/browser'

const tree = String.raw`\documentclass[border=10pt]{standalone}
\usepackage{tikz}
\definecolor{treegreen}{RGB}{20,130,70}
\newcommand{\fractaltree}[1]{%
    \ifnum#1>0
        \pgfmathtruncatemacro{\next}{#1-1}
        \pgfmathsetmacro{\len}{0.72^(16-#1)}
        \draw[treegreen, line width=0.25pt] (0,0) -- (0,\len);
        \begin{scope}[shift={(0,\len)}]
            \begin{scope}[rotate=25]\fractaltree{\next}\end{scope}
            \begin{scope}[rotate=-25]\fractaltree{\next}\end{scope}
        \end{scope}
    \fi
}
\begin{document}
\begin{tikzpicture}
    \node at (0,2) {Fractal Tree};
    \begin{scope}[scale=3]\fractaltree{16}\end{scope}
\end{tikzpicture}
\end{document}
`

const site = serve(dist)
const titleLine = tree.split('\n').findIndex((line) => line.includes('Fractal Tree'))
const budget = 150
let browser: Browser
let s: Session

const paths = () => s.page.locator('article svg path').count()
const longest = () => s.page.evaluate(() => Math.max(0, ...(window as unknown as { tasks: number[] }).tasks))
const glyphs = () => s.page.evaluate(() => document.querySelector('article svg')?.innerHTML.length ?? 0)
const shown = () => s.page.evaluate(() => (document.querySelector('.cm-content') as HTMLElement).innerText)

async function toTitle() {
  await s.page.locator('.cm-content').click()
  await s.page.keyboard.press('ControlOrMeta+ArrowUp')
  for (let i = 0; i < titleLine; i++) await s.page.keyboard.press('ArrowDown')
  await s.page.keyboard.press('End')
  for (let i = 0; i < 3; i++) await s.page.keyboard.press('ArrowLeft')
  await s.page.waitForTimeout(600)
  await s.page.evaluate(() => {
    const tasks: number[] = []
    Object.assign(window, { tasks })
    new PerformanceObserver((list) => list.getEntries().forEach((entry) => tasks.push(entry.duration))).observe({ type: 'longtask' })
  })
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
  s = await session(browser)
  await s.page.goto(`${site.url}editor/`)
  await s.page.waitForSelector('article svg')
  await s.page.evaluate((tex) => localStorage.setItem('quicktex-document', tex), tree)
  await s.page.reload()
  await s.page.waitForFunction(() => document.querySelectorAll('article svg path').length > 20)
  await toTitle()
})

describe('a heavy document', () => {
  test('draws with few page elements', async () => {
    expect(await paths()).toBeLessThan(200)
  })

  test('typing never blocks the page', async () => {
    for (let i = 0; i < 12; i++) {
      await s.page.keyboard.type('x')
      await s.page.waitForTimeout(40)
    }
    expect(await longest()).toBeLessThan(budget)
  })

  test('a held delete lands at once', async () => {
    const before = (await shown()).length
    await Promise.all(Array.from({ length: 6 }, () => s.page.keyboard.press('Backspace')))
    expect((await shown()).length).toBe(before - 6)
    expect(await longest()).toBeLessThan(budget)
  })

  test('the pages catch up with the newest text', async () => {
    const before = await glyphs()
    await Promise.all(Array.from({ length: 6 }, () => s.page.keyboard.press('Backspace')))
    await s.page.waitForFunction((size) => (document.querySelector('article svg')?.innerHTML.length ?? 0) < size, before)
    await s.page.waitForTimeout(800)
    const settled = await glyphs()
    await s.page.waitForTimeout(800)
    expect(await glyphs()).toBe(settled)
  })

  test('a mistake keeps the last good pages and leaves typing alone', async () => {
    await s.page.keyboard.press('ArrowRight')
    await s.page.keyboard.press('ArrowRight')
    await s.page.keyboard.press('Backspace')
    await s.page.locator('[role=status]').getByText('on line', { exact: false }).first().waitFor({ state: 'attached' })
    expect(await paths()).toBeGreaterThan(20)
    expect(await longest()).toBeLessThan(budget)
  })

  test('the status shows a time after typing stops', async () => {
    await s.page.keyboard.type('x')
    await s.page.waitForTimeout(1200)
    expect(await s.page.locator('[role=status] .sr-only').first().textContent()).toMatch(/[\d.]+ ms, PDF [\d.]+ (µs|ms)$/)
  })
})
