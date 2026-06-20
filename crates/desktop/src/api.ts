// SPDX-License-Identifier: GPL-3.0-or-later

import { invoke } from '@tauri-apps/api/core'

export interface NoteInfo {
  slug: string
  title: string
  modified: string
  size: number
  preview: string
  tags: string[]
}

export interface NoteContent {
  slug: string
  title: string
  content: string
}

export interface LinkedNote {
  slug: string
  title: string
}

export interface NoteGraph {
  outgoing: LinkedNote[]
  backlinks: LinkedNote[]
}

export type VaultNode =
  | {
      kind: 'folder'
      name: string
      path: string
      children: VaultNode[]
    }
  | {
      kind: 'note'
      slug: string
      title: string
      modified: string
      size: number
      preview: string
      tags: string[]
    }

export const api = {
  listNotes: (): Promise<NoteInfo[]> =>
    invoke('list_notes'),

  listTree: (): Promise<VaultNode[]> =>
    invoke('list_tree'),

  getNote: (slug: string): Promise<NoteContent> =>
    invoke('get_note', { slug }),

  saveNote: (slug: string, content: string): Promise<void> =>
    invoke('save_note', { slug, content }),

  createNote: (title: string, folder?: string | null): Promise<string> =>
    folder
      ? invoke('create_note', { title, folder })
      : invoke('create_note', { title }),

  createFolder: (path: string): Promise<void> =>
    invoke('create_folder', { path }),

  moveNote: (slug: string, folder: string | null = null): Promise<string> =>
    invoke('move_note', { slug, folder }),

  moveFolder: (path: string, destParent: string | null = null): Promise<string> =>
    invoke('move_folder', { path, destParent: destParent }),

  deleteNote: (slug: string): Promise<void> =>
    invoke('delete_note', { slug }),

  searchNotes: (query: string): Promise<NoteInfo[]> =>
    invoke('search_notes', { query }),

  noteGraph: (slug: string): Promise<NoteGraph> =>
    invoke('note_graph', { slug }),
}
