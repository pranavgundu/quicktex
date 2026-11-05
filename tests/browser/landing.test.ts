import { afterAll, beforeAll, beforeEach, describe, expect, test } from 'bun:test'
import type { Browser } from 'playwright-core'
import { dist, launch, serve, session, type Session } from '../support/browser'

const site = serve(dist)
let browser: Browser
let s: Session

const formula = () => s.page.locator('[aria-label="Rendered formula"] svg').first()
const radio = (name: string) => s.page.getByRole('radio', { name })
const input = () => s.page.locator('textarea')

async function open() {
  await s.page.goto(site.url)
  await s.page.waitForSelector('textarea')
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
  await open()
})

describe('formula page', () => {
  test('loads with a formula and no console problems', async () => {
    expect((await input().inputValue()).length).toBeGreaterThan(0)
    expect(await formula().count()).toBe(1)
    expect(s.problems).toEqual([])
  })

  test('has no example buttons', async () => {
    expect(await s.page.getByRole('button', { name: /Euler|Gaussian|Basel/ }).count()).toBe(0)
  })

  test('typing updates the formula', async () => {
    const before = await formula().innerHTML()
    await input().fill('\\frac{a}{b}')
    await s.page.waitForTimeout(150)
    expect(await formula().innerHTML()).not.toBe(before)
  })

  test('a mistake shows the error and keeps the page working', async () => {
    await input().fill('\\nope')
    await s.page.waitForTimeout(150)
    expect(await s.page.getByText(/unknown command/i).count()).toBeGreaterThan(0)
    await input().fill('x')
    await s.page.waitForTimeout(150)
    expect(await s.page.getByText(/unknown command/i).count()).toBe(0)
    expect(await formula().count()).toBe(1)
  })

  test('starts in display mode', async () => {
    expect(await radio('Display').isChecked()).toBe(true)
  })

  test('inline renders differently', async () => {
    await input().fill('\\sum_{i=1}^n i')
    await s.page.waitForTimeout(150)
    const display = await formula().getAttribute('width')
    await radio('Inline').check({ force: true })
    await s.page.waitForTimeout(150)
    expect(await formula().getAttribute('width')).not.toBe(display)
  })

  test('remembers inline across a reload', async () => {
    await radio('Inline').check({ force: true })
    await s.page.reload()
    await s.page.waitForSelector('textarea')
    expect(await radio('Inline').isChecked()).toBe(true)
  })

  test('remembers display across a reload', async () => {
    await radio('Inline').check({ force: true })
    await radio('Display').check({ force: true })
    await s.page.reload()
    await s.page.waitForSelector('textarea')
    expect(await radio('Display').isChecked()).toBe(true)
  })

  test('ignores a corrupt saved mode', async () => {
    await s.page.evaluate(() => localStorage.setItem('quicktex-layout', 'nonsense'))
    await s.page.reload()
    await s.page.waitForSelector('textarea')
    expect(await radio('Display').isChecked()).toBe(true)
  })

  test('works when storage is blocked', async () => {
    await s.page.addInitScript(() => {
      Object.defineProperty(window, 'localStorage', {
        get() {
          throw new Error('blocked')
        },
      })
    })
    await s.page.reload()
    await s.page.waitForSelector('textarea')
    expect(await formula().count()).toBe(1)
  })

  test('dark mode toggles and persists', async () => {
    await s.page.getByRole('button', { name: 'Switch to dark mode' }).click()
    expect(await s.page.evaluate(() => document.documentElement.classList.contains('dark'))).toBe(true)
    await s.page.reload()
    await s.page.waitForSelector('textarea')
    expect(await s.page.evaluate(() => document.documentElement.classList.contains('dark'))).toBe(true)
    await s.page.getByRole('button', { name: 'Switch to light mode' }).click()
    expect(await s.page.evaluate(() => document.documentElement.classList.contains('dark'))).toBe(false)
  })

  test('follows the system theme by default', async () => {
    const dark = await session(browser, { colorScheme: 'dark' })
    await dark.page.goto(site.url)
    await dark.page.waitForSelector('textarea')
    expect(await dark.page.evaluate(() => document.documentElement.classList.contains('dark'))).toBe(true)
    await dark.context.close()
  })

  test('links to the full editor', async () => {
    await s.page.locator('a[href*="editor"]').first().click()
    await s.page.waitForSelector('article svg')
    expect(s.page.url()).toContain('/editor/')
  })

  test('does not scroll sideways on a phone', async () => {
    await s.page.setViewportSize({ width: 390, height: 780 })
    await s.page.waitForTimeout(100)
    expect(await s.page.evaluate(() => document.documentElement.scrollWidth - innerWidth)).toBeLessThanOrEqual(0)
  })

  test('has no em dashes in the copy', async () => {
    expect(await s.page.evaluate(() => document.body.innerText)).not.toContain('\u2014')
  })
})

describe('render time', () => {
  const time = () => s.page.locator('.sr-only', { hasText: 'Rendered in' })

  test('reads as an exact time in microseconds or milliseconds', async () => {
    await input().fill('x^2')
    await s.page.waitForTimeout(300)
    expect(await time().textContent()).toMatch(/^Rendered in [\d.]+ (µs|ms)$/)
  })

  test('a tiny formula is timed below the resolution of the browser clock', async () => {
    await input().fill('x')
    await s.page.waitForTimeout(300)
    const text = (await time().textContent()) ?? ''
    expect(text).toMatch(/µs$/)
    expect(Number(text.match(/[\d.]+/)?.[0])).toBeGreaterThan(0)
  })
})
