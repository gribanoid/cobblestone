// SPDX-License-Identifier: GPL-3.0-or-later

import type { CobblestoneApi, NoteGraph, NoteInfo, VaultNode } from '@cobblestone/api'

import { bindTreeDragDrop, canDropNoteOrFolder, type TreeDrag } from './drag'
import { getDomRefs } from './dom'
import {
  closeFolderModal,
  closeNoteModal,
  openFolderModal,
  openNoteModal,
  renderEditorArea,
  renderNoteList,
  renderRightPanel,
  showError,
  toggleTheme,
  type RenderCtx,
} from './render'
import {
  contentWithTitle,
  flattenTree,
  noteParentFolder,
  remapPath,
} from './utils'

export function createApp(api: CobblestoneApi): void {
  const dom = getDomRefs()

  let notes: NoteInfo[] = []
  let tree: VaultNode[] = []
  let expandedFolders = new Set<string>()
  let activeSlug: string | null = null
  let activeTitle = ''
  let activeContent = ''
  let graph: NoteGraph | null = null
  let isEditing = false
  let isDirty = false
  let saveTimer: ReturnType<typeof setTimeout> | null = null

  function renderCtx(): RenderCtx {
    return {
      dom,
      activeSlug,
      activeTitle,
      activeContent,
      notes,
      tree,
      graph,
      expandedFolders,
      isEditing,
      isDirty,
      searchQuery: dom.searchEl.value.trim(),
      onOpenNote: (slug) => void openNote(slug),
      onToggleFolder: toggleFolder,
      onMoveNoteToRoot: () => {
        if (activeSlug) void moveNoteToFolder(activeSlug, null)
      },
    }
  }

  function refresh() {
    renderNoteList(renderCtx())
    renderEditorArea(renderCtx())
    renderRightPanel(renderCtx())
  }

  function toggleFolder(path: string) {
    if (expandedFolders.has(path)) expandedFolders.delete(path)
    else expandedFolders.add(path)
    renderNoteList(renderCtx())
  }

  async function loadNotes() {
    try {
      tree = await api.listTree()
      notes = flattenTree(tree)
      renderNoteList(renderCtx())
    } catch (e) {
      showError(dom, String(e))
    }
  }

  async function openNote(slug: string) {
    if (saveTimer !== null) {
      clearTimeout(saveTimer)
      saveTimer = null
    }
    if (isDirty && activeSlug !== null && activeSlug !== slug) {
      void saveNote()
    }
    try {
      const note = await api.getNote(slug)
      activeSlug = slug
      activeTitle = note.title
      activeContent = note.content
      isDirty = false
      isEditing = false
      graph = null

      dom.editorEl.value = activeContent
      refresh()

      api
        .noteGraph(slug)
        .then((g) => {
          if (activeSlug === slug) {
            graph = g
            renderRightPanel(renderCtx())
          }
        })
        .catch(() => {})
    } catch (e) {
      showError(dom, String(e))
    }
  }

  async function createNote() {
    const titleFromSearch = dom.searchEl.value.trim()
    const title =
      dom.noteTitleInputEl.value.trim() || titleFromSearch || `Note ${Date.now()}`
    closeNoteModal(dom)
    try {
      const slug = await api.createNote(title, null)
      dom.searchEl.value = ''
      await loadNotes()
      await openNote(slug)
      isEditing = true
      renderEditorArea(renderCtx())
    } catch (e) {
      showError(dom, String(e))
    }
  }

  async function moveFolderToParent(folderPath: string, destParent: string | null) {
    const payload: TreeDrag = { kind: 'folder', path: folderPath, el: document.body }
    if (!canDropNoteOrFolder(payload, destParent)) return
    try {
      const newPath = await api.moveFolder(folderPath, destParent)
      if (destParent) expandedFolders.add(destParent)
      expandedFolders.add(newPath)
      expandedFolders = new Set(
        [...expandedFolders].map((p) => remapPath(p, folderPath, newPath)),
      )
      if (activeSlug) {
        activeSlug = remapPath(activeSlug, folderPath, newPath)
      }
      await loadNotes()
      renderRightPanel(renderCtx())
    } catch (e) {
      showError(dom, String(e))
    }
  }

  async function moveNoteToFolder(slug: string, folder: string | null) {
    if (folder !== null && noteParentFolder(slug) === folder) return
    if (folder === null && noteParentFolder(slug) === null) return
    try {
      const newSlug = await api.moveNote(slug, folder)
      if (folder) expandedFolders.add(folder)
      if (activeSlug === slug) activeSlug = newSlug
      await loadNotes()
      if (activeSlug === newSlug) {
        renderNoteList(renderCtx())
        renderRightPanel(renderCtx())
      }
    } catch (e) {
      showError(dom, String(e))
    }
  }

  async function createFolder() {
    const name = dom.folderNameInputEl.value.trim()
    if (!name) {
      dom.folderNameInputEl.focus()
      return
    }
    closeFolderModal(dom)
    try {
      await api.createFolder(name)
      expandedFolders.add(name)
      await loadNotes()
    } catch (e) {
      showError(dom, String(e))
    }
  }

  async function saveNote() {
    if (!activeSlug) return
    if (saveTimer !== null) {
      clearTimeout(saveTimer)
      saveTimer = null
    }
    const slug = activeSlug
    const c = contentWithTitle(activeContent, activeTitle)
    try {
      await api.saveNote(slug, c)
      if (activeSlug === slug) {
        activeContent = c
        isDirty = false
        renderEditorArea(renderCtx())
        await loadNotes()
        api
          .noteGraph(slug)
          .then((g) => {
            if (activeSlug === slug) {
              graph = g
              renderRightPanel(renderCtx())
            }
          })
          .catch(() => {})
      }
    } catch (e) {
      showError(dom, String(e))
    }
  }

  async function doDelete() {
    if (!activeSlug) return
    if (saveTimer !== null) {
      clearTimeout(saveTimer)
      saveTimer = null
    }
    try {
      await api.deleteNote(activeSlug)
      activeSlug = null
      activeTitle = ''
      activeContent = ''
      graph = null
      isDirty = false
      refresh()
      await loadNotes()
    } catch (e) {
      showError(dom, String(e))
    }
  }

  function scheduleAutosave() {
    if (saveTimer !== null) clearTimeout(saveTimer)
    saveTimer = setTimeout(saveNote, 2000)
  }

  bindTreeDragDrop(dom, {
    canDrop: canDropNoteOrFolder,
    onDrop: (payload, destFolder) => {
      if (payload.kind === 'note') void moveNoteToFolder(payload.slug, destFolder)
      else void moveFolderToParent(payload.path, destFolder)
    },
  })

  dom.searchEl.addEventListener('input', async () => {
    const q = dom.searchEl.value.trim()
    if (!q) {
      await loadNotes()
      return
    }
    try {
      notes = await api.searchNotes(q)
      renderNoteList(renderCtx())
    } catch (e) {
      showError(dom, String(e))
    }
  })

  dom.newBtnEl.addEventListener('click', () => openNoteModal(dom))
  dom.newFolderBtnEl.addEventListener('click', () => openFolderModal(dom))
  dom.cancelNoteModalEl.addEventListener('click', () => closeNoteModal(dom))
  dom.confirmNoteModalEl.addEventListener('click', () => void createNote())
  dom.noteTitleInputEl.addEventListener('keydown', (e) => {
    if (e.key === 'Enter') {
      e.preventDefault()
      void createNote()
    }
  })
  dom.cancelFolderEl.addEventListener('click', () => closeFolderModal(dom))
  dom.confirmFolderEl.addEventListener('click', () => void createFolder())
  dom.folderNameInputEl.addEventListener('keydown', (e) => {
    if (e.key === 'Enter') {
      e.preventDefault()
      void createFolder()
    }
  })

  dom.noteTitleEl.addEventListener('input', () => {
    activeTitle = dom.noteTitleEl.value
    isDirty = true
    dom.saveIndicatorEl.textContent = 'unsaved'
    dom.saveIndicatorEl.className = 'edit-indicator unsaved'
    scheduleAutosave()
  })

  dom.editorEl.addEventListener('input', () => {
    activeContent = dom.editorEl.value
    isDirty = true
    dom.saveIndicatorEl.textContent = 'unsaved'
    dom.saveIndicatorEl.className = 'edit-indicator unsaved'
    scheduleAutosave()
  })

  dom.toggleModeEl.addEventListener('click', () => {
    isEditing = !isEditing
    renderEditorArea(renderCtx())
  })

  dom.saveBtnEl.addEventListener('click', () => void saveNote())
  dom.deleteBtnEl.addEventListener('click', () => {
    if (activeSlug) dom.confirmModalEl.style.display = 'flex'
  })
  dom.cancelDeleteEl.addEventListener('click', () => {
    dom.confirmModalEl.style.display = 'none'
  })
  dom.confirmDeleteEl.addEventListener('click', async () => {
    dom.confirmModalEl.style.display = 'none'
    await doDelete()
  })
  dom.themeBtnEl.addEventListener('click', toggleTheme)
  dom.errorToastEl.addEventListener('click', () => {
    dom.errorToastEl.style.display = 'none'
  })

  document.addEventListener('keydown', (e) => {
    if ((e.ctrlKey || e.metaKey) && e.key === 's') {
      e.preventDefault()
      void saveNote()
    }
    if (e.key === 'Escape') {
      if (dom.confirmModalEl.style.display !== 'none') dom.confirmModalEl.style.display = 'none'
      if (dom.folderModalEl.style.display !== 'none') closeFolderModal(dom)
      if (dom.noteModalEl.style.display !== 'none') closeNoteModal(dom)
    }
  })

  dom.editorEl.addEventListener('keydown', (e) => {
    if (e.key === 'Tab') {
      e.preventDefault()
      const start = dom.editorEl.selectionStart
      const end = dom.editorEl.selectionEnd
      dom.editorEl.value =
        dom.editorEl.value.slice(0, start) + '  ' + dom.editorEl.value.slice(end)
      dom.editorEl.selectionStart = dom.editorEl.selectionEnd = start + 2
      activeContent = dom.editorEl.value
      isDirty = true
      dom.saveIndicatorEl.textContent = 'unsaved'
      dom.saveIndicatorEl.className = 'edit-indicator unsaved'
      scheduleAutosave()
    }
  })

  loadNotes()
}
