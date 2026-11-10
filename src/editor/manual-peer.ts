import * as Y from 'yjs'
import { applyAwarenessUpdate, encodeAwarenessUpdate, type Awareness } from 'y-protocols/awareness'

function encode(bytes: Uint8Array) {
  let text = ''
  for (let i = 0; i < bytes.length; i += 8192) text += String.fromCharCode(...bytes.subarray(i, i + 8192))
  return btoa(text)
}
function decode(text: string) { return Uint8Array.from(atob(text), char => char.charCodeAt(0)) }

// Manual SDP exchange is a fallback when the public discovery server is down.
// It requires no signaling server, account, or hosted document storage.
export class ManualPeers {
  private peers = new Set<RTCPeerConnection>()
  private channels = new Set<RTCDataChannel>()
  private pending?: RTCPeerConnection
  private timers = new Set<ReturnType<typeof setTimeout>>()
  private destroyed = false
  private gathering = new Set<() => void>()
  private onDocument = (update: Uint8Array, origin: unknown) => this.broadcast('document', update, origin)
  private onAwareness = ({ added, updated, removed }: { added: number[]; updated: number[]; removed: number[] }, origin: unknown) => {
    this.broadcast('awareness', encodeAwarenessUpdate(this.awareness, [...added, ...updated, ...removed]), origin)
  }

  constructor(private token: string, private doc: Y.Doc, private awareness: Awareness) {
    doc.on('update', this.onDocument)
    awareness.on('update', this.onAwareness)
  }

  private broadcast(type: string, bytes: Uint8Array, origin?: unknown) {
    for (const channel of this.channels) if (channel !== origin) this.send(channel, type, bytes)
  }

  private send(channel: RTCDataChannel, type: string, bytes: Uint8Array) {
    if (channel.readyState !== 'open') return
    const data = encode(bytes)
    // Ordered channel; each message has bounded-size frames to support large assets.
    const id = crypto.randomUUID()
    for (let i = 0; i < data.length; i += 12000) {
      const message = JSON.stringify({ id, type, data: data.slice(i, i + 12000), end: i + 12000 >= data.length })
      const queue = this.queues.get(channel) ?? []
      queue.push(message)
      this.queues.set(channel, queue)
    }
    this.flush(channel)
  }
  private queues = new Map<RTCDataChannel, string[]>()
  private flush(channel: RTCDataChannel) {
    const queue = this.queues.get(channel)
    if (!queue || channel.readyState !== 'open') return
    while (queue.length && channel.bufferedAmount < 256 * 1024) channel.send(queue.shift()!)
  }

  private attach(channel: RTCDataChannel) {
    this.channels.add(channel)
    channel.binaryType = 'arraybuffer'
    channel.bufferedAmountLowThreshold = 64 * 1024
    channel.onbufferedamountlow = () => this.flush(channel)
    const incoming = new Map<string, { type: string; data: string }>()
    channel.onopen = () => {
      this.send(channel, 'document', Y.encodeStateAsUpdate(this.doc))
      this.send(channel, 'awareness', encodeAwarenessUpdate(this.awareness, [...this.awareness.getStates().keys()]))
    }
    channel.onmessage = event => {
      try {
        if (typeof event.data !== 'string' || event.data.length > 14000) throw new Error('Invalid frame')
        const frame = JSON.parse(event.data)
        if (typeof frame.id !== 'string' || !['document', 'awareness'].includes(frame.type) || typeof frame.data !== 'string') throw new Error('Invalid frame')
        const partial = incoming.get(frame.id) ?? { type: frame.type, data: '' }
        partial.data += frame.data
        if (partial.data.length > 32 * 1024 * 1024 || incoming.size > 16) throw new Error('Message too large')
        if (frame.end) {
          incoming.delete(frame.id)
          const bytes = decode(partial.data)
          if (partial.type === 'document') Y.applyUpdate(this.doc, bytes, channel)
          else applyAwarenessUpdate(this.awareness, bytes, channel)
        } else incoming.set(frame.id, partial)
      } catch { channel.close() }
    }
    channel.onclose = () => { this.channels.delete(channel); this.queues.delete(channel); incoming.clear() }
  }

  private create() {
    if (this.destroyed) throw new Error('Session ended.')
    if (this.peers.size >= 12) throw new Error('Manual sessions support up to 12 direct connections.')
    const peer = new RTCPeerConnection({ iceServers: [{ urls: 'stun:stun.l.google.com:19302' }] })
    this.peers.add(peer)
    peer.ondatachannel = event => this.attach(event.channel)
    peer.onconnectionstatechange = () => {
      if (peer.connectionState === 'failed' || peer.connectionState === 'closed') { peer.close(); this.peers.delete(peer) }
    }
    return peer
  }

  private async code(peer: RTCPeerConnection) {
    if (peer.iceGatheringState !== 'complete') await new Promise<void>(resolve => {
      const finish = () => { peer.removeEventListener('icegatheringstatechange', check); clearTimeout(timer); this.timers.delete(timer); this.gathering.delete(finish); resolve() }
      const check = () => { if (peer.iceGatheringState === 'complete') finish() }
      const timer = setTimeout(finish, 8000)
      this.timers.add(timer)
      this.gathering.add(finish)
      peer.addEventListener('icegatheringstatechange', check)
    })
    if (this.destroyed) throw new Error('Session ended.')
    return encode(new TextEncoder().encode(JSON.stringify({ token: this.token, description: peer.localDescription })))
  }

  private description(code: string, type: RTCSdpType): RTCSessionDescriptionInit {
    if (code.length > 100000) throw new Error('Connection code is too large.')
    let value
    try { value = JSON.parse(new TextDecoder().decode(decode(code.trim()))) }
    catch { throw new Error('Invalid connection code. Copy the entire code.') }
    if (!value || value.token !== this.token || value.description?.type !== type || typeof value.description?.sdp !== 'string') throw new Error('Wrong session or connection code type.')
    return value.description
  }

  async offer() {
    if (this.pending) { this.pending.close(); this.peers.delete(this.pending) }
    const peer = this.create()
    this.pending = peer
    this.attach(peer.createDataChannel('quicktex'))
    await peer.setLocalDescription(await peer.createOffer())
    return this.code(peer)
  }

  async answer(code: string) {
    const description = this.description(code, 'offer')
    const peer = this.create()
    try {
      await peer.setRemoteDescription(description)
      await peer.setLocalDescription(await peer.createAnswer())
      return await this.code(peer)
    } catch (error) { peer.close(); this.peers.delete(peer); throw error }
  }

  async finish(code: string) {
    if (this.destroyed) throw new Error('Session ended.')
    if (!this.pending) throw new Error('Create an invitation code first.')
    await this.pending.setRemoteDescription(this.description(code, 'answer'))
    this.pending = undefined
  }

  destroy() {
    this.destroyed = true
    this.doc.off('update', this.onDocument)
    this.awareness.off('update', this.onAwareness)
    for (const peer of this.peers) peer.close()
    for (const finish of [...this.gathering]) finish()
    for (const timer of this.timers) clearTimeout(timer)
    this.pending = undefined
    this.peers.clear(); this.channels.clear(); this.queues.clear()
  }
}
