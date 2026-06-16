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

export const api = {
  listNotes: (): Promise<NoteInfo[]> =>
    invoke('list_notes'),

  getNote: (slug: string): Promise<NoteContent> =>
    invoke('get_note', { slug }),

  saveNote: (slug: string, content: string): Promise<void> =>
    invoke('save_note', { slug, content }),

  createNote: (title: string): Promise<string> =>
    invoke('create_note', { title }),

  deleteNote: (slug: string): Promise<void> =>
    invoke('delete_note', { slug }),

  searchNotes: (query: string): Promise<NoteInfo[]> =>
    invoke('search_notes', { query }),

  noteGraph: (slug: string): Promise<NoteGraph> =>
    invoke('note_graph', { slug }),
}
