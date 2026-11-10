import { afterEach, describe, expect, test } from 'bun:test'
import * as Y from 'yjs'
import { Awareness } from 'y-protocols/awareness'
import { SharedProject, newSessionToken, sessionLimit, sessionToken } from '../src/editor/collaboration'

const projects: SharedProject[] = []
const awarenesses: Awareness[] = []
function project(source = 'root', files = {}) {
  const p = new SharedProject(); projects.push(p); p.seed(source, files); return p
}
function replica(p: SharedProject) {
  const other = new SharedProject(); projects.push(other)
  Y.applyUpdate(other.doc, Y.encodeStateAsUpdate(p.doc)); return other
}
function exchange(peers: SharedProject[]) {
  const updates = peers.map(p => Y.encodeStateAsUpdate(p.doc))
  for (const p of peers) for (const update of updates) Y.applyUpdate(p.doc, update)
}
function awareness(p: SharedProject) {
  const a = new Awareness(p.doc); awarenesses.push(a); return a
}
afterEach(() => { for (const a of awarenesses.splice(0)) a.destroy(); for (const p of projects.splice(0)) p.destroy() })

describe('session identities', () => {
  test.each(['', '#', '#session=', '#session=abc', '#session=' + 'a'.repeat(63), '#session=' + 'a'.repeat(65), '#session=' + 'A'.repeat(64), '#session=' + 'a'.repeat(63) + '/', '#other=' + 'a'.repeat(64)])('rejects malformed link %s', hash => {
    expect(sessionToken(hash)).toBeNull()
  })
  test('tokens round trip, contain 256 bits, and do not repeat in a sample', () => {
    const tokens = Array.from({ length: 200 }, newSessionToken)
    expect(new Set(tokens).size).toBe(200)
    for (const token of tokens) { expect(token).toMatch(/^[a-f0-9]{64}$/); expect(sessionToken(`#session=${token}`)).toBe(token) }
  })
  test('allows unrelated fragment parameters', () => {
    const token = newSessionToken()
    expect(sessionToken(`#other=1&session=${token}&last=2`)).toBe(token)
  })
})

