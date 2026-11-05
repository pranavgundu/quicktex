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

function stage(tex) {
  const len = tex.length;
  reserve(len * 3);
  const u8 = view();
  let n = 0;
  if (len <= 512) {
    for (; n < len; n++) {
      const c = tex.charCodeAt(n);
      if (c > 127) break;
      u8[input + n] = c;
    }
  }
  if (n < len) n = encoder.encodeInto(tex, u8.subarray(input, input + capacity)).written;
  return n;
}

function fail(size) {
  const out = exports_.qt_output();
  throw new QuicktexError(decoder.decode(view().subarray(out, out - size)), exports_.qt_error_pos());
}

function text(size) {
  const out = exports_.qt_output();
  return decoder.decode(view().subarray(out, out + size));
}

export function render(tex, display = false) {
  const size = exports_.qt_render(stage(tex), display ? 1 : 0);
  if (size < 0) fail(size);
  return text(size);
}

export function renderDocument(tex) {
  const pages = exports_.qt_document(stage(tex));
  if (pages < 0) fail(pages);
  return pages;
}

export function commandList() {
  return text(exports_.qt_names());
}

export function documentWarnings() {
  const list = text(exports_.qt_warnings());
  return list ? list.trim().split('\n').map(Number) : [];
}

export function documentPage(page) {
  return text(exports_.qt_page(page));
}

export function documentPdf() {
  const size = exports_.qt_pdf();
  const out = exports_.qt_output();
  return view().slice(out, out + size);
}

export function usePackage(name) {
  const encoded = encoder.encode(name);
  reserve(encoded.length);
  view().set(encoded, input);
  if (exports_.qt_use(encoded.length) !== 0) {
    throw new QuicktexError(`unknown package ${name}`, 0);
  }
}
