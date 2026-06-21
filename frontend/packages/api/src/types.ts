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

export interface CobblestoneApi {
  listNotes(): Promise<NoteInfo[]>
  listTree(): Promise<VaultNode[]>
  getNote(slug: string): Promise<NoteContent>
  saveNote(slug: string, content: string): Promise<void>
  createNote(title: string, folder?: string | null): Promise<string>
  createFolder(path: string): Promise<void>
  moveNote(slug: string, folder?: string | null): Promise<string>
  moveFolder(path: string, destParent?: string | null): Promise<string>
  deleteNote(slug: string): Promise<void>
  searchNotes(query: string): Promise<NoteInfo[]>
  noteGraph(slug: string): Promise<NoteGraph>
}
