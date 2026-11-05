import { afterAll, beforeAll, beforeEach, describe, expect, test } from 'bun:test'
import type { Browser } from 'playwright-core'
import { dist, launch, serve, session, type Session } from '../support/browser'
import { fixture } from '../support/engine'

const site = serve(dist)
const lab = fixture('lab')
let browser: Browser
let s: Session

const sheet = () => s.page.locator('article').first()
const percent = async () => Number((await s.page.getByRole('button', { name: 'Reset zoom' }).innerText()).match(/\d+/)?.[0])
const sheetWidth = async () => (await sheet().boundingBox())!.width
const scroll = () => s.page.evaluate(() => {
  const node = document.querySelector('[data-scroller]') as HTMLElement
  return { left: node.scrollLeft, top: node.scrollTop, wide: node.scrollWidth > node.clientWidth }
})
const settle = (ms = 150) => s.page.waitForTimeout(ms)

async function open(tex = lab, options = {}) {
  await s?.context.close()
  s = await session(browser, { viewport: { width: 1500, height: 1000 }, ...options })
  await s.page.goto(`${site.url}editor/`)
  await s.page.waitForSelector('article svg', { state: 'attached' })
  await s.page.evaluate((value) => localStorage.setItem('quicktex-document', value), tex)
  await s.page.reload()
  await s.page.waitForSelector('article svg', { state: 'attached' })
  await settle(250)
}

async function middleOfSheet() {
  const box = (await s.page.locator('[data-scroller]').boundingBox())!
  return { x: box.x + box.width / 2, y: box.y + box.height / 2 }
}

async function wheel(x: number, y: number, deltaY: number, modifier = 'Control', times = 1) {
  await s.page.mouse.move(x, y)
  await s.page.keyboard.down(modifier)
  for (let i = 0; i < times; i++) {
    await s.page.mouse.wheel(0, deltaY)
    await settle(40)
  }
  await s.page.keyboard.up(modifier)
  await settle()
}

function pointUnder(x: number, y: number) {
  return s.page.evaluate(([px, py]) => {
    const article = document.elementFromPoint(px, py)?.closest('article')
    if (!article) return null
    const box = article.getBoundingClientRect()
    return [(px - box.left) / box.width, (py - box.top) / box.height]
  }, [x, y])
}

beforeAll(async () => {
  browser = await launch()
})

afterAll(async () => {
  await browser.close()
  site.stop()
})

beforeEach(async () => {
  await open()
})

