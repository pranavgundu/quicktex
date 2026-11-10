import * as Y from 'yjs'
import { WebrtcProvider } from 'y-webrtc'
import type { Awareness } from 'y-protocols/awareness'
import type { ProjectFiles } from '../../wasm-core/js/quicktex.js'
import { validPath } from './project'
import { ManualPeers } from './manual-peer'

export const sessionLimit = 8 * 1024 * 1024
const tokenPattern = /^[a-f0-9]{64}$/

export function sessionToken(hash: string): string | null {
  const token = new URLSearchParams(hash.replace(/^#/, '')).get('session')
  return token && tokenPattern.test(token) ? token : null
}

export function newSessionToken() {
  return Array.from(crypto.getRandomValues(new Uint8Array(32)), byte => byte.toString(16).padStart(2, '0')).join('')
}

export type SharedEditor = { text: Y.Text; awareness: Awareness; undoManager: Y.UndoManager }

// Only this in-memory CRDT is transmitted. No IndexedDB, cloud database, or document server.
export class SharedProject {
  readonly doc = new Y.Doc()
  readonly entries = this.doc.getMap<Y.Map<unknown>>('files')
  private undos = new Map<string, Y.UndoManager>()

  seed(source: string, files: ProjectFiles) {
    if (this.entries.size) throw new Error('Session is already initialized.')
    this.doc.transact(() => {
      this.put('main.tex', source, 'main')
      for (const [path, content] of Object.entries(files)) if (path !== 'main.tex') this.put(path, content)
    })
  }

  get initialized() { return this.entries.has('main') }

  // Stable IDs preserve text identity across renames. Concurrent duplicate paths get
  // deterministic suffixes rather than hiding or overwriting either person's work.
  paths() {
    const result = new Map<string, string>()
    const used = new Set(['main.tex'])
    if (this.initialized) result.set('main.tex', 'main')
    for (const [id, entry] of [...this.entries].sort(([a], [b]) => a.localeCompare(b))) {
      if (id === 'main') continue
      const path = entry.get('path')
      if (typeof path !== 'string' || !validPath(path)) continue
      let unique = path
      while (used.has(unique)) {
        const dot = unique.lastIndexOf('.')
        unique = dot > unique.lastIndexOf('/') ? `${unique.slice(0, dot)}-conflict${unique.slice(dot)}` : `${unique}-conflict`
      }
      used.add(unique)
      result.set(unique, id)
    }
    return result
  }

  text(path: string) {
    const id = this.paths().get(path)
    const content = id ? this.entries.get(id)?.get('content') : undefined
    return content instanceof Y.Text ? content : undefined
  }

  snapshot(): { source: string; files: ProjectFiles } {
    const files: ProjectFiles = Object.create(null)
    let source = ''
    for (const [path, id] of this.paths()) {
      const content = this.entries.get(id)?.get('content')
      if (content instanceof Y.Text) {
        if (path === 'main.tex') source = content.toString()
        else files[path] = content.toString()
      } else if (content instanceof Uint8Array) files[path] = content
    }
    return { source, files }
  }

  put(path: string, content: string | Uint8Array, id = this.paths().get(path) ?? (path === 'main.tex' ? 'main' : crypto.randomUUID())) {
    if (!validPath(path)) throw new Error('Invalid file path.')
    const snapshot = this.snapshot()
    const size = (value: string | Uint8Array) => typeof value === 'string' ? new TextEncoder().encode(value).length : value.byteLength
    const total = size(snapshot.source) + Object.values(snapshot.files).reduce((sum, value) => sum + size(value), 0)
    const previous = path === 'main.tex' ? snapshot.source : snapshot.files[path]
    if (total - (previous === undefined ? 0 : size(previous)) + size(content) > sessionLimit) throw new Error('Live sessions support up to 8 MB. Use a smaller project.')
    this.doc.transact(() => {
      const existing = this.entries.get(id)?.get('content')
      if (existing instanceof Y.Text && typeof content === 'string') {
        existing.delete(0, existing.length)
        existing.insert(0, content)
        return
      }
      const entry = new Y.Map<unknown>()
      entry.set('path', path)
      entry.set('content', typeof content === 'string' ? new Y.Text(content) : content)
      this.entries.set(id, entry)
    })
  }

  rename(from: string, to: string) {
    if (from === 'main.tex' || to === 'main.tex' || !validPath(to) || this.paths().has(to)) throw new Error('That file name is unavailable.')
    const id = this.paths().get(from)
    if (id) this.entries.get(id)?.set('path', to)
  }

  remove(path: string) {
    const id = this.paths().get(path)
    if (id && id !== 'main') this.entries.delete(id)
  }

  editor(path: string, awareness: Awareness): SharedEditor | undefined {
    const text = this.text(path)
    if (!text) return undefined
    const id = this.paths().get(path)!
    let undoManager = this.undos.get(id)
    if (!undoManager || !undoManager.scope.includes(text)) {
      undoManager?.destroy()
      undoManager = new Y.UndoManager(text, { trackedOrigins: new Set() })
      this.undos.set(id, undoManager)
    }
    return { text, awareness, undoManager }
  }

  destroy() {
    for (const undo of this.undos.values()) undo.destroy()
    this.doc.destroy()
  }
}

export function connectSession(token: string, project: SharedProject, changed: () => void) {
  if (!tokenPattern.test(token)) throw new Error('Invalid session link.')
  // Separate discovery ID and secret. The password encrypts signaling; WebRTC
  // encrypts document traffic. Anyone holding the complete link can edit.
  const provider = new WebrtcProvider(`quicktex-${token.slice(0, 32)}`, project.doc, {
    password: token.slice(32),
    maxConns: 12,
    // STUN only: no paid TURN relay, and consequently no universal connectivity.
    peerOpts: { config: { iceServers: [{ urls: 'stun:stun.l.google.com:19302' }] } },
  })
  const colors = ['#1b5e44', '#8b4ab5', '#bd5734', '#2769ad']
  const color = colors[project.doc.clientID % colors.length]
  provider.awareness.setLocalStateField('user', { name: `Guest ${project.doc.clientID % 10000}`, color, colorLight: `${color}33` })
  const manual = new ManualPeers(token, project.doc, provider.awareness)
  project.entries.observeDeep(changed)
  provider.awareness.on('change', changed)
  provider.on('peers', changed)
  return {
    provider,
    manual,
    destroy() {
      project.entries.unobserveDeep(changed)
      provider.awareness.off('change', changed)
      provider.off('peers', changed)
      manual.destroy()
      provider.destroy()
      project.destroy()
    },
  }
}
