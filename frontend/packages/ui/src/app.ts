// SPDX-License-Identifier: MIT

import type { CobblestoneApi, NoteGraph, NoteInfo, VaultNode } from '@cobblestone/api'

import { bindTreeDragDrop, canDropNoteOrFolder, type TreeDrag } from './drag'
import { getDomRefs } from './dom'
import { refreshIcons } from './icons'
import {
  closeFolderModal,
  closeNoteModal,
  closeFolderContextMenu,
  closeRenameModal,
  closeDeleteConfirmModal,
  bindVaultRootMenuHandlers,
  openDeleteConfirmModal,
  openFolderModal,
  openNoteModal,
  openRenameModal,
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
  stripLeadingHeading,
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
  let pendingNoteFolder: string | null = null
  let pendingFolderParent: string | null = null
  let pendingRename:
    | { kind: 'note'; slug: string }
    | { kind: 'folder'; path: string }
    | null = null
  let pendingDelete:
    | { kind: 'note'; slug: string }
    | { kind: 'folder'; path: string }
    | null = null

  function noteTitleFor(slug: string): string {
    return notes.find((n) => n.slug === slug)?.title ?? slug.split('/').pop() ?? slug
  }

  function folderNameFor(path: string): string {
    return path.split('/').pop() ?? path
  }

  async function copyPath(text: string) {
    try {
      await navigator.clipboard.writeText(text)
    } catch (e) {
      showError(dom, String(e))
    }
  }

  function clearActiveNote() {
    activeSlug = null
    activeTitle = ''
    activeContent = ''
    graph = null
    isDirty = false
  }

  function openDeleteConfirm(
    target: { kind: 'note'; slug: string } | { kind: 'folder'; path: string },
  ) {
    pendingDelete = target
    if (target.kind === 'note') {
      openDeleteConfirmModal(
        dom,
        'Delete note?',
        'This will permanently remove the note from disk. This action cannot be undone.',
      )
    } else {
      openDeleteConfirmModal(
        dom,
        'Delete folder?',
        'This will permanently delete the folder and everything inside it.',
      )
    }
  }

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
      onCreateNoteInFolder: (path) => {
        pendingNoteFolder = path
        if (path) expandedFolders.add(path)
        renderNoteList(renderCtx())
        openNoteModal(dom, path)
      },
      onCreateFolderInFolder: (path) => {
        pendingFolderParent = path
        if (path) expandedFolders.add(path)
        renderNoteList(renderCtx())
        openFolderModal(dom, path)
      },
      onCopyNote: (slug) => void copyPath(slug),
      onCopyFolder: (path) => void copyPath(path),
      onRenameNote: (slug) => {
        pendingRename = { kind: 'note', slug }
        openRenameModal(dom, 'Rename note', noteTitleFor(slug))
      },
      onRenameFolder: (path) => {
        pendingRename = { kind: 'folder', path }
        openRenameModal(dom, 'Rename folder', folderNameFor(path))
      },
      onDeleteNote: (slug) => openDeleteConfirm({ kind: 'note', slug }),
      onDeleteFolder: (path) => openDeleteConfirm({ kind: 'folder', path }),
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
      activeContent = stripLeadingHeading(note.content)
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
    const folder = pendingNoteFolder
    closeNoteModal(dom)
    pendingNoteFolder = null
    try {
      const slug = await api.createNote(title, folder)
      dom.searchEl.value = ''
      if (folder) expandedFolders.add(folder)
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
    const parent = pendingFolderParent
    pendingFolderParent = null
    closeFolderModal(dom)
    const path = parent ? `${parent}/${name}` : name
    try {
      await api.createFolder(path)
      if (parent) expandedFolders.add(parent)
      expandedFolders.add(path)
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
    const title = activeTitle.trim()
    if (!title) {
      showError(dom, 'Note title cannot be empty')
      return
    }

    let slug = activeSlug
    const listed = notes.find((n) => n.slug === slug)
    if (listed && listed.title !== title) {
      try {
        slug = await api.renameNote(slug, title)
        activeSlug = slug
        activeTitle = title
      } catch (e) {
        showError(dom, String(e))
        return
      }
    }

    const c = contentWithTitle(activeContent, title)
    try {
      await api.saveNote(slug, c)
      if (activeSlug === slug) {
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
    if (!pendingDelete) return
    const target = pendingDelete
    pendingDelete = null
    closeDeleteConfirmModal(dom)

    if (saveTimer !== null) {
      clearTimeout(saveTimer)
      saveTimer = null
    }

    try {
      if (target.kind === 'note') {
        await api.deleteNote(target.slug)
        if (activeSlug === target.slug) clearActiveNote()
      } else {
        await api.deleteFolder(target.path)
        if (
          activeSlug !== null &&
          (activeSlug === target.path || activeSlug.startsWith(`${target.path}/`))
        ) {
          clearActiveNote()
        }
        expandedFolders = new Set(
          [...expandedFolders].filter(
            (p) => p !== target.path && !p.startsWith(`${target.path}/`),
          ),
        )
      }
      refresh()
      await loadNotes()
    } catch (e) {
      showError(dom, String(e))
    }
  }

  async function confirmRename() {
    const name = dom.renameInputEl.value.trim()
    if (!name || !pendingRename) {
      dom.renameInputEl.focus()
      return
    }
    const target = pendingRename
    pendingRename = null
    closeRenameModal(dom)

    try {
      if (target.kind === 'note') {
        const newSlug = await api.renameNote(target.slug, name)
        if (activeSlug === target.slug) {
          activeSlug = newSlug
          activeTitle = name
        }
        await loadNotes()
        if (activeSlug === newSlug) refresh()
      } else {
        const newPath = await api.renameFolder(target.path, name)
        expandedFolders = new Set(
          [...expandedFolders].map((p) => remapPath(p, target.path, newPath)),
        )
        if (activeSlug !== null && activeSlug.startsWith(`${target.path}/`)) {
          activeSlug = remapPath(activeSlug, target.path, newPath)
        } else if (activeSlug === target.path) {
          activeSlug = newPath
        }
        await loadNotes()
        refresh()
      }
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

  bindVaultRootMenuHandlers(
    dom.noteListEl,
    () => !dom.searchEl.value.trim(),
    (path) => {
      pendingNoteFolder = path
      if (path) expandedFolders.add(path)
      renderNoteList(renderCtx())
      openNoteModal(dom, path)
    },
    (path) => {
      pendingFolderParent = path
      if (path) expandedFolders.add(path)
      renderNoteList(renderCtx())
      openFolderModal(dom, path)
    },
  )

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

  dom.newBtnEl.addEventListener('click', () => {
    pendingNoteFolder = null
    openNoteModal(dom)
  })
  dom.newFolderBtnEl.addEventListener('click', () => {
    pendingFolderParent = null
    openFolderModal(dom)
  })
  dom.cancelNoteModalEl.addEventListener('click', () => {
    pendingNoteFolder = null
    closeNoteModal(dom)
  })
  dom.confirmNoteModalEl.addEventListener('click', () => void createNote())
  dom.noteTitleInputEl.addEventListener('keydown', (e) => {
    if (e.key === 'Enter') {
      e.preventDefault()
      void createNote()
    }
  })
  dom.cancelFolderEl.addEventListener('click', () => {
    pendingFolderParent = null
    closeFolderModal(dom)
  })
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
    if (isEditing) {
      activeContent = stripLeadingHeading(dom.editorEl.value)
      dom.editorEl.value = activeContent
    }
    isEditing = !isEditing
    renderEditorArea(renderCtx())
  })

  dom.saveBtnEl.addEventListener('click', () => void saveNote())
  dom.cancelDeleteEl.addEventListener('click', () => {
    pendingDelete = null
    closeDeleteConfirmModal(dom)
  })
  dom.confirmDeleteEl.addEventListener('click', () => void doDelete())
  dom.cancelRenameEl.addEventListener('click', () => {
    pendingRename = null
    closeRenameModal(dom)
  })
  dom.confirmRenameEl.addEventListener('click', () => void confirmRename())
  dom.renameInputEl.addEventListener('keydown', (e) => {
    if (e.key === 'Enter') {
      e.preventDefault()
      void confirmRename()
    }
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
      closeFolderContextMenu()
      if (dom.confirmModalEl.style.display !== 'none') {
        pendingDelete = null
        closeDeleteConfirmModal(dom)
      }
      if (dom.renameModalEl.style.display !== 'none') {
        pendingRename = null
        closeRenameModal(dom)
      }
      if (dom.folderModalEl.style.display !== 'none') {
        pendingFolderParent = null
        closeFolderModal(dom)
      }
      if (dom.noteModalEl.style.display !== 'none') {
        pendingNoteFolder = null
        closeNoteModal(dom)
      }
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

  refreshIcons()
  loadNotes()
}
