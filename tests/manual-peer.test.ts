import { afterEach, beforeEach, describe, expect, test } from 'bun:test'
import * as Y from 'yjs'
import { Awareness } from 'y-protocols/awareness'
import { ManualPeers } from '../src/editor/manual-peer'

class Channel {
  readyState = 'connecting'
  bufferedAmount = 0
  bufferedAmountLowThreshold = 0
  binaryType = ''
  sent: string[] = []
  onopen?: () => void
  onclose?: () => void
  onmessage?: (event: { data: unknown }) => void
  onbufferedamountlow?: () => void
  send(value: string) { this.sent.push(value) }
  open() { this.readyState = 'open'; this.onopen?.() }
  close() { this.readyState = 'closed'; this.onclose?.() }
  receive(data: unknown) { this.onmessage?.({ data }) }
}
class Peer extends EventTarget {
  static created: Peer[] = []
  static gathering = 'complete'
  static rejectRemote = false
  iceGatheringState = Peer.gathering
  connectionState = 'new'
  localDescription: RTCSessionDescriptionInit | null = null
  remoteDescription: RTCSessionDescriptionInit | null = null
  ondatachannel?: (event: { channel: Channel }) => void
  onconnectionstatechange?: () => void
  channels: Channel[] = []
  constructor(public config: RTCConfiguration) { super(); Peer.created.push(this) }
  createDataChannel() { const channel = new Channel(); this.channels.push(channel); return channel }
  async createOffer() { return { type: 'offer', sdp: 'offer-sdp' } as const }
  async createAnswer() { return { type: 'answer', sdp: 'answer-sdp' } as const }
  async setLocalDescription(description: RTCSessionDescriptionInit) { this.localDescription = description }
  async setRemoteDescription(description: RTCSessionDescriptionInit) { if (Peer.rejectRemote) throw new Error('Bad SDP'); this.remoteDescription = description }
  close() { this.connectionState = 'closed'; for (const channel of this.channels) channel.close() }
}
const token = 'a'.repeat(64)
const code = (type: string, session = token) => btoa(JSON.stringify({ token: session, description: { type, sdp: 'test-sdp' } }))
const originalPeer = globalThis.RTCPeerConnection
let doc: Y.Doc, awareness: Awareness, manual: ManualPeers
beforeEach(() => {
  Peer.created = []; Peer.gathering = 'complete'; Peer.rejectRemote = false
  globalThis.RTCPeerConnection = Peer as unknown as typeof RTCPeerConnection
  doc = new Y.Doc(); awareness = new Awareness(doc); manual = new ManualPeers(token, doc, awareness)
})
afterEach(() => { manual.destroy(); awareness.destroy(); doc.destroy(); globalThis.RTCPeerConnection = originalPeer })
const lastPeer = () => Peer.created.at(-1)!
async function open() { await manual.offer(); const channel = lastPeer().channels[0]; channel.open(); return channel }

describe('manual connection validation and lifecycle', () => {
  test('offer code contains the current session and local SDP', async () => {
    const value = JSON.parse(atob(await manual.offer()))
    expect(value.token).toBe(token); expect(value.description).toEqual({ type: 'offer', sdp: 'offer-sdp' })
    expect(lastPeer().config.iceServers).toEqual([{ urls: 'stun:stun.l.google.com:19302' }])
  })
  test('answer accepts whitespace around a valid invitation', async () => {
    const reply = JSON.parse(atob(await manual.answer(` \n${code('offer')}\n `)))
    expect(reply.description.type).toBe('answer'); expect(lastPeer().remoteDescription?.type).toBe('offer')
  })
  test.each(['', '%not-base64', btoa('not json'), btoa('null'), btoa('{}'), code('answer'), code('offer', 'b'.repeat(64)), 'x'.repeat(100001)])('rejects invalid invitation %# before allocating a peer', async invitation => {
    await expect(manual.answer(invitation)).rejects.toThrow()
    expect(Peer.created).toHaveLength(0)
  })
  test('finish requires a pending invitation', async () => { await expect(manual.finish(code('answer'))).rejects.toThrow('invitation') })
  test('wrong reply does not consume a valid pending invitation', async () => {
    await manual.offer(); await expect(manual.finish(code('offer'))).rejects.toThrow('Wrong session')
    await manual.finish(code('answer')); expect(lastPeer().remoteDescription?.type).toBe('answer')
    await expect(manual.finish(code('answer'))).rejects.toThrow('invitation')
  })
  test('a replacement invitation closes the old pending peer', async () => {
    await manual.offer(); const old = lastPeer(); await manual.offer()
    expect(old.connectionState).toBe('closed'); expect(lastPeer()).not.toBe(old)
  })
  test('SDP failure closes and releases the failed answer connection', async () => {
    Peer.rejectRemote = true; await expect(manual.answer(code('offer'))).rejects.toThrow('Bad SDP')
    expect(lastPeer().connectionState).toBe('closed')
  })
  test('rejects more than twelve direct peers', async () => {
    for (let i = 0; i < 12; i++) await manual.answer(code('offer'))
    await expect(manual.answer(code('offer'))).rejects.toThrow('12 direct')
  })
  test('destroy closes channels and stops document broadcasts', async () => {
    const channel = await open(); const count = channel.sent.length
    manual.destroy(); doc.getText('text').insert(0, 'after destroy')
    expect(channel.readyState).toBe('closed'); expect(channel.sent).toHaveLength(count)
  })
  test('destroyed sessions cannot allocate new peers', async () => {
    manual.destroy(); await expect(manual.offer()).rejects.toThrow('Session ended')
    await expect(manual.answer(code('offer'))).rejects.toThrow('Session ended')
    expect(Peer.created).toHaveLength(0)
  })
  test('destroy settles an invitation still gathering ICE', async () => {
    Peer.gathering = 'gathering'
    const pending = manual.offer()
    await new Promise(resolve => setTimeout(resolve, 0))
    manual.destroy()
    const result = await Promise.race([pending.then(() => 'resolved', error => error.message), new Promise(resolve => setTimeout(() => resolve('hung'), 50))])
    expect(result).toBe('Session ended.')
  })
})

