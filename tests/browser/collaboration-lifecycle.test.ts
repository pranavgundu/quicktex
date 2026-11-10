import { afterAll, afterEach, beforeAll, expect, test } from 'bun:test'
import { readFileSync } from 'node:fs'
import { unzipSync, strFromU8 } from 'fflate'
import type { Browser, WebSocketRoute } from 'playwright-core'
import { dist, launch, serve, session, type Session } from '../support/browser'

const site = serve(dist)
let browser: Browser
const clients: Session[] = []
const sockets = new Map<WebSocketRoute, Set<string>>()
beforeAll(async () => { browser = await launch() })
afterEach(async () => { for (const client of clients.splice(0)) await client.context.close(); sockets.clear() })
afterAll(async () => { await browser?.close(); site.stop() })

async function client() {
  const s = await session(browser); clients.push(s)
  await s.context.routeWebSocket('wss://y-webrtc-eu.fly.dev', socket => {
    sockets.set(socket, new Set()); socket.onClose(() => { sockets.delete(socket) })
    socket.onMessage(raw => {
      const message = JSON.parse(String(raw))
      if (message.type === 'subscribe') for (const topic of message.topics) sockets.get(socket)?.add(topic)
      if (message.type === 'unsubscribe') for (const topic of message.topics) sockets.get(socket)?.delete(topic)
      if (message.type === 'publish') for (const [peer, topics] of sockets) if (topics.has(message.topic)) peer.send(JSON.stringify(message))
      if (message.type === 'ping') socket.send(JSON.stringify({ type: 'pong' }))
    })
  })
  return s
}
async function start(s: Session) {
  await s.page.goto(site.url + 'editor/'); await s.page.waitForSelector('article svg')
  await s.page.getByRole('button', { name: 'Share', exact: true }).click()
  await s.page.getByRole('button', { name: 'Start temporary session' }).click()
  const link = await s.page.getByRole('textbox', { name: 'Session link' }).inputValue()
  await s.page.getByRole('button', { name: 'Close', exact: true }).click()
  return link
}
async function join(s: Session, link: string) { await s.page.goto(link); await s.page.waitForSelector('.cm-content', { timeout: 20000 }) }
async function insert(s: Session, text: string) {
  await s.page.locator('.cm-content').click(); await s.page.keyboard.press('ControlOrMeta+Home'); await s.page.keyboard.type(text)
}
async function contains(s: Session, text: string) {
  await s.page.waitForFunction(value => document.querySelector('.cm-content')?.textContent?.includes(value), text, { timeout: 15000 })
}
async function upload(s: Session, name: string, content: string | Buffer) {
  await s.page.locator('input[type=file]').setInputFiles({ name, mimeType: 'application/octet-stream', buffer: typeof content === 'string' ? Buffer.from(content) : content })
}
const navigation = (s: Session) => s.page.getByRole('complementary', { name: 'Project navigation' })
async function open(s: Session, name: string) { await navigation(s).getByRole('button', { name, exact: true }).click() }
function clean() { for (const s of clients) expect(s.problems).toEqual([]) }

test('late join receives edits already made and a reloaded peer catches up', async () => {
  const host = await client(), guest = await client(); const link = await start(host)
  await insert(host, '% BEFORE JOIN\n'); await join(guest, link); await contains(guest, 'BEFORE JOIN')
  await guest.page.reload(); await guest.page.waitForSelector('.cm-content')
  await insert(host, '% AFTER RELOAD\n'); await contains(guest, 'AFTER RELOAD')
  expect((await guest.page.locator('.cm-content').innerText()).match(/BEFORE JOIN/g)).toHaveLength(1)
  clean()
}, 60000)

test('three peers keep editing after the original creator closes', async () => {
  const host = await client(), a = await client(), b = await client(); const link = await start(host)
  await join(a, link); await join(b, link)
  await host.context.close()
  await insert(a, '% CREATOR LEFT\n'); await contains(b, 'CREATOR LEFT')
  await insert(b, '% STILL EDITING\n'); await contains(a, 'STILL EDITING')
  const source = (s: Session) => s.page.locator('.cm-content').evaluate(node => {
    const copy = node.cloneNode(true) as HTMLElement
    copy.querySelectorAll('.cm-ySelectionCaret').forEach(caret => caret.remove())
    return [...copy.querySelectorAll('.cm-line')].map(line => line.textContent).join('\n')
  })
  expect(await source(a)).toBe(await source(b))
  clean()
}, 60000)

test('remote rename preserves the selected file and remote deletion returns to root', async () => {
  const host = await client(), guest = await client(); const link = await start(host); await join(guest, link)
  await upload(host, 'part.tex', 'Shared chapter'); await open(guest, 'part.tex')
  await host.page.getByLabel('Actions for part.tex').click()
  await host.page.getByRole('button', { name: 'Rename', exact: true }).click()
  await host.page.getByRole('textbox', { name: 'File name', exact: true }).fill('renamed.tex')
  await host.page.getByRole('button', { name: 'Rename', exact: true }).click()
  await navigation(guest).getByRole('button', { name: 'renamed.tex', exact: true }).waitFor()
  await contains(guest, 'Shared chapter'); await insert(guest, 'EDIT AFTER RENAME ')
  await open(host, 'renamed.tex'); await contains(host, 'EDIT AFTER RENAME')
  host.page.once('dialog', dialog => dialog.accept())
  await host.page.getByLabel('Actions for renamed.tex').click()
  await host.page.getByRole('button', { name: 'Delete', exact: true }).click()
  await guest.page.waitForFunction(() => !document.querySelector('[aria-label="Project navigation"]')?.textContent?.includes('renamed.tex'))
  await contains(guest, '\\documentclass'); clean()
}, 60000)

