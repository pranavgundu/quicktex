import { readFileSync, readdirSync, writeFileSync } from 'node:fs'
import { join, resolve, relative } from 'node:path'
import { init, loadFontsFor, renderDocument, documentPdf, documentWarnings, QuicktexError } from '../wasm-core/js/quicktex.js'

// One process per project isolates traps, memory growth and project state.
const [input, root, output] = process.argv.slice(2).map(p => resolve(p))
const files: Record<string, string | Uint8Array> = {}
function collect(directory: string) {
  for (const entry of readdirSync(directory, { withFileTypes: true })) {
    if (entry.name.startsWith('.') || ['node_modules', 'target'].includes(entry.name)) continue
    const path = join(directory, entry.name)
    if (entry.isDirectory()) collect(path)
    else if (/\.(tex|bib|cls|sty|png|jpe?g|pdf|svg)$/i.test(path)) {
      files[relative(process.cwd(), path)] = /\.(tex|bib|cls|sty)$/i.test(path) ? readFileSync(path, 'utf8') : readFileSync(path)
    }
  }
}
collect(root)
const source = readFileSync(input, 'utf8')
await init(readFileSync(new URL('../wasm-core/js/quicktex.wasm', import.meta.url)))
await loadFontsFor(source + Object.values(files).filter(v => typeof v === 'string').join('\n'), readFileSync)
const start = performance.now()
try {
  const pages = renderDocument(source, { files })
  writeFileSync(output, documentPdf())
  console.log(JSON.stringify({ pages, warnings: documentWarnings(), compileMs: performance.now() - start }))
} catch (error) {
  console.log(JSON.stringify({ error: String(error), position: error instanceof QuicktexError ? error.position : undefined, warnings: documentWarnings(), compileMs: performance.now() - start }))
  process.exitCode = 1
}
