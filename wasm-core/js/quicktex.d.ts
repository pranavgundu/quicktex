export class QuicktexError extends Error {
  position: number;
}

export function init(source?: URL | string | ArrayBuffer | ArrayBufferView | WebAssembly.Module): Promise<void>;

export function render(tex: string, display?: boolean): string;

export function renderDocument(tex: string): number;

export function documentPage(page: number): string;

export function documentPdf(): Uint8Array;

export function commandList(): string;

export function documentWarnings(): number[];

export function usePackage(name: string): void;
