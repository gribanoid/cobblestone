// SPDX-License-Identifier: MIT

import type { CobblestoneApi, NoteGraph } from '@cobblestone/api'

import type { DomRefs } from '../ui/dom'
import { createSaveQueue } from '../lib/async-queue'
import { contentWithTitle, stripLeadingHeading } from '../lib/utils'

export interface NoteEditorCallbacks {
  onError: (message: string) => void
  onSaved: (slug: string) => void
  onEditorRefresh: () => void
  onPanelRefresh: () => void
  reloadTree: () => Promise<void>
  loadGraph: (slug: string) => Promise<NoteGraph>
}

export class NoteEditor {
  activeSlug: string | null = null
  activeTitle = ''
  activeContent = ''
  persistedTitle = ''
  graph: NoteGraph | null = null
  isEditing = false
  isDirty = false

  private saveTimer: ReturnType<typeof setTimeout> | null = null
  private readonly saveQueue = createSaveQueue()

  constructor(
    private readonly api: CobblestoneApi,
    private readonly dom: DomRefs,
    private readonly cb: NoteEditorCallbacks,
  ) {}

  isActive(slug: string): boolean {
    return this.activeSlug === slug
  }

  /** Slug to use after flush when the note was active at the start of an operation. */
  resolveSlug(originalSlug: string, wasActive: boolean): string {
    return wasActive ? (this.activeSlug ?? originalSlug) : originalSlug
  }

  syncFromDom(): void {
    if (this.activeSlug !== null) {
      this.activeTitle = this.dom.noteTitleEl.value
    }
    if (this.isEditing) {
      this.activeContent = stripLeadingHeading(this.dom.editorEl.value)
    }
  }

  markDirty(): void {
    this.isDirty = true
    this.dom.saveIndicatorEl.textContent = 'unsaved'
    this.dom.saveIndicatorEl.className = 'edit-indicator unsaved'
    this.scheduleAutosave()
  }

  clear(): void {
    this.activeSlug = null
    this.activeTitle = ''
    this.activeContent = ''
    this.persistedTitle = ''
    this.graph = null
    this.isDirty = false
  }

  async load(slug: string): Promise<void> {
    const note = await this.api.getNote(slug)
    this.activeSlug = slug
    this.activeTitle = note.title
    this.persistedTitle = note.title
    this.activeContent = stripLeadingHeading(note.content)
    this.isDirty = false
    this.isEditing = true
    this.graph = null
    this.dom.editorEl.value = this.activeContent
    this.cb.onEditorRefresh()
    void this.refreshGraph(slug)
  }

  async flush(): Promise<boolean> {
    this.clearSaveTimer()
    if (this.isDirty && this.activeSlug !== null) {
      return this.save()
    }
    return true
  }

  async flushUntilClean(maxAttempts = 3): Promise<boolean> {
    for (let i = 0; i < maxAttempts; i++) {
      const saved = await this.flush()
      if (!saved) return false
      if (!this.isDirty) return true
    }
    return !this.isDirty
  }

  save(): Promise<boolean> {
    return this.saveQueue.enqueue(() => this.saveOnce())
  }

  private async saveOnce(): Promise<boolean> {
    this.clearSaveTimer()
    if (!this.activeSlug) return true

    this.syncFromDom()
    if (!this.isDirty) return true

    const slugAtStart = this.activeSlug
    const titleAtStart = this.persistedTitle
    const title = this.activeTitle.trim()
    if (!title) {
      this.cb.onError('Note title cannot be empty')
      return false
    }

    let slug = slugAtStart
    const content = this.activeContent

    if (titleAtStart !== title) {
      try {
        slug = await this.api.renameNote(slugAtStart, title)
        if (this.activeSlug === slugAtStart) {
          this.activeSlug = slug
          this.activeTitle = title
        }
      } catch (e) {
        this.cb.onError(String(e))
        return false
      }
    }

    try {
      await this.api.saveNote(slug, contentWithTitle(content, title))
      if (this.activeSlug === slug) {
        this.activeContent = content
        this.persistedTitle = title
        this.isDirty = false
        this.cb.onSaved(slug)
      }
      return true
    } catch (e) {
      this.cb.onError(String(e))
      return false
    }
  }

  private scheduleAutosave(): void {
    this.clearSaveTimer()
    this.saveTimer = setTimeout(() => void this.save(), 2000)
  }

  private clearSaveTimer(): void {
    if (this.saveTimer !== null) {
      clearTimeout(this.saveTimer)
      this.saveTimer = null
    }
  }

  private async refreshGraph(slug: string): Promise<void> {
    try {
      const g = await this.cb.loadGraph(slug)
      if (this.activeSlug === slug) {
        this.graph = g
        this.cb.onPanelRefresh()
      }
    } catch {
      // graph is optional
    }
  }
}
