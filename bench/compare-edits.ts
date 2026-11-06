import { readFileSync, writeFileSync } from 'node:fs'
import { createHash } from 'node:crypto'
import { deflateSync } from 'node:zlib'
import { documents } from './documents'

// Real edits always run layout. Resources are uploaded once per project, as in
// the editor. PDF equality is checked outside the compile-only timing samples.
const [baseline, candidate = 'wasm-core/js/quicktex.wasm', output = 'bench/edit-comparison.json'] = process.argv.slice(2)
if (!baseline) throw new Error('Usage: bun bench/compare-edits.ts BASELINE.wasm [CANDIDATE.wasm] [OUTPUT.json]')
const tolerance = Number(process.env.BENCH_TOLERANCE ?? 0.05)
if (!Number.isFinite(tolerance) || tolerance < 0) throw new Error('BENCH_TOLERANCE must be a nonnegative finite fraction')
type Engine = typeof import('../wasm-core/js/quicktex.js')
const load = (name: string): Promise<Engine> => import(`../wasm-core/js/quicktex.js?edits=${name}`)
const engines = [await load('before'), await load('after')]
await engines[0].init(readFileSync(baseline))
await engines[1].init(readFileSync(candidate))
const median = (values: number[]) => [...values].sort((a, b) => a - b)[values.length >> 1]
const hash = (bytes: Uint8Array) => createHash('sha256').update(bytes).digest('hex')
function png() {
  const width = 768, height = 512
  const raw = Buffer.alloc(height * (width * 4 + 1))
  for (let y = 0; y < height; y++) for (let x = 0; x < width; x++) {
    const at = y * (width * 4 + 1) + 1 + x * 4
    raw[at] = x % 256; raw[at + 1] = y % 256; raw[at + 2] = (x + y) % 256; raw[at + 3] = 180
  }
  const chunk = (name: string, data: Buffer) => {
    const tag = Buffer.from(name), payload = Buffer.concat([tag, data])
    let crc = 0xffffffff
    for (const byte of payload) {
      crc ^= byte
      for (let bit = 0; bit < 8; bit++) crc = (crc >>> 1) ^ (crc & 1 ? 0xedb88320 : 0)
    }
    const size = Buffer.alloc(4), checksum = Buffer.alloc(4)
    size.writeUInt32BE(data.length); checksum.writeUInt32BE((crc ^ 0xffffffff) >>> 0)
    return Buffer.concat([size, payload, checksum])
  }
  const ihdr = Buffer.alloc(13)
  ihdr.writeUInt32BE(width); ihdr.writeUInt32BE(height, 4); ihdr[8] = 8; ihdr[9] = 6
  return Buffer.concat([Buffer.from([137,80,78,71,13,10,26,10]), chunk('IHDR', ihdr), chunk('IDAT', deflateSync(raw)), chunk('IEND', Buffer.alloc(0))])
}
const article = (body: string) => `\\documentclass[11pt]{article}\\begin{document}${body}\\end{document}`
const scenarios: { name: string; source: string; files: import('../wasm-core/js/quicktex.js').ProjectFiles }[] = [
  ...['note', 'paper', 'book'].map(name => ({ name, source: documents[name].tex(), files: {} })),
  { name: 'image-project', source: article(String.raw`\includegraphics[width=8cm]{plot.png}\par Text.`), files: { 'plot.png': png() } },
  { name: 'bibliography', source: article(String.raw`\nocite{*}\bibliographystyle{unsrt}\bibliography{refs}`), files: {
    'refs.bib': Array.from({ length: 300 }, (_, i) => `@article{key${i},author={Jane Doe},title={Reference number ${i}},journal={Journal},year=2024}`).join('\n'),
  } },
]
const results: Record<string, unknown> = {}
let passed = true
for (const scenario of scenarios) {
  const sources = Array.from({ length: 8 }, (_, i) => scenario.source.replace('\\end{document}', `\nEdit number ${i}.\\end{document}`))
  for (const engine of engines) engine.setFiles(scenario.files)
  let outputChanged = false
  for (const source of sources) {
    const fingerprints = engines.map(engine => {
      const pages = engine.renderDocument(source)
      return { pages, warnings: engine.documentWarnings(), hash: hash(engine.documentPdf()) }
    })
    outputChanged ||= JSON.stringify(fingerprints[0]) !== JSON.stringify(fingerprints[1]) || fingerprints[1].warnings.length > 0
  }
  const pilot = performance.now()
  engines[0].renderDocument(sources[0])
  const batch = Math.min(500, Math.max(1, Math.ceil(20 / Math.max(0.01, performance.now() - pilot))))
  const samples: [number[], number[]] = [[], []]
  for (let round = 0; round < 21; round++) for (const index of round % 2 ? [1, 0] : [0, 1]) {
    const start = performance.now()
    for (let edit = 0; edit < batch; edit++) engines[index].renderDocument(sources[(round * batch + edit) % sources.length])
    samples[index].push((performance.now() - start) / batch)
  }
  const before = median(samples[0]), after = median(samples[1]), change = after / before - 1
  const regression = outputChanged || change > tolerance
  passed &&= !regression
  results[scenario.name] = { before_ms: before, after_ms: after, change_percent: 100 * change, outputChanged, regression, batch, samples_ms: samples }
  console.log(`${scenario.name.padEnd(15)} ${before.toFixed(3)} → ${after.toFixed(3)} ms (${(change * 100).toFixed(1)}%)${regression ? ' REGRESSION' : ''}`)
}
writeFileSync(output, JSON.stringify({ description: 'Compile-only source-changing edits with stable project assets; eight PDF fingerprints checked per scenario.', before: { path: baseline, sha256: hash(readFileSync(baseline)) }, after: { path: candidate, sha256: hash(readFileSync(candidate)) }, tolerance, results, passed }, null, 2) + '\n')
if (!passed) process.exitCode = 1
