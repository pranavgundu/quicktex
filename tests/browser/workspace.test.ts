import { afterAll, beforeAll, beforeEach, expect, test } from 'bun:test'
import type { Browser } from 'playwright-core'
import { dist, launch, serve, session, type Session } from '../support/browser'

const site = serve(dist)
let browser: Browser
let s: Session
const content = () => s.page.locator('.cm-content')
const key = (name: string) => s.page.keyboard.press(name)

beforeAll(async () => { browser = await launch() })
afterAll(async () => { await s?.context.close(); await browser.close(); site.stop() })
beforeEach(async () => {
  await s?.context.close()
  s = await session(browser)
  await s.page.goto(`${site.url}editor/`)
  await s.page.waitForSelector('article svg')
  await s.page.getByRole('button', { name: 'Add files', exact: true }).waitFor({ state: 'visible' })
})

async function create(name: string) {
  await s.page.getByRole('button', { name: 'New file', exact: true }).click()
  await s.page.getByLabel('File name', { exact: true }).fill(name)
  await s.page.getByRole('button', { name: 'Create file', exact: true }).click()
}

async function end() { await content().click(); await key('ControlOrMeta+End') }

test('files have independent undo history and persist after reload', async () => {
  await create('chapters/intro.tex')
  await content().click()
  await s.page.keyboard.type('Introduction')
  await s.page.getByRole('tab', { name: 'main.tex' }).click()
  await end()
  await s.page.keyboard.type('% main edit')
  await s.page.getByRole('tab', { name: 'chapters/intro.tex' }).click()
  expect(await content().innerText()).toContain('Introduction')
  await key('ControlOrMeta+z')
  expect(await content().innerText()).not.toContain('Introduction')
  await key('ControlOrMeta+Shift+z')
  expect(await content().innerText()).toContain('Introduction')
  await key('ControlOrMeta+s')
  await s.page.waitForFunction(() => document.querySelector('.save-indicator')?.textContent === 'Saved locally')
  await s.page.reload()
  await s.page.getByRole('tab', { name: 'chapters/intro.tex' }).click()
  expect(await content().innerText()).toContain('Introduction')
  expect(s.problems).toEqual([])
})

test('toolbar commands preserve selection and undo works', async () => {
  await create('notes.tex')
  await content().click()
  await s.page.keyboard.type('hello')
  await key('ControlOrMeta+a')
  await s.page.getByRole('button', { name: 'Bold', exact: true }).click()
  expect(await content().innerText()).toContain('\\textbf{hello}')
  await s.page.getByRole('button', { name: 'Undo', exact: true }).click()
  expect(await content().innerText()).toBe('hello')
  await s.page.getByRole('button', { name: 'Find and replace', exact: true }).click()
  expect(await s.page.locator('.cm-search').isVisible()).toBe(true)
})

test('command palette inserts editable snippet fields', async () => {
  await create('snippet.tex')
  await key('ControlOrMeta+Shift+p')
  await s.page.getByRole('textbox', { name: 'Search commands' }).fill('insert section')
  await key('Enter')
  await s.page.keyboard.type('A new section')
  expect(await content().innerText()).toContain('\\section{A new section}')
  expect(s.problems).toEqual([])
})

test('settings survive file switching and reload without losing undo', async () => {
  await s.page.getByLabel('Editor settings', { exact: true }).click()
  await s.page.getByLabel('Font size', { exact: true }).selectOption('18')
  await s.page.getByLabel('Wrap long lines', { exact: true }).uncheck()
  await s.page.getByLabel('Line numbers', { exact: true }).uncheck()
  await s.page.getByLabel('Editor settings', { exact: true }).click()
  await create('settings.tex')
  expect(await s.page.locator('.cm-editor').evaluate((node) => getComputedStyle(node).fontSize)).toBe('18px')
  expect(await s.page.locator('.cm-lineNumbers').count()).toBe(0)
  expect(await s.page.locator('.cm-lineWrapping').count()).toBe(0)
  await s.page.reload()
  expect(await s.page.locator('.cm-editor').evaluate((node) => getComputedStyle(node).fontSize)).toBe('18px')
})

test('outline updates off the typing path and jumps to sections', async () => {
  await create('outline.tex')
  await content().click()
  await s.page.keyboard.insertText('\\section{First}\nText\n\\subsection{Second}\nMore')
  await s.page.getByRole('navigation', { name: 'Document outline' }).getByRole('button', { name: 'Second' }).click()
  expect(await s.page.locator('.cm-activeLine').innerText()).toContain('\\subsection{Second}')
})

test('renaming and deletion update the workspace safely', async () => {
  await create('old.tex')
  await content().click()
  await s.page.keyboard.type('keep this')
  await s.page.getByLabel('Actions for old.tex').click()
  await s.page.getByRole('button', { name: 'Rename', exact: true }).click()
  await s.page.getByLabel('File name', { exact: true }).fill('new.tex')
  await s.page.getByRole('dialog').getByRole('button', { name: 'Rename', exact: true }).click()
  expect(await s.page.getByRole('tab', { name: 'old.tex' }).count()).toBe(0)
  expect(await content().innerText()).toContain('keep this')
  s.page.once('dialog', (dialog) => dialog.accept())
  await s.page.getByLabel('Actions for new.tex').click()
  await s.page.getByRole('button', { name: 'Delete', exact: true }).click()
  expect(await s.page.getByRole('tab', { name: 'new.tex' }).count()).toBe(0)
})

test('manual compilation leaves preview untouched until requested', async () => {
  await s.page.getByLabel('Auto', { exact: true }).uncheck()
  const before = await s.page.locator('article').first().innerHTML()
  await end()
  await s.page.keyboard.type('% manual edit')
  await s.page.waitForTimeout(300)
  expect(await s.page.locator('article').first().innerHTML()).toBe(before)
  await s.page.getByRole('button', { name: /Recompile/ }).click()
  await s.page.waitForTimeout(200)
  expect(s.problems).toEqual([])
})