test('ZIP backup contains exact shared text, nested files, and binary assets', async () => {
  const host = await client(), guest = await client(); const link = await start(host); await join(guest, link)
  const image = readFileSync(new URL('../fixtures/image.png', import.meta.url))
  await upload(host, 'image.png', image); await upload(host, 'résumé.tex', '中文 😀 résumé\n')
  await open(guest, 'résumé.tex'); await contains(guest, 'résumé')
  const pending = guest.page.waitForEvent('download')
  await guest.page.getByRole('button', { name: 'Download project', exact: true }).click()
  const download = await pending; const path = await download.path()
  const zip = unzipSync(readFileSync(path!))
  expect(strFromU8(zip['résumé.tex'])).toBe('中文 😀 résumé\n')
  expect(Buffer.from(zip['image.png'])).toEqual(image)
  expect(strFromU8(zip['main.tex'])).toContain('\\documentclass')
  clean()
}, 60000)

test('leaving restores the previous local draft and supporting files', async () => {
  const host = await client(); await host.page.goto(site.url + 'editor/'); await host.page.waitForSelector('article svg')
  await upload(host, 'private.tex', 'PRIVATE SUPPORT FILE')
  await host.page.waitForFunction(() => document.querySelector('.save-indicator')?.textContent === 'Saved locally')
  const before = await host.page.evaluate(() => localStorage.getItem('quicktex-document'))
  // start navigates once more, exercising ordinary local reload before sharing.
  await start(host); await insert(host, '% TEMPORARY ONLY\n')
  host.page.once('dialog', dialog => dialog.accept())
  await host.page.getByRole('button', { name: 'Leave', exact: true }).click()
  await host.page.waitForFunction(() => !location.hash && !!document.querySelector('.cm-content'))
  expect(await host.page.evaluate(() => localStorage.getItem('quicktex-document'))).toBe(before)
  expect(await host.page.locator('.cm-content').innerText()).not.toContain('TEMPORARY ONLY')
  await open(host, 'private.tex'); await contains(host, 'PRIVATE SUPPORT FILE'); clean()
}, 60000)

test('an empty room never seeds a starter document or touches private storage', async () => {
  const guest = await client(); await guest.page.goto(site.url)
  await guest.page.evaluate(() => localStorage.setItem('quicktex-document', 'SECRET DRAFT'))
  await guest.page.goto(site.url + 'editor/#session=' + 'c'.repeat(64))
  await guest.page.getByText('Waiting for someone with this session open…', { exact: false }).waitFor()
  expect(await guest.page.locator('.cm-content').count()).toBe(0)
  expect(await guest.page.evaluate(() => localStorage.getItem('quicktex-document'))).toBe('SECRET DRAFT')
  expect(await guest.page.evaluate(async () => (await indexedDB.databases()).map(db => db.name))).not.toContain('quicktex-workspace')
  expect(await guest.page.getByRole('button', { name: 'Download project', exact: true }).isDisabled()).toBe(true)
  clean()
}, 30000)

test('separate session links never receive each other’s document edits', async () => {
  const hostA = await client(), hostB = await client(), guestA = await client(), guestB = await client()
  const linkA = await start(hostA), linkB = await start(hostB)
  expect(linkA).not.toBe(linkB)
  await join(guestA, linkA); await join(guestB, linkB)
  await insert(hostA, '% ROOM A SECRET\n'); await insert(hostB, '% ROOM B SECRET\n')
  await contains(guestA, 'ROOM A SECRET'); await contains(guestB, 'ROOM B SECRET')
  expect(await guestA.page.locator('.cm-content').innerText()).not.toContain('ROOM B SECRET')
  expect(await guestB.page.locator('.cm-content').innerText()).not.toContain('ROOM A SECRET')
  clean()
}, 60000)

test('invalid manual invitations show an error without breaking the active session', async () => {
  const host = await client(); await start(host)
  await host.page.getByRole('button', { name: 'Share (1)', exact: true }).click()
  await host.page.locator('summary').filter({ hasText: 'Not connecting?' }).click()
  await host.page.getByRole('textbox', { name: 'Received connection code' }).fill('not a connection code')
  await host.page.getByRole('button', { name: 'Create reply code', exact: true }).click()
  await host.page.getByRole('dialog', { name: 'Share a temporary editing session' }).getByRole('alert').getByText('Invalid connection code.', { exact: false }).waitFor()
  await host.page.getByRole('button', { name: 'Close', exact: true }).click()
  await insert(host, '% STILL WORKS\n'); await contains(host, 'STILL WORKS'); clean()
}, 30000)
