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
  prepareGlyphs(tex);
  const size = exports_.qt_render(stage(tex), display ? 1 : 0);
  if (size < 0) fail(size);
  return text(size);
}

export function renderDocument(tex, options) {
  if (options?.files) setFiles(options.files);
  prepareGlyphs(tex);
  if (!projectGlyphsReady) {
    prepareGlyphs(projectText);
    projectGlyphsReady = true;
  }
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

/** Replace the virtual project filesystem. Nothing is fetched implicitly. */
export function setFiles(files) {
  exports_.qt_files_clear();
  projectText = Object.values(files).filter((value) => typeof value === 'string').join(' ');
  projectGlyphsReady = false;
  for (const [name, value] of Object.entries(files)) {
    const key = encoder.encode(name.replaceAll('\\', '/').replace(/^\.\//, ''));
    const data = typeof value === 'string' ? encoder.encode(value) : new Uint8Array(value.buffer, value.byteOffset, value.byteLength);
    reserve(key.length + data.length);
    view().set(key, input);
    view().set(data, input + key.length);
    if (exports_.qt_file(key.length, key.length + data.length) !== 0) throw new Error(`Invalid project file: ${name}`);
  }
}

let fontExports = null;
let fontReady = null;
const prepared = new Set();
let projectText = '';
let projectGlyphsReady = false;

async function initFonts(read) {
  if (fontExports) return;
  if (!fontReady) fontReady = (async () => {
    const url = new URL('./quicktex-fonts.wasm', import.meta.url);
    const data = await (read ? read(url) : fetch(url).then((response) => {
      if (!response.ok) throw new Error(`Unable to load font parser: ${response.status}`);
      return response.arrayBuffer();
    }));
    fontExports = (await WebAssembly.instantiate(data, {})).instance.exports;
  })().catch((error) => { fontReady = null; throw error; });
  await fontReady;
}

export async function registerFont(data, read) {
  await initFonts(read);
  const font = new Uint8Array(data.buffer ?? data, data.byteOffset ?? 0, data.byteLength);
  const pointer = fontExports.font_input(font.length);
  new Uint8Array(fontExports.memory.buffer).set(font, pointer);
  if (fontExports.font_load(font.length) !== 0) throw new Error('Invalid OpenType font');
  prepared.clear();
  projectGlyphsReady = false;
}

function prepareGlyphs(tex) {
  if (!fontExports || !/[^\x00-\x7f]/.test(tex)) return;
  for (const char of tex) {
    const cp = char.codePointAt(0);
    if (cp < 128 || prepared.has(cp)) continue;
    const len = fontExports.font_glyph(cp);
    if (len > 0) {
      reserve(len);
      view().set(new Uint8Array(fontExports.memory.buffer, fontExports.font_output(), len), input);
      exports_.qt_glyph(len);
    }
    prepared.add(cp);
  }
}

const fontLoads = new Map();
/** Load optional Noto outlines only for scripts that need them. The CLI supplies
 * a file loader; browsers fetch the bundled assets from the same origin. */
export async function loadFontsFor(tex, read = async (url) => {
  const response = await fetch(url);
  if (!response.ok) throw new Error(`Unable to load font: ${response.status}`);
  return response.arrayBuffer();
}) {
  const fonts = [];
  if (/[\u0370-\u052f\u1f00-\u1fff]/u.test(tex)) fonts.push(new URL('./fonts/NotoSerif-Regular.ttf', import.meta.url));
  if (/[\u2e80-\u9fff\uac00-\ud7ff\uf900-\ufaff\u{20000}-\u{3134f}]/u.test(tex)) fonts.push(new URL('./fonts/NotoSerifCJKsc-Regular.otf', import.meta.url));
  await Promise.all(fonts.map((url) => {
    const key = url.href;
    if (!fontLoads.has(key)) {
      fontLoads.set(key, Promise.resolve(read(url)).then((data) => registerFont(data, read)).catch((error) => { fontLoads.delete(key); throw error; }));
    }
    return fontLoads.get(key);
  }));
}
