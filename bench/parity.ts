import { spawnSync } from 'node:child_process'
import { mkdirSync, readFileSync, rmSync, writeFileSync } from 'node:fs'
import { fileURLToPath } from 'node:url'
import { join } from 'node:path'
import { documentPdf, documentWarnings, init, renderDocument } from '../wasm-core/js/quicktex.js'
import { documents } from './documents'

// Run independently of speed benchmarks: matching page counts is required,
// but is not proof that line breaks or the rendered content match.
const work = fileURLToPath(new URL('./.work/parity/', import.meta.url))
const selected = process.argv.slice(2)
const names = selected.length ? selected : Object.keys(documents)
const env = { ...process.env, PATH: `/Library/TeX/texbin:${process.env.PATH ?? ''}` }
const results: { document: string; quicktex: number; pdflatex: number; warnings: number; matches: boolean }[] = []

function run(command: string, args: string[], cwd: string) {
  const result = spawnSync(command, args, { cwd, env, encoding: 'utf8', timeout: 120_000 })
  if (result.error) throw result.error
  if (result.status !== 0) throw new Error(`${command} failed in ${cwd}\n${result.stdout}\n${result.stderr}`)
  return result.stdout
}

await init(readFileSync(new URL('../wasm-core/js/quicktex.wasm', import.meta.url)))
for (const name of names) {
  const document = documents[name]
  if (!document) throw new Error(`Unknown document: ${name}`)
  const directory = join(work, name)
  rmSync(directory, { recursive: true, force: true })
  mkdirSync(directory, { recursive: true })
  const source = document.tex()
  writeFileSync(join(directory, 'document.tex'), source)
  // Three cleanly ordered passes resolve forward references and the TOC.
  for (let pass = 0; pass < (document.passes ? 3 : 1); pass++) {
    run('pdflatex', ['-interaction=batchmode', '-halt-on-error', 'document.tex'], directory)
  }
  const info = run('pdfinfo', ['document.pdf'], directory)
  const pdflatex = Number(/Pages:\s+(\d+)/.exec(info)?.[1])
  if (!pdflatex) throw new Error(`Cannot read pdflatex page count for ${name}`)
  const quicktex = renderDocument(source)
  const warnings = documentWarnings().length
  writeFileSync(join(directory, 'quicktex.pdf'), documentPdf())
  const matches = quicktex === pdflatex && warnings === 0
  results.push({ document: name, quicktex, pdflatex, warnings, matches })
  console.log(`${matches ? 'PASS' : 'FAIL'} ${name}: QuickTeX ${quicktex}, pdflatex ${pdflatex}, warnings ${warnings}`)
}
writeFileSync(join(work, 'results.json'), JSON.stringify(results, null, 2) + '\n')
console.log(`Comparison PDFs and results: ${work}`)
if (results.some((result) => !result.matches)) process.exitCode = 1
