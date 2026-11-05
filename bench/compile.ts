import { readFileSync, writeFileSync, readdirSync } from 'node:fs'
import { dirname, join } from 'node:path'
import { documentPdf, loadFontsFor, documentWarnings, init, renderDocument } from '../wasm-core/js/quicktex.js'

const [input, output] = process.argv.slice(2)

await init(readFileSync(new URL('../wasm-core/js/quicktex.wasm', import.meta.url)))
const source = readFileSync(input, 'utf8')
const files: Record<string, string | Uint8Array> = {}
if (/\\(?:input|include|includegraphics)\b/.test(source)) {
  const collect = (directory: string, prefix = '') => {
    for (const entry of readdirSync(directory, { withFileTypes: true })) {
      if (entry.name.startsWith('.') || entry.name === 'node_modules' || entry.name === 'target') continue
      const name = prefix + entry.name
      if (entry.isDirectory()) collect(join(directory, entry.name), name + '/')
      else if (/\.(tex|png|jpe?g)$/i.test(name)) files[name] = /\.tex$/i.test(name) ? readFileSync(join(directory, entry.name), 'utf8') : readFileSync(join(directory, entry.name))
    }
  }
  collect(dirname(input))
}
await loadFontsFor(source + Object.values(files).filter((v) => typeof v === 'string').join(' '), readFileSync)
renderDocument(source, Object.keys(files).length ? { files } : undefined)
writeFileSync(output, documentPdf())

const skipped = documentWarnings().length
if (skipped > 0) {
  console.error(`quicktex skipped ${skipped} commands in ${input}`)
  process.exit(1)
}
