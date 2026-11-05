import { readFileSync, writeFileSync } from 'node:fs'

type Result = { median?: number; memory?: number; pages?: number; bytes?: number; failed?: string }

const root = new URL('.', import.meta.url).pathname
const data = JSON.parse(readFileSync(`${root}results.json`, 'utf8'))
const tools = Object.keys(data.versions)
const others = tools.filter((tool) => tool !== 'quicktex')
const names = Object.keys(data.results)

const time = (ms: number) => (ms < 1 ? `${(ms * 1000).toFixed(0)} µs` : ms >= 1000 ? `${(ms / 1000).toFixed(1)} s` : `${ms < 10 ? ms.toFixed(1) : ms.toFixed(0)} ms`)
const times = (value: number) => `${value < 10 ? value.toFixed(1) : value.toFixed(0)}x`

function cell(name: string, tool: string) {
  const result: Result = data.results[name][tool]
  return result.median === undefined ? 'failed' : time(result.median)
}

function ratio(name: string, tool: string) {
  const own: Result = data.results[name].quicktex
  const other: Result = data.results[name][tool]
  return own.median !== undefined && other.median !== undefined ? other.median / own.median : null
}

const compiled = names.map((name) => {
  const values = others.flatMap((tool) => ratio(name, tool) ?? [])
  return `| ${name} | ${tools.map((tool) => cell(name, tool)).join(' | ')} | ${values.length ? times(Math.min(...values)) : 'n/a'} |`
})

const speedups = names.map((name) => `| ${name} | ${others.map((tool) => { const value = ratio(name, tool); return value === null ? 'n/a' : times(value) }).join(' | ')} |`)

const pages = names.map((name) => `| ${name} | ${tools.map((tool) => { const result: Result = data.results[name][tool]; return result.pages ?? 'n/a' }).join(' | ')} |`)

const memory = names.map((name) => `| ${name} | ${tools.map((tool) => { const result: Result = data.results[name][tool]; return result.memory === undefined ? 'n/a' : `${result.memory.toFixed(0)} MB` }).join(' | ')} |`)

const size = (bytes: number) => (bytes >= 2 ** 20 ? `${(bytes / 2 ** 20).toFixed(1)} MB` : `${Math.round(bytes / 1024)} KB`)
const sizes = names.map((name) => `| ${name} | ${tools.map((tool) => { const result: Result = data.results[name][tool]; return result.bytes === undefined ? 'n/a' : size(result.bytes) }).join(' | ')} |`)

const warm = names.map((name) => {
  const cold: Result = data.results[name].quicktex
  return `| ${name} | ${time(data.warm[name])} | ${cold.median === undefined ? 'n/a' : time(cold.median)} |`
})

const engines = data.math.engines
const math = Object.keys(engines)
  .sort((a, b) => engines[a].micros - engines[b].micros)
  .map((name) => {
    const engine = engines[name]
    return `| ${name} | ${engine.micros.toFixed(1)} µs | ${times(engine.micros / engines.quicktex.micros)} | ${engine.output} | ${(engine.bytes / 1024).toFixed(1)} KB | ${engine.version} |`
  })

const row = (cells: string[]) => `| ${cells.join(' | ')} |`
const rule = (count: number) => `|${' --- |'.repeat(count)}`

const text = `# Results

Run on ${data.date}. ${data.machine.cpu}, ${data.machine.cores} cores, ${data.machine.memory} GB, ${data.machine.platform}. Regenerate with \`bun run bench\`.

## Source to PDF on disk

Median wall time from a fresh process to a finished PDF. Lower is better. The last column is the speedup over whichever other compiler was fastest on that document.

${row(['Document', ...tools, 'vs fastest other'])}
${rule(tools.length + 2)}
${compiled.join('\n')}

## How many times faster quicktex is

${row(['Document', ...others])}
${rule(others.length + 1)}
${speedups.join('\n')}

## Peak memory

${row(['Document', ...tools])}
${rule(tools.length + 1)}
${memory.join('\n')}

## PDF size

quicktex embeds reusable Type 3 glyph fonts with Unicode maps and compresses page streams. TeX uses different font formats; compare the resulting sizes below.

${row(['Document', ...tools])}
${rule(tools.length + 1)}
${sizes.join('\n')}

## Page counts

Layout is not identical, so page counts can differ. See [the README](README.md) for what quicktex does not do.

${row(['Document', ...tools])}
${rule(tools.length + 1)}
${pages.join('\n')}

## quicktex already loaded

This is what an open editor tab pays per compile. No other compiler here has a mode like this, so there is no column to compare it with.

${row(['Document', 'Already loaded', 'Fresh process'])}
${rule(3)}
${warm.join('\n')}

## One formula

Mean time per formula over the ${data.math.formulas} formulas that every library accepted, after warming up, in Node. Output formats differ, and none of this includes the browser drawing the result.

${row(['Library', 'Per formula', 'vs quicktex', 'Output', 'Mean output', 'Version'])}
${rule(6)}
${math.join('\n')}

## Versions

${Object.entries(data.versions).map(([tool, version]) => `- ${tool}: ${version}`).join('\n')}
`

writeFileSync(`${root}results.md`, text)
console.log(text)
