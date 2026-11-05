import { documentPage, documentPdf, documentWarnings, init, loadFontsFor, registerFont, setFiles, QuicktexError, renderDocument } from '../../wasm-core/js/quicktex.js'
import { charIndex } from '../lib/latex'
import { measure } from '../lib/timing'
import type { Answer, Request, Sheet } from './engine'

const heavy = 2000
const ready = init()
const drawn: number[] = []

let stamp = 0
let tex = ''
let total = 0
let watched = [0]

function sheets() {
  const fresh: Sheet[] = []
  for (const page of new Set([0, ...watched])) {
    if (page >= total || drawn[page] === stamp) continue
    fresh.push([page, documentPage(page)])
    drawn[page] = stamp
  }
  return fresh
}

function compile(id: number, source: string): Answer {
  const start = performance.now()
  try {
    total = renderDocument(source)
    const micros = (performance.now() - start) * 1000
    tex = source
    stamp++
    const warnings = documentWarnings()
    const warningAt = warnings.length ? charIndex(source, warnings[0]) : null
    return { kind: 'compiled', id, pages: total, micros, warningCount: warnings.length, warningAt, sheets: sheets() }
  } catch (error) {
    if (!(error instanceof QuicktexError)) throw error
    total = 0
    const at = charIndex(source, error.position)
    return { kind: 'failed', id, message: error.message, at, line: source.slice(0, at).split('\n').length }
  }
}

function time(id: number, micros: number): Answer {
  const compile = micros < heavy ? measure(() => renderDocument(tex)) : micros
  const start = performance.now()
  documentPdf()
  const once = (performance.now() - start) * 1000
  return { kind: 'timed', id, micros: compile, pdfMicros: once < heavy ? measure(documentPdf) : once }
}

function answer(request: Request): Answer {
  if (request.kind === 'compile') return compile(request.id, request.tex)
  if (request.kind === 'time') return time(request.id, request.micros)
  if (request.kind === 'pdf') return { kind: 'pdf', id: request.id, bytes: documentPdf() }
  watched = request.pages
  return { kind: 'sheets', id: request.id, sheets: sheets() }
}

self.onmessage = async ({ data }: MessageEvent<Request>) => {
  await ready
  if (data.kind === 'compile') {
    if (data.files) setFiles(data.files);
    try {
      for (const [name, value] of Object.entries(data.files ?? {})) {
        if (/\.(ttf|otf)$/i.test(name) && typeof value !== 'string') await registerFont(value);
      }
      const included = Object.values(data.files ?? {}).filter((value): value is string => typeof value === 'string').join(' ');
      await loadFontsFor(data.tex + included);
    } catch (error) {
      total = 0
      self.postMessage({ kind: 'failed', id: data.id, message: String(error), at: 0, line: 1 });
      return;
    }
  }
  const reply = answer(data)
  if (reply.kind === 'pdf') self.postMessage(reply, { transfer: [reply.bytes.buffer as ArrayBuffer] })
  else self.postMessage(reply)
}