describe('project invariants', () => {
  test('empty root is initialized and cannot be seeded twice', () => {
    const p = project(''); expect(p.initialized).toBe(true); expect(p.snapshot().source).toBe('')
    expect(() => p.seed('replacement', {})).toThrow('already initialized')
    expect(p.snapshot().source).toBe('')
  })
  test('root put on a fresh project establishes the canonical root', () => {
    const p = new SharedProject(); projects.push(p); p.put('main.tex', 'hello')
    expect(p.initialized).toBe(true); expect(p.snapshot().source).toBe('hello')
    expect(p.paths().get('main.tex')).toBe('main')
  })
  test('a files entry cannot override the seeded root', () => {
    expect(project('root', { 'main.tex': 'imposter' }).snapshot().source).toBe('root')
  })
  test.each(['', '/absolute.tex', '../up.tex', 'a/../b.tex', 'a/./b.tex', 'a//b.tex', 'a\\b.tex', 'a/'])('rejects unsafe path %s without mutation', path => {
    const p = project(); const before = p.snapshot()
    expect(() => p.put(path, 'bad')).toThrow(); expect(p.snapshot()).toEqual(before)
  })
  test.each(['chapter.tex', 'a/b/c.tex', 'résumé.tex', '中文.tex', '__proto__', 'constructor', 'dir.with.dot/file'])('round trips path %s safely', path => {
    const p = project(); p.put(path, 'contents')
    expect(p.snapshot().files[path]).toBe('contents'); expect(Object.getPrototypeOf(p.snapshot().files)).toBeNull()
  })
  test('binary bytes and zero-length assets survive replication', () => {
    const bytes = Uint8Array.from({ length: 1024 }, (_, i) => i % 256)
    const p = project('root', { 'data.bin': bytes, 'empty.bin': new Uint8Array() })
    expect(replica(p).snapshot()).toEqual(p.snapshot()); expect(p.text('data.bin')).toBeUndefined()
  })
  test('snapshots are independent containers and do not expose text mutation', () => {
    const p = project('root', { 'part.tex': 'part' }); const snapshot = p.snapshot()
    snapshot.source = 'changed'; snapshot.files['part.tex'] = 'changed'; snapshot.files['new.tex'] = 'new'
    expect(p.snapshot()).toEqual({ source: 'root', files: { 'part.tex': 'part' } })
  })
  test('replacing text preserves CRDT identity', () => {
    const p = project(); p.put('part.tex', 'before'); const text = p.text('part.tex')
    p.put('part.tex', 'after'); expect(p.text('part.tex')).toBe(text); expect(text?.toString()).toBe('after')
  })
  test.each(['main.tex', 'existing.tex', '../bad.tex'])('rejects rename to %s without mutation', to => {
    const p = project('root', { 'part.tex': 'part', 'existing.tex': 'existing' }); const before = p.snapshot()
    expect(() => p.rename('part.tex', to)).toThrow(); expect(p.snapshot()).toEqual(before)
  })
  test('root cannot be renamed and missing removal is harmless', () => {
    const p = project(); expect(() => p.rename('main.tex', 'other.tex')).toThrow()
    p.remove('absent.tex'); p.remove('main.tex'); expect(p.snapshot().source).toBe('root')
  })
  test('size boundary accepts exact limit and rejects one extra byte atomically', () => {
    const p = project(''); p.put('large.bin', new Uint8Array(sessionLimit))
    const before = p.snapshot(); expect(() => p.put('one.txt', 'x')).toThrow('8 MB')
    expect(p.snapshot()).toEqual(before)
  })
  test('replacing a large file subtracts its existing size', () => {
    const p = project(''); p.put('large.bin', new Uint8Array(sessionLimit))
    p.put('large.bin', new Uint8Array(sessionLimit)); p.put('large.bin', new Uint8Array(1))
    p.put('other.bin', new Uint8Array(sessionLimit - 1)); expect(Object.keys(p.snapshot().files)).toHaveLength(2)
  })
  test('size uses UTF-8 bytes, not JavaScript string length', () => {
    const p = project(''); p.put('large.bin', new Uint8Array(sessionLimit - 3))
    expect(() => p.put('unicode.tex', '😀')).toThrow('8 MB'); p.put('unicode.tex', '€')
    expect(p.snapshot().files['unicode.tex']).toBe('€')
  })
  test('undo managers persist through rename and differ between files', () => {
    const p = project('root', { 'a.tex': 'a', 'b.tex': 'b' }); const a = awareness(p)
    const editor = p.editor('a.tex', a)!; p.rename('a.tex', 'renamed.tex')
    expect(p.editor('renamed.tex', a)?.undoManager).toBe(editor.undoManager)
    expect(p.editor('b.tex', a)?.undoManager).not.toBe(editor.undoManager)
    expect(p.editor('missing.tex', a)).toBeUndefined()
  })
  test('replacing binary then restoring text uses a fresh undo scope', () => {
    const p = project('root', { 'a.tex': 'old' }); const a = awareness(p)
    const old = p.editor('a.tex', a)!
    p.put('a.tex', new Uint8Array([1])); p.put('a.tex', 'new')
    const fresh = p.editor('a.tex', a)!
    expect(fresh.text).not.toBe(old.text); expect(fresh.undoManager.scope).toContain(fresh.text)
  })
})

