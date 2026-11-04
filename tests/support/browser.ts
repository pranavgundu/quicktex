import { chromium, type Browser, type BrowserContext, type Page } from 'playwright-core'
import { existsSync, readFileSync } from 'node:fs'
import { extname, join } from 'node:path'

const types: Record<string, string> = {
  '.html': 'text/html',
  '.js': 'text/javascript',
  '.css': 'text/css',
  '.wasm': 'application/wasm',
  '.svg': 'image/svg+xml',
}

export const dist = new URL('../../dist', import.meta.url).pathname

export function serve(root: string, prefix = '/quicktex/') {
  const server = Bun.serve({
    port: 0,
    fetch(request) {
      const path = decodeURIComponent(new URL(request.url).pathname)
      if (!path.startsWith(prefix)) return new Response('', { status: 404 })
      let file = join(root, path.slice(prefix.length))
      if (path.endsWith('/')) file = join(file, 'index.html')
      if (!existsSync(file)) return new Response('', { status: 404 })
      return new Response(readFileSync(file), { headers: { 'content-type': types[extname(file)] ?? 'application/octet-stream' } })
    },
  })
  return { url: `http://localhost:${server.port}${prefix}`, stop: () => server.stop(true) }
}

export function launch() {
  return chromium.launch({ channel: 'chrome', headless: true })
}

export type Session = { context: BrowserContext; page: Page; problems: string[] }

export async function session(browser: Browser, options: Parameters<Browser['newContext']>[0] = {}): Promise<Session> {
  const context = await browser.newContext({ viewport: { width: 1400, height: 900 }, colorScheme: 'light', acceptDownloads: true, ...options })
  const page = await context.newPage()
  const problems: string[] = []
  page.on('console', (message) => {
    if (message.type() === 'error' || message.type() === 'warning') problems.push(message.text())
  })
  page.on('pageerror', (error) => problems.push(error.message))
  return { context, page, problems }
}
