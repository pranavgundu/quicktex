import type { ProjectFiles } from '../../wasm-core/js/quicktex.js'

// Assets can be large: keep them in IndexedDB, never serialize them on the typing path.
function database(): Promise<IDBDatabase> {
  return new Promise((resolve, reject) => {
    const request = indexedDB.open('quicktex-workspace', 1)
    request.onupgradeneeded = () => request.result.createObjectStore('project')
    request.onsuccess = () => resolve(request.result)
    request.onerror = () => reject(request.error)
  })
}

export async function loadProject(): Promise<ProjectFiles> {
  const db = await database()
  try {
    return await new Promise((resolve, reject) => {
      const request = db.transaction('project').objectStore('project').get('files')
      request.onsuccess = () => resolve(request.result ?? {})
      request.onerror = () => reject(request.error)
    })
  } finally { db.close() }
}

export async function saveProject(files: ProjectFiles): Promise<void> {
  const db = await database()
  try {
    await new Promise<void>((resolve, reject) => {
      const transaction = db.transaction('project', 'readwrite')
      transaction.objectStore('project').put(files, 'files')
      transaction.oncomplete = () => resolve()
      transaction.onerror = () => reject(transaction.error)
      transaction.onabort = () => reject(transaction.error)
    })
  } finally { db.close() }
}

export function validPath(name: string) {
  return name.length > 0 && !name.startsWith('/') && !name.includes('\\') &&
    !name.split('/').some((part) => !part || part === '.' || part === '..')
}

export const textFile = (name: string) => /\.(tex|bib|sty|cls|txt|csv)$/i.test(name)
