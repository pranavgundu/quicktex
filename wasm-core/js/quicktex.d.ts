export class QuicktexError extends Error {
  position: number;
}

export function init(source?: URL | string | ArrayBuffer | ArrayBufferView | WebAssembly.Module): Promise<void>;

export function render(tex: string, display?: boolean): string;

export type ProjectFiles = Record<string, string | Uint8Array>;
export function renderDocument(tex: string, options?: { files?: ProjectFiles }): number;
export function setFiles(files: ProjectFiles): void;
export function registerFont(data: ArrayBuffer | ArrayBufferView, read?: (url: URL) => Promise<ArrayBuffer | ArrayBufferView> | ArrayBuffer | ArrayBufferView): Promise<void>;
export function loadFontsFor(tex: string, read?: (url: URL) => Promise<ArrayBuffer | ArrayBufferView> | ArrayBuffer | ArrayBufferView): Promise<void>;

export function documentPage(page: number): string;

export function documentPdf(): Uint8Array;

export function commandList(): string;

export function documentWarnings(): number[];

export function usePackage(name: string): void;