describe('manual data channel protocol', () => {
  test('opening sends initial document and awareness state', async () => {
    doc.getText('text').insert(0, 'existing')
    const channel = await open()
    expect(channel.sent.map(raw => JSON.parse(raw).type)).toEqual(['document', 'awareness'])
    const remote = new Y.Doc()
    Y.applyUpdate(remote, Uint8Array.from(atob(JSON.parse(channel.sent[0]).data), c => c.charCodeAt(0)))
    expect(remote.getText('text').toString()).toBe('existing'); remote.destroy()
  })
  test('large state is split into bounded frames and reassembled exactly', async () => {
    const content = 'abcdefghij'.repeat(12000); doc.getText('text').insert(0, content)
    const channel = await open(); const frames = channel.sent.map(raw => JSON.parse(raw)).filter(frame => frame.type === 'document')
    expect(frames.length).toBeGreaterThan(10); expect(frames.every(frame => frame.data.length <= 12000)).toBe(true)
    expect(frames.at(-1).end).toBe(true)
    const remote = new Y.Doc()
    Y.applyUpdate(remote, Uint8Array.from(atob(frames.map(frame => frame.data).join('')), c => c.charCodeAt(0)))
    expect(remote.getText('text').toString()).toBe(content); remote.destroy()
  })
  test('backpressure queues updates until the channel drains', async () => {
    const channel = await open(); channel.sent = []; channel.bufferedAmount = 300000
    doc.getText('text').insert(0, 'queued'); expect(channel.sent).toHaveLength(0)
    channel.bufferedAmount = 0; channel.onbufferedamountlow?.(); expect(channel.sent).toHaveLength(1)
  })
  test('remote updates apply without echoing to their originating channel', async () => {
    const channel = await open(); channel.sent = []
    const remote = new Y.Doc(); remote.getText('text').insert(0, 'remote')
    const data = btoa(String.fromCharCode(...Y.encodeStateAsUpdate(remote)))
    channel.receive(JSON.stringify({ id: 'one', type: 'document', data, end: true }))
    expect(doc.getText('text').toString()).toBe('remote'); expect(channel.sent).toHaveLength(0)
    remote.destroy()
  })
  test.each(['not json', 'null', '{}', JSON.stringify({ id: 1, type: 'document', data: '' }), JSON.stringify({ id: 'a', type: 'unknown', data: '' }), JSON.stringify({ id: 'a', type: 'document', data: '!!!', end: true }), 'x'.repeat(14001)])('closes malformed input %# without mutating document', async input => {
    const channel = await open(); channel.receive(input)
    expect(channel.readyState).toBe('closed'); expect(doc.getText('text').toString()).toBe('')
  })
  test('fragmented incoming state applies only after the final frame', async () => {
    const channel = await open(); const remote = new Y.Doc()
    remote.getText('text').insert(0, 'large remote state '.repeat(2000))
    const bytes = Y.encodeStateAsUpdate(remote)
    let binary = ''; for (const byte of bytes) binary += String.fromCharCode(byte)
    const data = btoa(binary)
    for (let i = 0; i < data.length; i += 12000) {
      const end = i + 12000 >= data.length
      channel.receive(JSON.stringify({ id: 'fragmented', type: 'document', data: data.slice(i, i + 12000), end }))
      if (!end) expect(doc.getText('text').toString()).toBe('')
    }
    expect(doc.getText('text').toString()).toBe(remote.getText('text').toString()); remote.destroy()
  })
  test('star topology forwards remote edits to other peers but not the sender', async () => {
    const first = await open(); await manual.finish(code('answer')); const second = await open()
    first.sent = []; second.sent = []
    const remote = new Y.Doc(); remote.getText('text').insert(0, 'forward this')
    const data = btoa(String.fromCharCode(...Y.encodeStateAsUpdate(remote)))
    first.receive(JSON.stringify({ id: 'relay', type: 'document', data, end: true }))
    expect(first.sent).toHaveLength(0); expect(second.sent).toHaveLength(1)
    expect(JSON.parse(second.sent[0]).type).toBe('document'); remote.destroy()
  })
  test('repeated incoming updates are idempotent', async () => {
    const channel = await open(); const remote = new Y.Doc(); remote.getText('text').insert(0, 'once')
    const frame = JSON.stringify({ id: 'duplicate', type: 'document', data: btoa(String.fromCharCode(...Y.encodeStateAsUpdate(remote))), end: true })
    channel.receive(frame); channel.receive(frame)
    expect(doc.getText('text').toString()).toBe('once'); expect(channel.readyState).toBe('open'); remote.destroy()
  })
  test('closed channel discards buffered updates', async () => {
    const channel = await open(); channel.sent = []; channel.bufferedAmount = 300000
    doc.getText('text').insert(0, 'queued'); channel.close(); channel.bufferedAmount = 0; channel.onbufferedamountlow?.()
    expect(channel.sent).toHaveLength(0)
  })
  test('binary frames are rejected', async () => {
    const channel = await open(); channel.receive(new ArrayBuffer(8)); expect(channel.readyState).toBe('closed')
  })
  test('too many incomplete messages close the channel', async () => {
    const channel = await open()
    for (let i = 0; i < 18; i++) channel.receive(JSON.stringify({ id: String(i), type: 'document', data: 'a', end: false }))
    expect(channel.readyState).toBe('closed')
  })
})
