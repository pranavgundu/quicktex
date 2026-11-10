import { afterAll, beforeAll, expect, test } from 'bun:test'
import type { Browser, WebSocketRoute } from 'playwright-core'
import { dist, launch, serve, session, type Session } from '../support/browser'

const site = serve(dist)
let browser: Browser
beforeAll(async () => { browser = await launch() })
afterAll(async () => { await browser?.close(); site.stop() })

// In-memory signaling only. Separate contexts cannot use BroadcastChannel, so
// document traffic must actually travel over WebRTC. No public service needed.
const sockets = new Map<WebSocketRoute, Set<string>>()
async function signaling(s: Session) {
  await s.context.routeWebSocket('wss://y-webrtc-eu.fly.dev', socket => {
    sockets.set(socket, new Set())
    socket.onClose(() => { sockets.delete(socket) })
    socket.onMessage(raw => {
      const message = JSON.parse(String(raw))
      if (message.type === 'subscribe') for (const topic of message.topics) sockets.get(socket)?.add(topic)
      if (message.type === 'publish') for (const [peer, topics] of sockets) if (topics.has(message.topic)) peer.send(JSON.stringify(message))
      if (message.type === 'ping') socket.send(JSON.stringify({ type: 'pong' }))
    })
  })
}

test('two browsers edit live, undo only local edits, and never overwrite local saves', async () => {
  const host = await session(browser), guest = await session(browser)
  try {
    await signaling(host); await signaling(guest)
    await host.page.goto(site.url + 'editor/')
    await host.page.waitForSelector('article svg')
    await host.page.getByRole('button', { name: 'Share', exact: true }).click()
    await host.page.getByRole('button', { name: 'Start temporary session' }).click()
    await host.page.waitForTimeout(500)
    if (await host.page.getByRole('textbox', { name: 'Session link' }).count() === 0) throw new Error(`${await host.page.locator('body').innerText()}\n${host.problems.join('\n')}`)
    const link = await host.page.getByRole('textbox', { name: 'Session link' }).inputValue()
    const original = await host.page.evaluate(() => localStorage.getItem('quicktex-document'))
    await host.page.getByRole('button', { name: 'Close', exact: true }).click()
    await guest.page.goto(site.url + 'editor/')
    await guest.page.evaluate(() => localStorage.setItem('quicktex-document', 'PRIVATE LOCAL DRAFT'))
    await guest.page.goto(link)
    await guest.page.reload() // fragment navigation alone does not remount the app
    await guest.page.waitForSelector('.cm-content', { timeout: 25000 })
    const hostEditor = host.page.locator('.cm-content'), guestEditor = guest.page.locator('.cm-content')
    await hostEditor.click(); await host.page.keyboard.press('ControlOrMeta+Home')
    await host.page.keyboard.type('% HOST EDIT\n')
    await guest.page.waitForFunction(() => document.querySelector('.cm-content')?.textContent?.includes('HOST EDIT'), { timeout: 15000 })
    await guestEditor.click(); await guest.page.keyboard.press('ControlOrMeta+Home')
    await guest.page.keyboard.type('% GUEST EDIT\n')
    await host.page.waitForFunction(() => document.querySelector('.cm-content')?.textContent?.includes('GUEST EDIT'), { timeout: 15000 })
    await guest.page.keyboard.press('ControlOrMeta+z')
    await host.page.waitForFunction(() => !document.querySelector('.cm-content')?.textContent?.includes('GUEST EDIT'), { timeout: 15000 })
    expect(await hostEditor.innerText()).toContain('HOST EDIT')
    await host.page.locator('input[type=file]').setInputFiles({ name: 'chapter.tex', mimeType: 'text/plain', buffer: Buffer.from('Shared chapter') })
    await guest.page.getByRole('complementary', { name: 'Project navigation' }).getByRole('button', { name: 'chapter.tex', exact: true }).click()
    await guestEditor.click(); await guest.page.keyboard.press('ControlOrMeta+End')
    await guest.page.keyboard.type(' from guest')
    await host.page.getByRole('complementary', { name: 'Project navigation' }).getByRole('button', { name: 'chapter.tex', exact: true }).click()
    await host.page.waitForFunction(() => document.querySelector('.cm-content')?.textContent?.includes('from guest'))
    const download = host.page.waitForEvent('download')
    await host.page.getByRole('button', { name: 'Download project', exact: true }).click()
    expect((await download).suggestedFilename()).toBe('quicktex-project.zip')
    await host.page.waitForTimeout(800)
    expect(await host.page.evaluate(() => localStorage.getItem('quicktex-document'))).toBe(original)
    expect(await guest.page.evaluate(() => localStorage.getItem('quicktex-document'))).toBe('PRIVATE LOCAL DRAFT')
    expect(host.problems.filter(p => !p.includes('WebSocket'))).toEqual([])
    expect(guest.problems.filter(p => !p.includes('WebSocket'))).toEqual([])
  } finally {
    await host.context.close(); await guest.context.close()
  }
}, 60000)

test('manual code exchange works when public signaling is unavailable', async () => {
  const host = await session(browser), guest = await session(browser)
  try {
    for (const s of [host, guest]) await s.context.routeWebSocket('wss://y-webrtc-eu.fly.dev', socket => socket.close())
    await host.page.goto(site.url + 'editor/')
    await host.page.waitForSelector('article svg')
    await host.page.getByRole('button', { name: 'Share', exact: true }).click()
    await host.page.getByRole('button', { name: 'Start temporary session' }).click()
    const link = await host.page.getByRole('textbox', { name: 'Session link' }).inputValue()
    await host.page.locator('summary').filter({ hasText: 'Not connecting?' }).click()
    await host.page.getByRole('button', { name: 'Create invitation code', exact: true }).click()
    await host.page.getByRole('textbox', { name: 'Generated connection code' }).waitFor()
    const invitation = await host.page.getByRole('textbox', { name: 'Generated connection code' }).inputValue()
    await guest.page.goto(link)
    await guest.page.getByRole('button', { name: 'Share (1)', exact: true }).click()
    await guest.page.locator('summary').filter({ hasText: 'Not connecting?' }).click()
    await guest.page.getByRole('textbox', { name: 'Received connection code' }).fill(invitation)
    await guest.page.getByRole('button', { name: 'Create reply code', exact: true }).click()
    await guest.page.getByRole('textbox', { name: 'Generated connection code' }).waitFor()
    const reply = await guest.page.getByRole('textbox', { name: 'Generated connection code' }).inputValue()
    await host.page.getByRole('textbox', { name: 'Received connection code' }).fill(reply)
    await host.page.getByRole('button', { name: 'Connect using reply', exact: true }).click()
    for (const s of [host, guest]) await s.page.getByRole('button', { name: 'Close', exact: true }).click()
    await guest.page.waitForSelector('.cm-content')
    await host.page.locator('.cm-content').click()
    await host.page.keyboard.press('ControlOrMeta+Home')
    await host.page.keyboard.type('% NO SIGNALING SERVER\n')
    await guest.page.waitForFunction(() => document.querySelector('.cm-content')?.textContent?.includes('NO SIGNALING SERVER'))
    expect(await guest.page.locator('.cm-content').innerText()).toContain('NO SIGNALING SERVER')
    expect(host.problems.filter(p => !p.includes('WebSocket'))).toEqual([])
    expect(guest.problems.filter(p => !p.includes('WebSocket'))).toEqual([])
  } finally { await host.context.close(); await guest.context.close() }
}, 60000)
