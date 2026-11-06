import { readFileSync, writeFileSync } from 'node:fs'
import { spawnSync } from 'node:child_process'
import { createHash } from 'node:crypto'
import { documents } from './documents'

// Each measurement gets a fresh process. Process startup itself is excluded;
// initialization and the first layout + PDF are measured separately.
const args = process.argv.slice(2)
if (args[0] === '--sample') {
  const [, path, name] = args
  const document = documents[name as keyof typeof documents]
  if (!document) throw new Error(`Unknown benchmark document: ${name}`)
  const source = document.tex()
  const bytes = readFileSync(path)
  const engine = await import('../wasm-core/js/quicktex.js')
  const start = performance.now()
  await engine.init(bytes)
  const initialized = performance.now()
  const pages = engine.renderDocument(source)
  const pdf = engine.documentPdf()
  const end = performance.now()
  console.log(JSON.stringify({
    init_ms: initialized - start, compile_ms: end - initialized, total_ms: end - start,
    pages, bytes: pdf.length, warnings: engine.documentWarnings().length,
    pdf_sha256: createHash('sha256').update(pdf).digest('hex'),
  }))
} else {
  const [before, after = 'wasm-core/js/quicktex.wasm', output = 'bench/cold-comparison.json'] = args
  if (!before) throw new Error('Usage: bun bench/compare-cold.ts BASELINE.wasm [CANDIDATE.wasm] [OUTPUT.json]')
  const tolerance = Number(process.env.BENCH_TOLERANCE ?? 0.05)
  if (!Number.isFinite(tolerance) || tolerance < 0) throw new Error('BENCH_TOLERANCE must be a nonnegative finite fraction')
  const median = (values: number[]) => [...values].sort((a, b) => a - b)[values.length >> 1]
  type Sample = { init_ms: number; compile_ms: number; total_ms: number; pages: number; bytes: number; warnings: number; pdf_sha256: string }
  const results: Record<string, unknown> = {}
  let passed = true
  for (const name of ['note', 'paper', 'book']) {
    const samples: [Sample[], Sample[]] = [[], []]
    for (let round = 0; round < 21; round++) {
      for (const index of round % 2 ? [1, 0] : [0, 1]) {
        const run = spawnSync(process.execPath, [import.meta.path, '--sample', index ? after : before, name], { encoding: 'utf8', timeout: 120000 })
        if (run.error) throw run.error
        if (run.status !== 0) throw new Error(`Cold sample failed: ${run.stderr}`)
        samples[index].push(JSON.parse(run.stdout))
      }
    }
    const medians = samples.map(batch => ({
      init_ms: median(batch.map(s => s.init_ms)),
      compile_ms: median(batch.map(s => s.compile_ms)),
      total_ms: median(batch.map(s => s.total_ms)),
    }))
    const outputChanged = samples.flat().some(s => s.pdf_sha256 !== samples[0][0].pdf_sha256 || s.pages !== samples[0][0].pages || s.warnings !== 0)
    const change = medians[1].total_ms / medians[0].total_ms - 1
    const regression = outputChanged || change > tolerance
    passed &&= !regression
    results[name] = { before: medians[0], after: medians[1], change_percent: 100 * change, outputChanged, regression, samples }
    console.log(`${name.padEnd(10)} ${medians[0].total_ms.toFixed(3)} → ${medians[1].total_ms.toFixed(3)} ms (${(100 * change).toFixed(1)}%)${regression ? ' REGRESSION' : ''}`)
  }
  const identity = (path: string) => ({ path, sha256: createHash('sha256').update(readFileSync(path)).digest('hex') })
  writeFileSync(output, JSON.stringify({ description: '21 alternating fresh-process pairs; initialization plus first PDF, excluding process startup.', before: identity(before), after: identity(after), tolerance, results, passed }, null, 2) + '\n')
  if (!passed) process.exitCode = 1
}
