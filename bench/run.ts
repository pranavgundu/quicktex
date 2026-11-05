import { spawnSync } from 'node:child_process'
import { cpus, totalmem } from 'node:os'
import { existsSync, mkdirSync, readFileSync, rmSync, statSync, writeFileSync } from 'node:fs'
import { documentPdf, init, renderDocument } from '../wasm-core/js/quicktex.js'
import { documents } from './documents'

type Tool = {
  name: string
  version: string[]
  command(document: string, passes: boolean): string
  output(document: string): string
}

type Result = { median: number; low: number; high: number; runs: number; memory: number; pages: number; bytes: number }
type Failure = { failed: string }

const root = new URL('.', import.meta.url).pathname
const work = `${root}.work`
const budget = Number(process.env.BENCH_SECONDS ?? 15)
const limit = 600
const clean = "sh -c 'rm -f *.aux *.toc *.log *.out *.pdf *.fls *.fdb_latexmk *.xdv'"
const wanted = process.argv.slice(2)
const texPath = `/Library/TeX/texbin:${process.env.PATH}`
const env = { ...process.env, PATH: texPath }

const engine = (name: string, flag: string): Tool => ({
  name,
  version: [name, '--version'],
  command: (document, passes) =>
    passes
      ? `latexmk ${flag} -interaction=batchmode -halt-on-error -silent ${document}.tex`
      : `${name} -interaction=batchmode -halt-on-error ${document}.tex`,
  output: (document) => `${document}.pdf`,
})

const tools: Tool[] = [
  {
    name: 'quicktex',
    version: ['git', '-C', root, 'rev-parse', '--short', 'HEAD'],
    command: (document) => `bun ${root}compile.ts ${document}.tex ${document}.pdf`,
    output: (document) => `${document}.pdf`,
  },
  engine('pdflatex', '-pdf'),
  engine('xelatex', '-xelatex'),
  engine('lualatex', '-lualatex'),
  {
    name: 'tectonic',
    version: ['tectonic', '--version'],
    command: (document) => `tectonic --only-cached --chatter minimal ${document}.tex`,
    output: (document) => `${document}.pdf`,
  },
]

function run(line: string, cwd: string) {
  const start = performance.now()
  const done = spawnSync('sh', ['-c', line], { cwd, env, timeout: limit * 1000, encoding: 'utf8' })
  return { seconds: (performance.now() - start) / 1000, ok: done.status === 0, output: `${done.stdout}${done.stderr}` }
}

function available(tool: Tool) {
  const done = spawnSync(tool.version[0], tool.version.slice(1), { env, encoding: 'utf8' })
  return done.status === 0 ? done.stdout.split('\n')[0].trim() : null
}

function pages(file: string) {
  const done = spawnSync('pdfinfo', [file], { encoding: 'utf8' })
  return Number(/Pages:\s+(\d+)/.exec(done.stdout)?.[1] ?? 0)
}

function measure(tool: Tool, name: string, passes: boolean): Result | Failure {
  const folder = `${work}/${tool.name}/${name}`
  rmSync(folder, { recursive: true, force: true })
  mkdirSync(folder, { recursive: true })
  writeFileSync(`${folder}/${name}.tex`, documents[name].tex())
  const line = tool.command(name, passes)
  if (tool.name === 'tectonic') run(line.replace(' --only-cached', ''), folder)
  const first = run(line, folder)
  const pdf = `${folder}/${tool.output(name)}`
  if (!first.ok || !existsSync(pdf)) return { failed: first.output.trim().split('\n').slice(-1)[0]?.slice(0, 80) ?? 'failed' }
  const count = Math.min(tool.name === 'quicktex' ? 60 : 15, Math.max(3, Math.ceil(budget / first.seconds)))
  const exported = `${folder}/hyperfine.json`
  const timed = spawnSync('hyperfine', ['-N', '--warmup', '1', '--runs', String(count), '--prepare', clean, '--export-json', exported, line], {
    cwd: folder,
    env,
    encoding: 'utf8',
  })
  if (timed.status !== 0) return { failed: timed.stderr.trim().split('\n').slice(-1)[0]?.slice(0, 80) ?? 'failed' }
  const { time_wall_clock: wall, memory_peak_resident: memory } = JSON.parse(readFileSync(exported, 'utf8')).results[0].summary
  run(line, folder)
  return {
    median: wall.median * 1000,
    low: wall.min * 1000,
    high: wall.max * 1000,
    runs: wall.count,
    memory: memory.median / 2 ** 20,
    pages: pages(pdf),
    bytes: statSync(pdf).size,
  }
}

async function warm() {
  await init(readFileSync(new URL('../wasm-core/js/quicktex.wasm', import.meta.url)))
  const out: Record<string, number> = {}
  for (const name of Object.keys(documents)) {
    if (wanted.length && !wanted.includes(name)) continue
    const tex = documents[name].tex()
    const times: number[] = []
    const end = performance.now() + 2000
    while (performance.now() < end || times.length < 5) {
      const start = performance.now()
      renderDocument(tex)
      documentPdf()
      times.push(performance.now() - start)
    }
    times.sort((a, b) => a - b)
    out[name] = times[times.length >> 1]
  }
  return out
}

const seen = tools.flatMap((tool) => {
  const version = available(tool)
  if (!version) console.log(`skipping ${tool.name}, not installed`)
  return version ? [{ tool, version }] : []
})

const results: Record<string, Record<string, Result | Failure>> = {}
for (const name of Object.keys(documents)) {
  if (wanted.length && !wanted.includes(name)) continue
  results[name] = {}
  for (const { tool } of seen) {
    const result = measure(tool, name, documents[name].passes)
    results[name][tool.name] = result
    console.log(name.padEnd(10), tool.name.padEnd(10), 'failed' in result ? `failed: ${result.failed}` : `${result.median.toFixed(0)} ms, ${result.pages} pages, ${result.runs} runs`)
  }
}

const previous = wanted.length && existsSync(`${root}results.json`) ? JSON.parse(readFileSync(`${root}results.json`, 'utf8')) : null

const math = spawnSync('node', [`${root}math.mjs`], { encoding: 'utf8', maxBuffer: 1 << 24 })
if (math.status !== 0) throw new Error(math.stderr)

writeFileSync(
  `${root}results.json`,
  `${JSON.stringify(
    {
      machine: { cpu: cpus()[0].model, cores: cpus().length, memory: Math.round(totalmem() / 2 ** 30), platform: `${process.platform} ${process.arch}` },
      date: new Date().toISOString().slice(0, 10),
      versions: Object.fromEntries(seen.map(({ tool, version }) => [tool.name, version])),
      documents: Object.fromEntries(Object.entries(documents).map(([name, document]) => [name, { about: document.about, passes: document.passes }])),
      results: { ...previous?.results, ...results },
      warm: { ...previous?.warm, ...(await warm()) },
      math: JSON.parse(math.stdout),
    },
    null,
    1,
  )}\n`,
)

spawnSync('bun', [`${root}report.ts`], { stdio: 'inherit' })
