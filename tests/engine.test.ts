import { beforeEach, describe, expect, test } from 'bun:test'
import { type Answer, connect, type Compiled, type Failed, type Port, type Request, type Sheet, type Timed } from '../src/editor/engine'

class FakePort implements Port {
  sent: Request[] = []
  onmessage: Port['onmessage'] = null
  onerror: Port['onerror'] = null

  postMessage(request: Request) {
    this.sent.push(request)
  }

  reply(answer: Answer) {
    this.onmessage?.({ data: answer } as MessageEvent<Answer>)
  }

  compiled(id = 1): Compiled {
    return { kind: 'compiled', id, pages: 1, micros: 5, warningCount: 0, warningAt: null, sheets: [] }
  }

  kinds() {
    return this.sent.map((request) => request.kind)
  }

  texts() {
    return this.sent.flatMap((request) => (request.kind === 'compile' ? [request.tex] : []))
  }
}

describe('engine connection', () => {
  let port: FakePort
  let engine: ReturnType<typeof connect>
  let compiled: Compiled[]
  let failed: Failed[]
  let timed: Timed[]
  let sheets: Sheet[][]
  let broken: number

  beforeEach(() => {
    port = new FakePort()
    compiled = []
    failed = []
    timed = []
    sheets = []
    broken = 0
    engine = connect(port, {
      compiled: (answer) => compiled.push(answer),
      failed: (answer) => failed.push(answer),
      timed: (answer) => timed.push(answer),
      sheets: (list) => sheets.push(list),
      broken: () => broken++,
    })
  })

  test('the first text goes straight to the worker', () => {
    engine.compile('a')
    expect(port.texts()).toEqual(['a'])
  })

  test('typing preserves project files queued during a compile', () => {
    engine.compile('a')
    const files = { 'part.tex': 'included text' }
    engine.compile('b', files)
    engine.compile('c')
    port.reply(port.compiled())
    expect(port.sent[1]).toMatchObject({ kind: 'compile', tex: 'c', files })
  })

  test('only one compile runs at a time', () => {
    engine.compile('a')
    engine.compile('b')
    expect(port.texts()).toEqual(['a'])
  })

  test('text typed during a compile collapses to the newest', () => {
    engine.compile('a')
    for (const tex of ['b', 'c', 'd', 'e']) engine.compile(tex)
    port.reply(port.compiled())
    expect(port.texts()).toEqual(['a', 'e'])
  })

  test('a long held key costs two compiles, not one per key', () => {
    engine.compile('start')
    for (let i = 0; i < 500; i++) engine.compile(`key ${i}`)
    port.reply(port.compiled())
    port.reply(port.compiled())
    expect(port.texts()).toEqual(['start', 'key 499'])
  })

  test('a failed compile still starts the next one', () => {
    engine.compile('a')
    engine.compile('b')
    port.reply({ kind: 'failed', id: 1, message: 'no', at: 0, line: 1 })
    expect(failed).toHaveLength(1)
    expect(port.texts()).toEqual(['a', 'b'])
  })

  test('nothing more is sent once the worker catches up', () => {
    engine.compile('a')
    port.reply(port.compiled())
    expect(port.sent).toHaveLength(1)
    expect(compiled).toHaveLength(1)
  })

  test('timing waits for a running compile', () => {
    engine.compile('a')
    engine.time(5)
    expect(port.kinds()).toEqual(['compile'])
    port.reply(port.compiled())
    expect(port.kinds()).toEqual(['compile', 'time'])
  })

  test('new text cancels timing that has not started', () => {
    engine.compile('a')
    engine.time(5)
    engine.compile('b')
    port.reply(port.compiled())
    port.reply(port.compiled())
    expect(port.kinds()).toEqual(['compile', 'compile'])
  })

  test('timing is skipped when text is already waiting', () => {
    engine.compile('a')
    engine.compile('b')
    engine.time(5)
    port.reply(port.compiled())
    port.reply(port.compiled())
    expect(port.kinds()).toEqual(['compile', 'compile'])
  })

  test('text waits for running timing', () => {
    engine.time(5)
    engine.compile('a')
    expect(port.kinds()).toEqual(['time'])
    port.reply({ kind: 'timed', id: 1, micros: 1, pdfMicros: 2 })
    expect(port.kinds()).toEqual(['time', 'compile'])
  })

  test('stale timing is not shown over newer text', () => {
    engine.time(5)
    engine.compile('a')
    port.reply({ kind: 'timed', id: 1, micros: 1, pdfMicros: 2 })
    expect(timed).toHaveLength(0)
  })

  test('fresh timing is shown', () => {
    engine.time(5)
    port.reply({ kind: 'timed', id: 1, micros: 1, pdfMicros: 2 })
    expect(timed).toHaveLength(1)
  })

  test('watching pages does not wait for a compile', () => {
    engine.compile('a')
    engine.watch([3])
    expect(port.kinds()).toEqual(['compile', 'watch'])
  })

  test('a page reply does not end a running compile', () => {
    engine.compile('a')
    engine.compile('b')
    port.reply({ kind: 'sheets', id: 2, sheets: [[0, '<svg/>']] })
    expect(sheets).toEqual([[[0, '<svg/>']]])
    expect(port.texts()).toEqual(['a'])
  })

  test('a pdf reply does not end a running compile', async () => {
    engine.compile('a')
    engine.compile('b')
    const printed = engine.pdf()
    port.reply({ kind: 'pdf', id: 2, bytes: new Uint8Array([1]) })
    expect(await printed).toEqual(new Uint8Array([1]))
    expect(port.texts()).toEqual(['a'])
  })

  test('pdf replies resolve in order', async () => {
    const first = engine.pdf()
    const second = engine.pdf()
    port.reply({ kind: 'pdf', id: 1, bytes: new Uint8Array([1]) })
    port.reply({ kind: 'pdf', id: 2, bytes: new Uint8Array([2]) })
    expect(await first).toEqual(new Uint8Array([1]))
    expect(await second).toEqual(new Uint8Array([2]))
  })

  test('a crashed worker is reported', () => {
    port.onerror?.({} as ErrorEvent)
    expect(broken).toBe(1)
  })
})
