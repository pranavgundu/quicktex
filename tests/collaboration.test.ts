import { describe, expect, test } from 'bun:test'
import * as Y from 'yjs'
import { SharedProject, sessionToken } from '../src/editor/collaboration'

function sync(a: SharedProject, b: SharedProject) {
  const updateA = Y.encodeStateAsUpdate(a.doc)
  const updateB = Y.encodeStateAsUpdate(b.doc)
  Y.applyUpdate(a.doc, updateB)
  Y.applyUpdate(b.doc, updateA)
}

describe('temporary collaboration', () => {
  test('accepts only complete unguessable session tokens', () => {
    expect(sessionToken('#session=' + 'a'.repeat(64))).toBe('a'.repeat(64))
    expect(sessionToken('#session=short')).toBeNull()
    expect(sessionToken('#session=' + 'z'.repeat(64))).toBeNull()
  })

  test('joining does not seed a second starter document; edits converge', () => {
    const a = new SharedProject(), b = new SharedProject()
    a.seed('hello', { 'chapter.tex': 'chapter', 'image.png': new Uint8Array([1, 2, 3]) })
    expect(b.initialized).toBe(false)
    sync(a, b)
    a.text('main.tex')!.insert(0, 'A')
    b.text('main.tex')!.insert(0, 'B')
    a.text('chapter.tex')!.insert(0, 'C')
    b.text('chapter.tex')!.insert(0, 'D')
    sync(a, b)
    expect(a.snapshot()).toEqual(b.snapshot())
    expect(a.snapshot().source).toContain('hello')
    expect(a.snapshot().source).toHaveLength(7)
    expect(a.snapshot().files['image.png']).toEqual(new Uint8Array([1, 2, 3]))
    a.destroy(); b.destroy()
  })

  test('rename preserves concurrent edits and file identity', () => {
    const a = new SharedProject(), b = new SharedProject()
    a.seed('root', { 'chapter.tex': 'chapter' })
    sync(a, b)
    const text = a.text('chapter.tex')
    a.rename('chapter.tex', 'renamed.tex')
    b.text('chapter.tex')!.insert(0, 'updated ')
    sync(a, b)
    expect(a.text('renamed.tex')).toBe(text)
    expect(a.snapshot().files['renamed.tex']).toBe('updated chapter')
    expect(a.snapshot()).toEqual(b.snapshot())
    a.destroy(); b.destroy()
  })

  test('concurrent file creation preserves both files, root cannot be deleted', () => {
    const a = new SharedProject(), b = new SharedProject()
    a.seed('root', {})
    sync(a, b)
    a.put('same.tex', 'A'); b.put('same.tex', 'B')
    sync(a, b)
    expect(a.snapshot()).toEqual(b.snapshot())
    expect(Object.values(a.snapshot().files).sort()).toEqual(['A', 'B'])
    a.remove('main.tex')
    expect(a.initialized).toBe(true)
    a.destroy(); b.destroy()
  })

  test('deletes propagate and large uploads are rejected', () => {
    const a = new SharedProject(), b = new SharedProject()
    a.seed('root', { 'chapter.tex': 'chapter' }); sync(a, b)
    a.remove('chapter.tex'); sync(a, b)
    expect(b.snapshot().files).toEqual({})
    expect(() => a.put('../bad.tex', 'bad')).toThrow()
    expect(() => a.put('large.png', new Uint8Array(9 * 1024 * 1024))).toThrow('8 MB')
    a.destroy(); b.destroy()
  })
})