describe('zooming the pages', () => {
  test('starts at 100%', async () => {
    expect(await percent()).toBe(100)
  })

  test('buttons step through the presets', async () => {
    await s.page.getByRole('button', { name: 'Zoom in' }).click()
    await settle()
    expect(await percent()).toBe(125)
    await s.page.getByRole('button', { name: 'Zoom out' }).click()
    await s.page.getByRole('button', { name: 'Zoom out' }).click()
    await settle()
    expect(await percent()).toBe(80)
  })

  test('zoom out stops at 25% and disables its button', async () => {
    const out = s.page.getByRole('button', { name: 'Zoom out' })
    for (let i = 0; i < 5; i++) await out.click()
    await settle()
    expect(await percent()).toBe(25)
    expect(await out.isDisabled()).toBe(true)
  })

  test('zoom in stops at 500% and disables its button', async () => {
    const zoomIn = s.page.getByRole('button', { name: 'Zoom in' })
    for (let i = 0; i < 6; i++) await zoomIn.click()
    await settle()
    expect(await percent()).toBe(500)
    expect(await zoomIn.isDisabled()).toBe(true)
  })

  test('page width follows the zoom exactly', async () => {
    const before = await sheetWidth()
    await s.page.getByRole('button', { name: 'Zoom in' }).click()
    await settle()
    expect(await sheetWidth()).toBeCloseTo(before * 1.25, 0)
  })

  test('ctrl scroll zooms in and out smoothly', async () => {
    const { x, y } = await middleOfSheet()
    await wheel(x, y, -30)
    const zoomed = await percent()
    expect(zoomed).toBeGreaterThan(100)
    expect(zoomed).toBeLessThan(150)
    await wheel(x, y, 30)
    expect(await percent()).toBe(100)
  })

  test('cmd scroll zooms too', async () => {
    const { x, y } = await middleOfSheet()
    await wheel(x, y, -30, 'Meta')
    expect(await percent()).toBeGreaterThan(100)
  })

  test('plain scroll still scrolls and does not zoom', async () => {
    const { x, y } = await middleOfSheet()
    await s.page.mouse.move(x, y)
    await s.page.mouse.wheel(0, 300)
    await settle()
    expect(await percent()).toBe(100)
    expect((await scroll()).top).toBeGreaterThan(0)
  })

  test('ctrl scroll does not zoom the whole browser page', async () => {
    const { x, y } = await middleOfSheet()
    const before = await s.page.evaluate(() => [window.visualViewport?.scale, window.devicePixelRatio])
    await wheel(x, y, -60, 'Control', 3)
    expect(await s.page.evaluate(() => [window.visualViewport?.scale, window.devicePixelRatio])).toEqual(before)
  })

  test('the spot under the cursor stays under the cursor', async () => {
    const box = (await sheet().boundingBox())!
    const x = box.x + box.width * 0.8
    const y = box.y + box.height * 0.3 + 40
    const before = (await pointUnder(x, y))!
    await wheel(x, y, -60, 'Control', 6)
    expect(await percent()).toBeGreaterThan(200)
    const after = (await pointUnder(x, y))!
    expect(Math.abs(after[0] - before[0])).toBeLessThan(0.005)
    expect(Math.abs(after[1] - before[1])).toBeLessThan(0.005)
  })

  test('the spot stays put zooming back out too', async () => {
    const box = (await sheet().boundingBox())!
    const x = box.x + box.width * 0.5
    const y = box.y + box.height * 0.5
    await wheel(x, y, -60, 'Control', 6)
    const before = (await pointUnder(x, y))!
    await wheel(x, y, 40, 'Control', 2)
    const after = (await pointUnder(x, y))!
    expect(Math.abs(after[0] - before[0])).toBeLessThan(0.005)
    expect(Math.abs(after[1] - before[1])).toBeLessThan(0.005)
  })

  test('zooming on the second page stays on the second page', async () => {
    await s.page.evaluate(() => {
      const node = document.querySelector('[data-scroller]') as HTMLElement
      node.scrollTop = node.scrollHeight
    })
    await settle(250)
    const second = (await s.page.locator('article').nth(1).boundingBox())!
    const x = second.x + second.width / 2
    const y = Math.max(second.y, 100) + 200
    const before = (await pointUnder(x, y))!
    await wheel(x, y, -60, 'Control', 4)
    const after = (await pointUnder(x, y))!
    expect(Math.abs(after[1] - before[1])).toBeLessThan(0.01)
  })

  test('buttons keep the middle of the view in place', async () => {
    const { x, y } = await middleOfSheet()
    await s.page.evaluate(() => {
      const node = document.querySelector('[data-scroller]') as HTMLElement
      node.scrollTop = 200
    })
    await settle()
    const before = (await pointUnder(x, y))!
    await s.page.getByRole('button', { name: 'Zoom in' }).click()
    await settle()
    const after = (await pointUnder(x, y))!
    expect(Math.abs(after[1] - before[1])).toBeLessThan(0.005)
  })

  test('double click zooms in at the spot, then returns', async () => {
    const box = (await sheet().boundingBox())!
    const x = box.x + box.width * 0.7
    const y = box.y + box.height * 0.4
    const before = (await pointUnder(x, y))!
    await s.page.mouse.dblclick(x, y)
    await settle()
    expect(await percent()).toBe(200)
    const after = (await pointUnder(x, y))!
    expect(Math.abs(after[0] - before[0])).toBeLessThan(0.005)
    await s.page.mouse.dblclick(x, y)
    await settle()
    expect(await percent()).toBe(100)
  })

  test('keys zoom when the pages have focus', async () => {
    await s.page.locator('[data-scroller]').focus()
    await s.page.keyboard.press('+')
    await settle()
    expect(await percent()).toBe(125)
    await s.page.keyboard.press('=')
    await settle()
    expect(await percent()).toBe(150)
    await s.page.keyboard.press('-')
    await settle()
    expect(await percent()).toBe(125)
    await s.page.keyboard.press('0')
    await settle()
    expect(await percent()).toBe(100)
  })

  test('cmd plus zooms the pages when they have focus', async () => {
    await s.page.locator('[data-scroller]').focus()
    await s.page.keyboard.press('ControlOrMeta+=')
    await settle()
    expect(await percent()).toBe(125)
  })

  test('clicking the pages gives them the keys', async () => {
    const box = (await sheet().boundingBox())!
    await s.page.mouse.click(box.x + 100, box.y + 100)
    await s.page.keyboard.press('+')
    await settle()
    expect(await percent()).toBe(125)
  })

  test('keys work from the zoom bar', async () => {
    await s.page.getByRole('button', { name: 'Zoom in' }).focus()
    await s.page.keyboard.press('-')
    await settle()
    expect(await percent()).toBe(80)
  })

  test('typing in the source does not zoom', async () => {
    await s.page.locator('.cm-content').click()
    await s.page.keyboard.type('+-0=')
    await settle()
    expect(await percent()).toBe(100)
  })

  test('dragging pans a zoomed page', async () => {
    await s.page.getByRole('button', { name: 'Reset zoom' }).focus()
    for (let i = 0; i < 3; i++) await s.page.getByRole('button', { name: 'Zoom in' }).click()
    await settle()
    const start = await scroll()
    expect(start.wide).toBe(true)
    const { x, y } = await middleOfSheet()
    await s.page.mouse.move(x, y)
    await s.page.mouse.down()
    await s.page.mouse.move(x - 120, y - 80, { steps: 5 })
    await s.page.mouse.up()
    await settle()
    const end = await scroll()
    expect(end.left - start.left).toBeCloseTo(120, 0)
    expect(end.top - start.top).toBeCloseTo(80, 0)
  })

  test('the pointer leaves pan mode after a drag', async () => {
    const { x, y } = await middleOfSheet()
    await s.page.mouse.move(x, y)
    await s.page.mouse.down()
    await s.page.mouse.move(x, y - 60, { steps: 3 })
    expect(await sheet().evaluate((node) => getComputedStyle(node).cursor)).toBe('grabbing')
    await s.page.mouse.up()
    await settle()
    expect(await sheet().evaluate((node) => getComputedStyle(node).cursor)).toBe('grab')
    const top = (await scroll()).top
    await s.page.mouse.move(x, y + 100)
    await settle()
    expect((await scroll()).top).toBe(top)
  })

  test('a tiny click does not move the page', async () => {
    const { x, y } = await middleOfSheet()
    await s.page.evaluate(() => {
      const node = document.querySelector('[data-scroller]') as HTMLElement
      node.scrollTop = 200
    })
    await settle()
    const start = await scroll()
    await s.page.mouse.move(x, y)
    await s.page.mouse.down()
    await s.page.mouse.move(x, y + 2)
    await s.page.mouse.up()
    await settle()
    expect(await scroll()).toEqual(start)
  })

  test('fit whole page shows all of it', async () => {
    await s.page.getByRole('button', { name: 'Fit whole page' }).click()
    await settle()
    const view = (await s.page.locator('[data-scroller]').boundingBox())!
    const box = (await sheet().boundingBox())!
    expect(box.height).toBeLessThanOrEqual(view.height)
    expect(box.y).toBeGreaterThanOrEqual(view.y)
    expect(box.y + box.height).toBeLessThanOrEqual(view.y + view.height)
    expect(view.height - box.height).toBeLessThan(60)
  })

  test('fit whole page stays on the page being read', async () => {
    await open('\\documentclass{article}\n\\begin{document}\na\\newpage b\\newpage c\n\\end{document}\n')
    await s.page.getByRole('button', { name: 'Zoom in' }).click()
    await s.page.getByRole('button', { name: 'Zoom in' }).click()
    await settle()
    await s.page.locator('article').nth(1).evaluate((node) => node.scrollIntoView({ block: 'center' }))
    await settle(250)
    await s.page.getByRole('button', { name: 'Fit whole page' }).click()
    await settle(250)
    const view = (await s.page.locator('[data-scroller]').boundingBox())!
    const middle = (await s.page.locator('article').nth(1).boundingBox())!
    expect(middle.y).toBeGreaterThanOrEqual(view.y)
    expect(middle.y + middle.height).toBeLessThanOrEqual(view.y + view.height)
  })

  test('reset returns to 100%', async () => {
    await s.page.getByRole('button', { name: 'Zoom in' }).click()
    await s.page.getByRole('button', { name: 'Reset zoom' }).click()
    await settle()
    expect(await percent()).toBe(100)
  })

  test('zoom survives a reload', async () => {
    await s.page.getByRole('button', { name: 'Zoom in' }).click()
    await settle(450)
    await s.page.reload()
    await s.page.waitForSelector('article svg')
    await settle()
    expect(await percent()).toBe(125)
  })

  test('a corrupt saved zoom falls back to 100%', async () => {
    await s.page.evaluate(() => localStorage.setItem('quicktex-zoom', 'nonsense'))
    await s.page.reload()
    await s.page.waitForSelector('article svg')
    await settle()
    expect(await percent()).toBe(100)
  })

  test('zooming never rebuilds the page content', async () => {
    await sheet().locator('svg').evaluate((svg) => {
      for (const child of svg.children) child.setAttribute('data-same', '')
    })
    const { x, y } = await middleOfSheet()
    await wheel(x, y, -60, 'Control', 4)
    await s.page.getByRole('button', { name: 'Fit whole page' }).click()
    await settle()
    const untouched = await sheet().locator('svg').evaluate((svg) => [...svg.children].every((child) => child.hasAttribute('data-same')))
    expect(untouched).toBe(true)
  })

  test('editing keeps the zoom and scroll position', async () => {
    const { x, y } = await middleOfSheet()
    await wheel(x, y, -60, 'Control', 4)
    const zoomed = await percent()
    const before = await scroll()
    await s.page.locator('.cm-content').click()
    await s.page.keyboard.type('x')
    await settle(300)
    expect(await percent()).toBe(zoomed)
    expect(await scroll()).toEqual(before)
  })

  test('a zoomed page stays sharp because it is redrawn as vectors', async () => {
    await s.page.getByRole('button', { name: 'Zoom in' }).click()
    await settle()
    const box = await sheet().locator('svg').evaluate((svg) => {
      const rect = svg.getBoundingClientRect()
      return [rect.width, svg.parentElement!.getBoundingClientRect().width, getComputedStyle(svg).transform]
    })
    expect(box[0]).toBeCloseTo(box[1] as number, 0)
    expect(box[2]).toBe('none')
  })

  test('no console problems across a whole zoom session', async () => {
    const { x, y } = await middleOfSheet()
    await wheel(x, y, -60, 'Control', 3)
    await s.page.mouse.dblclick(x, y)
    await s.page.getByRole('button', { name: 'Fit whole page' }).click()
    await settle()
    expect(s.problems).toEqual([])
  })
})