describe('convergence under contention', () => {
  test('three offline editors merge inserts, deletes, and unicode edits', () => {
    const a = project('abcdef'), b = replica(a), c = replica(a)
    a.text('main.tex')!.insert(2, '😀'); b.text('main.tex')!.delete(1, 3); c.text('main.tex')!.insert(0, '中文')
    exchange([c, a, b]); expect(a.snapshot()).toEqual(b.snapshot()); expect(b.snapshot()).toEqual(c.snapshot())
    expect(a.snapshot().source).toContain('😀'); expect(a.snapshot().source).toContain('中文')
  })
  test('duplicate and out-of-order updates are idempotent', () => {
    const p = project(''); const updates: Uint8Array[] = []
    p.doc.on('update', update => updates.push(update))
    for (let i = 0; i < 30; i++) p.text('main.tex')!.insert(i, String(i % 10))
    const other = new SharedProject(); projects.push(other)
    for (const update of [...updates].reverse()) { Y.applyUpdate(other.doc, update); Y.applyUpdate(other.doc, update) }
    Y.applyUpdate(other.doc, Y.encodeStateAsUpdate(p.doc))
    expect(other.snapshot()).toEqual(p.snapshot())
  })
  test('simultaneous renames converge to one path without losing edits', () => {
    const a = project('root', { 'part.tex': 'part' }), b = replica(a)
    a.rename('part.tex', 'a.tex'); b.rename('part.tex', 'b.tex'); b.text('b.tex')!.insert(4, '!')
    exchange([a, b]); expect(a.snapshot()).toEqual(b.snapshot()); expect(Object.values(a.snapshot().files)).toEqual(['part!'])
  })
  test('delete wins over an offline edit without resurrecting the file', () => {
    const a = project('root', { 'part.tex': 'part' }), b = replica(a)
    a.remove('part.tex'); b.text('part.tex')!.insert(0, 'offline ')
    exchange([a, b]); expect(a.snapshot().files).toEqual({}); expect(b.snapshot().files).toEqual({})
  })
  test('delete and recreate does not attach stale edits to a new file', () => {
    const a = project('root', { 'part.tex': 'old' }), b = replica(a)
    a.remove('part.tex'); a.put('part.tex', 'new'); b.text('part.tex')!.insert(0, 'stale ')
    exchange([a, b]); expect(a.snapshot().files['part.tex']).toBe('new'); expect(a.snapshot()).toEqual(b.snapshot())
  })
  test.each(['same.tex', 'dir/same.tex', 'no-extension'])('three colliding creations preserve all contents at %s', path => {
    const a = project(), b = replica(a), c = replica(a)
    a.put(path, 'A'); b.put(path, 'B'); c.put(path, 'C'); exchange([a, b, c])
    expect(a.snapshot()).toEqual(b.snapshot()); expect(b.snapshot()).toEqual(c.snapshot())
    expect(Object.values(a.snapshot().files).sort()).toEqual(['A', 'B', 'C']); expect(a.paths().size).toBe(4)
  })
  test('collision resolution also handles existing conflict-suffixed names', () => {
    const a = project('root', { 'same-conflict.tex': 'original' }), b = replica(a)
    a.put('same.tex', 'A'); b.put('same.tex', 'B'); exchange([a, b])
    expect(Object.values(a.snapshot().files).sort()).toEqual(['A', 'B', 'original']); expect(a.snapshot()).toEqual(b.snapshot())
  })
  for (const seed of [1, 42, 2026, 65537, 999999]) test(`seeded randomized edits converge (seed ${seed})`, () => {
    const peers = [project('abcdef')]; peers.push(replica(peers[0]), replica(peers[0]))
    let state = seed
    const random = (limit: number) => { state = (Math.imul(state, 1664525) + 1013904223) >>> 0; return state % limit }
    for (let i = 0; i < 200; i++) {
      const text = peers[random(3)].text('main.tex')!
      if (text.length && random(3) === 0) text.delete(random(text.length), 1)
      else text.insert(random(text.length + 1), String.fromCharCode(65 + random(26)))
      if (i % 17 === 0) exchange(peers)
    }
    exchange(peers); expect(peers[0].snapshot()).toEqual(peers[1].snapshot()); expect(peers[1].snapshot()).toEqual(peers[2].snapshot())
  })
})
