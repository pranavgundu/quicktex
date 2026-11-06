import { describe, expect, test } from 'bun:test'
import { documents } from '../bench/documents'
import { documentWarnings } from '../wasm-core/js/quicktex.js'
import { renderDocument } from './support/engine'

describe('benchmark documents', () => {
  for (const [name, document] of Object.entries(documents)) {
    test(`${name} compiles without compatibility warnings`, () => {
      expect(renderDocument(document.tex())).toBeGreaterThan(0)
      expect(documentWarnings()).toEqual([])
    })
  }

  test('documents that need more than one pass say so', () => {
    for (const [name, document] of Object.entries(documents)) {
      const crossReferenced = /\\ref\{|\\tableofcontents/.test(document.tex())
      expect(`${name} ${document.passes}`).toBe(`${name} ${crossReferenced}`)
    }
  })
})
