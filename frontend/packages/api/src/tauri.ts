import { invoke } from '@tauri-apps/api/core'

import type { CobblestoneApi } from './types'

export const tauriApi: CobblestoneApi = {
  listNotes: () => invoke('list_notes'),

  listTree: () => invoke('list_tree'),

  getNote: (slug) => invoke('get_note', { slug }),

  saveNote: (slug, content) => invoke('save_note', { slug, content }),

  createNote: (title, folder) =>
    folder != null
      ? invoke('create_note', { title, folder })
      : invoke('create_note', { title }),

  createFolder: (path) => invoke('create_folder', { path }),

  moveNote: (slug, folder = null) => invoke('move_note', { slug, folder }),

  moveFolder: (path, destParent = null) =>
    invoke('move_folder', { path, destParent }),

  deleteNote: (slug) => invoke('delete_note', { slug }),

  renameNote: (slug, title) => invoke('rename_note', { slug, title }),

  renameFolder: (path, name) => invoke('rename_folder', { path, name }),

  deleteFolder: (path) => invoke('delete_folder', { path }),

  searchNotes: (query) => invoke('search_notes', { query }),

  noteGraph: (slug) => invoke('note_graph', { slug }),
}
