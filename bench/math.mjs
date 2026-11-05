import { readFileSync } from 'node:fs'
import { createRequire } from 'node:module'
import katex from 'katex'
import temml from 'temml'
import { init, render } from '../wasm-core/js/quicktex.js'
import { corpus } from '../tests/support/corpus.ts'

const require = createRequire(import.meta.url)
const version = (name) => JSON.parse(readFileSync(new URL(`./node_modules/${name}/package.json`, import.meta.url), 'utf8')).version

await init(readFileSync(new URL('../wasm-core/js/quicktex.wasm', import.meta.url)))

const { mathjax } = require('mathjax-full/js/mathjax.js')
const { TeX } = require('mathjax-full/js/input/tex.js')
const { SVG } = require('mathjax-full/js/output/svg.js')
const { liteAdaptor } = require('mathjax-full/js/adaptors/liteAdaptor.js')
const { RegisterHTMLHandler } = require('mathjax-full/js/handlers/html.js')
const { AllPackages } = require('mathjax-full/js/input/tex/AllPackages.js')

const adaptor = liteAdaptor()
RegisterHTMLHandler(adaptor)
const page = mathjax.document('', { InputJax: new TeX({ packages: AllPackages }), OutputJax: new SVG({ fontCache: 'none' }) })

const engines = {
  quicktex: { output: 'SVG', version: 'this repo', run: (tex) => render(tex, true) },
  katex: { output: 'HTML and MathML', version: version('katex'), run: (tex) => katex.renderToString(tex, { displayMode: true, throwOnError: true }) },
  temml: { output: 'MathML', version: version('temml'), run: (tex) => temml.renderToString(tex, { displayMode: true, throwOnError: true }) },
  mathjax: { output: 'SVG', version: version('mathjax-full'), run: (tex) => adaptor.outerHTML(page.convert(tex, { display: true })) },
}

const accepted = (tex) =>
  Object.values(engines).every((engine) => {
    try {
      const out = engine.run(tex)
      return !/merror|mjx-merror|data-mjx-error/.test(out)
    } catch {
      return false
    }
  })

const formulas = corpus.filter((tex) => tex.trim() && accepted(tex))

function median(values) {
  const sorted = [...values].sort((a, b) => a - b)
  return sorted[sorted.length >> 1]
}

const names = Object.keys(engines)
const rounds = Object.fromEntries(names.map((name) => [name, []]))
for (let round = 0; round < 40; round++) {
  for (const name of round % 2 ? names : [...names].reverse()) {
    const { run } = engines[name]
    const start = performance.now()
    for (const tex of formulas) run(tex)
    if (round >= 5) rounds[name].push(((performance.now() - start) * 1000) / formulas.length)
  }
}

const summary = Object.fromEntries(
  names.map((name) => {
    const bytes = formulas.reduce((sum, tex) => sum + engines[name].run(tex).length, 0)
    return [name, { micros: median(rounds[name]), output: engines[name].output, version: engines[name].version, bytes: Math.round(bytes / formulas.length) }]
  }),
)

console.log(JSON.stringify({ formulas: formulas.length, rejected: corpus.length - formulas.length, engines: summary }))