describe('pinching on a touch screen', () => {
  async function pinch(from: number, to: number) {
    const client = await s.context.newCDPSession(s.page)
    const box = (await s.page.locator('[data-scroller]').boundingBox())!
    const cx = box.x + box.width / 2
    const cy = box.y + box.height / 2
    const points = (gap: number) => [{ x: cx - gap / 2, y: cy, id: 0 }, { x: cx + gap / 2, y: cy, id: 1 }]
    await client.send('Input.dispatchTouchEvent', { type: 'touchStart', touchPoints: points(from) })
    for (let i = 1; i <= 8; i++) {
      await client.send('Input.dispatchTouchEvent', { type: 'touchMove', touchPoints: points(from + ((to - from) * i) / 8) })
      await settle(20)
    }
    await client.send('Input.dispatchTouchEvent', { type: 'touchEnd', touchPoints: [] })
    await settle()
  }

  beforeEach(async () => {
    await open(lab, { hasTouch: true, viewport: { width: 1000, height: 800 } })
  })

  test('spreading two fingers zooms in by their ratio', async () => {
    await pinch(100, 200)
    expect(await percent()).toBe(200)
  })

  test('pinching two fingers zooms out by their ratio', async () => {
    await pinch(200, 100)
    expect(await percent()).toBe(50)
  })

  test('a second pinch continues from the first', async () => {
    await pinch(100, 200)
    await pinch(100, 150)
    expect(await percent()).toBe(300)
  })

  test('pinching stops at the limits', async () => {
    await pinch(40, 400)
    await pinch(40, 200)
    expect(await percent()).toBe(500)
  })

  test('pinching does not zoom the browser', async () => {
    await pinch(100, 200)
    expect(await s.page.evaluate(() => window.visualViewport?.scale)).toBe(1)
  })
})
