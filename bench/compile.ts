import { readFileSync, writeFileSync } from 'node:fs'
import { documentPdf, documentWarnings, init, renderDocument } from '../wasm-core/js/quicktex.js'

const [input, output] = process.argv.slice(2)

await init(readFileSync(new URL('../wasm-core/js/quicktex.wasm', import.meta.url)))
renderDocument(readFileSync(input, 'utf8'))
writeFileSync(output, documentPdf())

const skipped = documentWarnings().length
if (skipped > 0) {
  console.error(`quicktex skipped ${skipped} commands in ${input}`)
  process.exit(1)
}
