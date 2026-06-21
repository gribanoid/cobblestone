import type {
  CobblestoneApi,
  NoteContent,
  NoteGraph,
  NoteInfo,
  VaultNode,
} from './types'

async function api<T>(
  method: string,
  path: string,
  body?: unknown,
): Promise<T> {
  const opts: RequestInit = {
    method,
    headers: body !== undefined ? { 'Content-Type': 'application/json' } : {},
  }
  if (body !== undefined) opts.body = JSON.stringify(body)

  const r = await fetch(path, opts)
  if (!r.ok) {
    const text = await r.text()
    throw new Error(text || r.statusText)
  }
  const text = await r.text()
  if (!text) return undefined as T
  return JSON.parse(text) as T
}

export const webApi: CobblestoneApi = {
  listNotes: () => api<NoteInfo[]>('GET', '/api/notes'),

  listTree: () => api<VaultNode[]>('GET', '/api/tree'),

  getNote: (slug) => api<NoteContent>('GET', `/api/notes/${encodeURIComponent(slug)}`),

  saveNote: (slug, content) =>
    api<void>('POST', `/api/notes/${encodeURIComponent(slug)}`, { content }),

  createNote: (title, folder) =>
    api<{ slug: string }>('PUT', '/api/notes', { title, folder }).then((r) => r.slug),

  createFolder: (path) => api<void>('POST', '/api/folders', { path }),

  moveNote: (slug, folder = null) =>
    api<{ slug: string }>('POST', '/api/notes/move', { slug, folder }).then((r) => r.slug),

  moveFolder: (path, destParent = null) =>
    api<{ path: string }>('POST', '/api/folders/move', { path, destParent }).then(
      (r) => r.path,
    ),

  deleteNote: (slug) =>
    api<void>('DELETE', `/api/notes/${encodeURIComponent(slug)}`),

  searchNotes: (query) =>
    api<NoteInfo[]>('GET', `/api/search?query=${encodeURIComponent(query)}`),

  noteGraph: (slug) =>
    api<NoteGraph>('GET', `/api/notes/${encodeURIComponent(slug)}/graph`),
}
