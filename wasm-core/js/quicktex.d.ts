export class QuicktexError extends Error {
  position: number;
}

export function init(source?: URL | string | ArrayBuffer | ArrayBufferView | WebAssembly.Module): Promise<void>;

export function render(tex: string, display?: boolean): string;

export function usePackage(name: string): void;
