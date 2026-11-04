const encoder = new TextEncoder();
const decoder = new TextDecoder();

let exports_ = null;
let bytes = null;
let input = 0;
let capacity = 0;

function view() {
  if (bytes === null || bytes.buffer !== exports_.memory.buffer) {
    bytes = new Uint8Array(exports_.memory.buffer);
  }
  return bytes;
}

export class QuicktexError extends Error {
  constructor(message, position) {
    super(message);
    this.name = 'QuicktexError';
    this.position = position;
  }
}

function reserve(size) {
  if (size <= capacity) return;
  capacity = Math.max(size, capacity * 2, 4096);
  input = exports_.qt_input(capacity);
}

export async function init(source) {
  if (exports_) return;
  const wasm = source ?? new URL('./quicktex.wasm', import.meta.url);
  let result;
  if (wasm instanceof WebAssembly.Module) {
    result = { instance: await WebAssembly.instantiate(wasm, {}) };
  } else if (wasm instanceof ArrayBuffer || ArrayBuffer.isView(wasm)) {
    result = await WebAssembly.instantiate(wasm, {});
  } else {
    result = await WebAssembly.instantiateStreaming(fetch(wasm), {});
  }
  exports_ = result.instance.exports;
  reserve(4096);
}

export function render(tex, display = false) {
  const len = tex.length;
  reserve(len * 3);
  let u8 = view();
  let n = 0;
  if (len <= 512) {
    for (; n < len; n++) {
      const c = tex.charCodeAt(n);
      if (c > 127) break;
      u8[input + n] = c;
    }
  }
  if (n < len) n = encoder.encodeInto(tex, u8.subarray(input, input + capacity)).written;
  const size = exports_.qt_render(n, display ? 1 : 0);
  const out = exports_.qt_output();
  u8 = view();
  if (size < 0) {
    throw new QuicktexError(decoder.decode(u8.subarray(out, out - size)), exports_.qt_error_pos());
  }
  return decoder.decode(u8.subarray(out, out + size));
}

export function usePackage(name) {
  const encoded = encoder.encode(name);
  reserve(encoded.length);
  view().set(encoded, input);
  if (exports_.qt_use(encoded.length) !== 0) {
    throw new QuicktexError(`unknown package ${name}`, 0);
  }
}
