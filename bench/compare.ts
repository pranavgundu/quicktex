import { readFileSync, writeFileSync } from 'node:fs'
import { resolve } from 'node:path'
import { createHash } from 'node:crypto'
import { documents } from './documents'
import { corpus } from '../tests/support/corpus'

// Compare two builds in alternating batches on the same process and machine.
// Supply the old WASM explicitly; never infer a baseline from historical timings.
const [baselinePath, candidatePath = 'wasm-core/js/quicktex.wasm', outputPath = 'bench/compatibility-comparison.json'] = process.argv.slice(2)
if (!baselinePath) throw new Error('Usage: bun bench/compare.ts BASELINE.wasm [CANDIDATE.wasm] [OUTPUT.json]')
const tolerance = Number(process.env.BENCH_TOLERANCE ?? 0.05)
if (!Number.isFinite(tolerance) || tolerance < 0) throw new Error('BENCH_TOLERANCE must be a nonnegative finite fraction')
const before = await import('../wasm-core/js/quicktex.js?comparison=baseline')
const after = await import('../wasm-core/js/quicktex.js?comparison=candidate')
const baselineBytes = readFileSync(baselinePath)
const candidateBytes = readFileSync(candidatePath)
await before.init(baselineBytes)
await after.init(candidateBytes)
const median = (values: number[]) => [...values].sort((a, b) => a - b)[values.length >> 1]
const results: Record<string, unknown> = {}
let failed = false
for (const [name, document] of Object.entries(documents)) {
  const source = document.tex()
  const run = (engine: typeof before) => {
    const pages = engine.renderDocument(source)
    const pdf = engine.documentPdf()
    return { pages, bytes: pdf.length, warnings: engine.documentWarnings().length }
  }
  const oldOutput = run(before)
  const newOutput = run(after)
  const oldPdfHash = createHash('sha256').update(before.documentPdf()).digest('hex')
  const newPdfHash = createHash('sha256').update(after.documentPdf()).digest('hex')
  for (let i = 0; i < 10; i++) { run(before); run(after) }
  const pilot = performance.now()
  run(before)
  const batch = Math.min(500, Math.max(1, Math.ceil(20 / Math.max(0.01, performance.now() - pilot))))
  const samples = [[], []] as [number[], number[]]
  for (let round = 0; round < 21; round++) {
    for (const index of round % 2 ? [1, 0] : [0, 1]) {
      const start = performance.now()
      for (let i = 0; i < batch; i++) run(index ? after : before)
      samples[index].push((performance.now() - start) / batch)
    }
  }
  const oldMs = median(samples[0])
  const newMs = median(samples[1])
  const change = newMs / oldMs - 1
  const outputChanged = oldOutput.pages !== newOutput.pages || oldOutput.bytes !== newOutput.bytes || oldPdfHash !== newPdfHash
  const regression = change > tolerance || newOutput.warnings > 0 || outputChanged
  failed ||= regression
  results[name] = { before: { ...oldOutput, pdf_sha256: oldPdfHash, warm_ms: oldMs }, after: { ...newOutput, pdf_sha256: newPdfHash, warm_ms: newMs }, change_percent: Number((100 * change).toFixed(2)), batch, samples_ms: { before: samples[0], after: samples[1] }, regression }
  console.log(`${name.padEnd(10)} ${oldMs.toFixed(3)} → ${newMs.toFixed(3)} ms (${(change * 100).toFixed(1)}%)${regression ? ' REGRESSION' : ''}`)
}
// Formula rendering also pays parser-reset costs; document-only timings can hide
// regressions in this much shorter path.
const formulaRun = (engine: typeof before) => {
  let bytes = 0
  for (const source of corpus) bytes += engine.render(source, true).length
  return bytes
}
const oldFormulaBytes = formulaRun(before)
const newFormulaBytes = formulaRun(after)
const formulaFingerprint = (engine: typeof before) => {
  const hash = createHash('sha256')
  for (const source of corpus) hash.update(engine.render(source, true))
  return hash.digest('hex')
}
const oldFormulaHash = formulaFingerprint(before)
const newFormulaHash = formulaFingerprint(after)
for (let i = 0; i < 10; i++) { formulaRun(before); formulaRun(after) }
const formulaSamples = [[], []] as [number[], number[]]
for (let round = 0; round < 21; round++) {
  for (const index of round % 2 ? [1, 0] : [0, 1]) {
    const start = performance.now()
    for (let i = 0; i < 50; i++) formulaRun(index ? after : before)
    formulaSamples[index].push((performance.now() - start) / (50 * corpus.length))
  }
}
const oldFormulaMs = median(formulaSamples[0])
const newFormulaMs = median(formulaSamples[1])
const formulaChange = newFormulaMs / oldFormulaMs - 1
const formulaRegression = formulaChange > tolerance || oldFormulaBytes !== newFormulaBytes || oldFormulaHash !== newFormulaHash
failed ||= formulaRegression
results.formula_corpus = { before: { warm_ms: oldFormulaMs, svg_bytes: oldFormulaBytes, svg_sha256: oldFormulaHash }, after: { warm_ms: newFormulaMs, svg_bytes: newFormulaBytes, svg_sha256: newFormulaHash }, change_percent: Number((100 * formulaChange).toFixed(2)), formulas: corpus.length, samples_ms: { before: formulaSamples[0], after: formulaSamples[1] }, regression: formulaRegression }
console.log(`formula    ${(oldFormulaMs * 1000).toFixed(2)} → ${(newFormulaMs * 1000).toFixed(2)} µs (${(formulaChange * 100).toFixed(1)}%)${formulaRegression ? ' REGRESSION' : ''}`)
writeFileSync(outputPath, JSON.stringify({ description: 'Alternating same-process layout + PDF comparison. Existing benchmark documents must keep page counts, identical PDF bytes, and zero warnings; formula SVG output must also be identical. Timings allow the stated noise tolerance.', baseline: { path: resolve(baselinePath), sha256: createHash('sha256').update(baselineBytes).digest('hex') }, candidate: { path: resolve(candidatePath), sha256: createHash('sha256').update(candidateBytes).digest('hex') }, date: new Date().toISOString(), tolerance_percent: tolerance * 100, passed: !failed, documents: results }, null, 2) + '\n')
if (failed) process.exitCode = 1
