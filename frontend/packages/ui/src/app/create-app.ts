// SPDX-License-Identifier: MIT

import type { CobblestoneApi, NoteInfo, VaultNode } from '@cobblestone/api'

import { createAsyncQueue } from '../lib/async-queue'
import { bindTreeDragDrop, canDropNoteOrFolder, type TreeDrag } from '../tree/drag'
import { getDomRefs } from '../ui/dom'
import { refreshIcons } from '../ui/icons'
import { NoteEditor } from '../editor/note-editor'
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
} from '../ui/render'
import { flattenTree, noteParentFolder, remapPath, stripLeadingHeading } from '../lib/utils'

const SEARCH_DEBOUNCE_MS = 200

type PendingDelete =
  | { kind: 'note'; slug: string; isActive: boolean }
  | { kind: 'folder'; path: string }

type PendingRename =
  | { kind: 'note'; slug: string }
  | { kind: 'folder'; path: string }

export function createApp(api: CobblestoneApi): void {
  const dom = getDomRefs()
  const ops = createAsyncQueue()

  let notes: NoteInfo[] = []
  let tree: VaultNode[] = []
  let expandedFolders = new Set<string>()
  let loadSeq = 0
  let searchSeq = 0
  let searchDebounce: ReturnType<typeof setTimeout> | null = null

  let pendingNoteFolder: string | null = null
  let pendingFolderParent: string | null = null
  let pendingRename: PendingRename | null = null
  let pendingDelete: PendingDelete | null = null

  const editor = new NoteEditor(api, dom, {
    onError: (msg) => showError(dom, msg),
    onSaved: (slug) => {
      renderEditorArea(renderCtx())
      void loadNotes().then(() => {
        if (editor.activeSlug === slug) {
          editor.graph = null
          void api.noteGraph(slug).then((g) => {
            if (editor.activeSlug === slug) {
              editor.graph = g
              renderRightPanel(renderCtx())
            }
          })
        }
      })
    },
    onEditorRefresh: () => refresh(),
    onPanelRefresh: () => renderRightPanel(renderCtx()),
    reloadTree: loadNotes,
    loadGraph: (slug) => api.noteGraph(slug),
  })

  function noteTitleFor(slug: string): string {
    return notes.find((n) => n.slug === slug)?.title ?? slug.split('/').pop() ?? slug
  }

  function folderNameFor(path: string): string {
    return path.split('/').pop() ?? path
  }

  function renderCtx(): RenderCtx {
    return {
      dom,
      activeSlug: editor.activeSlug,
      activeTitle: editor.activeTitle,
      activeContent: editor.activeContent,
      notes,
      tree,
      graph: editor.graph,
      expandedFolders,
      isEditing: editor.isEditing,
      isDirty: editor.isDirty,
      searchQuery: dom.searchEl.value.trim(),
      onOpenNote: (slug) => void openNote(slug),
      onToggleFolder: toggleFolder,
      onCreateNoteInFolder: (path) => openCreateNoteModal(path),
      onCreateFolderInFolder: (path) => openCreateFolderModal(path),
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
        if (editor.activeSlug) void moveNoteToFolder(editor.activeSlug, null)
      },
    }
  }

  function refresh(): void {
    renderNoteList(renderCtx())
    renderEditorArea(renderCtx())
    renderRightPanel(renderCtx())
  }

  function toggleFolder(path: string): void {
    if (expandedFolders.has(path)) expandedFolders.delete(path)
    else expandedFolders.add(path)
    renderNoteList(renderCtx())
  }

  async function loadNotes(): Promise<void> {
    const seq = ++loadSeq
    try {
      const nextTree = await api.listTree()
      if (seq !== loadSeq) return
      tree = nextTree
      notes = flattenTree(tree)
      renderNoteList(renderCtx())
    } catch (e) {
      if (seq !== loadSeq) return
      showError(dom, String(e))
    }
  }

  async function copyPath(text: string): Promise<void> {
    try {
      await navigator.clipboard.writeText(text)
    } catch (e) {
      showError(dom, String(e))
    }
  }

  function openCreateNoteModal(folder: string | null): void {
    pendingNoteFolder = folder
    if (folder) expandedFolders.add(folder)
    renderNoteList(renderCtx())
    openNoteModal(dom, folder)
  }

  function openCreateFolderModal(parent: string | null): void {
    pendingFolderParent = parent
    if (parent) expandedFolders.add(parent)
    renderNoteList(renderCtx())
    openFolderModal(dom, parent)
  }

  function openDeleteConfirm(
    target: { kind: 'note'; slug: string } | { kind: 'folder'; path: string },
  ): void {
    pendingDelete =
      target.kind === 'note'
        ? { kind: 'note', slug: target.slug, isActive: editor.isActive(target.slug) }
        : target
    openDeleteConfirmModal(
      dom,
      target.kind === 'note' ? 'Delete note?' : 'Delete folder?',
      target.kind === 'note'
        ? 'This will permanently remove the note from disk. This action cannot be undone.'
        : 'This will permanently delete the folder and everything inside it.',
    )
  }

  async function loadNote(slug: string): Promise<void> {
    if (!(await editor.flushUntilClean())) return
    if (editor.activeSlug === slug) return
    try {
      await editor.load(slug)
    } catch (e) {
      showError(dom, String(e))
    }
  }

  async function openNote(slug: string): Promise<void> {
    return ops.enqueue(() => loadNote(slug))
  }

  async function createNote(): Promise<void> {
    return ops.enqueue(async () => {
      const title =
        dom.noteTitleInputEl.value.trim() ||
        dom.searchEl.value.trim() ||
        `Note ${Date.now()}`
      const folder = pendingNoteFolder
      closeNoteModal(dom)
      pendingNoteFolder = null

      if (!(await editor.flushUntilClean())) return

      try {
        const slug = await api.createNote(title, folder)
        dom.searchEl.value = ''
        searchSeq += 1
        if (folder) expandedFolders.add(folder)
        await loadNotes()
        await loadNote(slug)
      } catch (e) {
        showError(dom, String(e))
      }
    })
  }

  async function createFolder(): Promise<void> {
    return ops.enqueue(async () => {
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
    })
  }

  async function moveNoteToFolder(slug: string, folder: string | null): Promise<void> {
    return ops.enqueue(async () => {
      if (folder !== null && noteParentFolder(slug) === folder) return
      if (folder === null && noteParentFolder(slug) === null) return

      const movingActive = editor.isActive(slug)
      if (movingActive && !(await editor.flush())) return

      const moveSlug = editor.resolveSlug(slug, movingActive)
      try {
        const newSlug = await api.moveNote(moveSlug, folder)
        if (folder) expandedFolders.add(folder)
        if (editor.activeSlug === moveSlug) editor.activeSlug = newSlug
        await loadNotes()
        if (editor.activeSlug === newSlug) {
          renderNoteList(renderCtx())
          renderEditorArea(renderCtx())
          renderRightPanel(renderCtx())
        }
      } catch (e) {
        showError(dom, String(e))
      }
    })
  }

  async function moveFolderToParent(
    folderPath: string,
    destParent: string | null,
  ): Promise<void> {
    return ops.enqueue(async () => {
      const payload: TreeDrag = { kind: 'folder', path: folderPath, el: document.body }
      if (!canDropNoteOrFolder(payload, destParent)) return

      const affectsActive =
        editor.activeSlug !== null &&
        (editor.activeSlug === folderPath ||
          editor.activeSlug.startsWith(`${folderPath}/`))
      if (affectsActive && !(await editor.flush())) return

      try {
        const newPath = await api.moveFolder(folderPath, destParent)
        if (destParent) expandedFolders.add(destParent)
        expandedFolders.add(newPath)
        expandedFolders = new Set(
          [...expandedFolders].map((p) => remapPath(p, folderPath, newPath)),
        )
        if (editor.activeSlug) {
          editor.activeSlug = remapPath(editor.activeSlug, folderPath, newPath)
        }
        await loadNotes()
        renderRightPanel(renderCtx())
      } catch (e) {
        showError(dom, String(e))
      }
    })
  }

  async function doDelete(): Promise<void> {
    return ops.enqueue(async () => {
      if (!pendingDelete) return
      if (!(await editor.flush())) return

      const target = pendingDelete
      pendingDelete = null
      closeDeleteConfirmModal(dom)

      try {
        if (target.kind === 'note') {
          const slug = editor.resolveSlug(target.slug, target.isActive)
          await api.deleteNote(slug)
          if (editor.activeSlug === slug) editor.clear()
        } else {
          await api.deleteFolder(target.path)
          if (
            editor.activeSlug !== null &&
            (editor.activeSlug === target.path ||
              editor.activeSlug.startsWith(`${target.path}/`))
          ) {
            editor.clear()
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
    })
  }

  async function confirmRename(): Promise<void> {
    return ops.enqueue(async () => {
      const name = dom.renameInputEl.value.trim()
      if (!name || !pendingRename) {
        dom.renameInputEl.focus()
        return
      }

      const target = pendingRename
      pendingRename = null
      closeRenameModal(dom)

      const renamingActiveNote =
        target.kind === 'note' && editor.isActive(target.slug)
      const renamingActiveFolder =
        target.kind === 'folder' &&
        editor.activeSlug !== null &&
        editor.activeSlug.startsWith(`${target.path}/`)

      if (renamingActiveNote || renamingActiveFolder) {
        if (!(await editor.flush())) {
          pendingRename = target
          openRenameModal(
            dom,
            target.kind === 'note' ? 'Rename note' : 'Rename folder',
            name,
          )
          return
        }
      }

      try {
        if (target.kind === 'note') {
          const noteSlug = editor.resolveSlug(target.slug, renamingActiveNote)
          const newSlug = await api.renameNote(noteSlug, name)
          if (renamingActiveNote || editor.activeSlug === noteSlug) {
            editor.activeSlug = newSlug
            editor.activeTitle = name
            editor.persistedTitle = name
            editor.isDirty = false
          }
          await loadNotes()
          if (editor.activeSlug === newSlug) refresh()
        } else {
          const newPath = await api.renameFolder(target.path, name)
          expandedFolders = new Set(
            [...expandedFolders].map((p) => remapPath(p, target.path, newPath)),
          )
          if (editor.activeSlug?.startsWith(`${target.path}/`)) {
            editor.activeSlug = remapPath(editor.activeSlug, target.path, newPath)
          }
          await loadNotes()
          refresh()
        }
      } catch (e) {
        showError(dom, String(e))
      }
    })
  }

  async function runSearch(): Promise<void> {
    const q = dom.searchEl.value.trim()
    if (!q) {
      searchSeq += 1
      await loadNotes()
      renderRightPanel(renderCtx())
      return
    }
    const seq = ++searchSeq
    try {
      const results = await api.searchNotes(q)
      if (seq !== searchSeq) return
      notes = results
      renderNoteList(renderCtx())
    } catch (e) {
      if (seq !== searchSeq) return
      showError(dom, String(e))
    }
  }

  function bindEvents(): void {
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
      openCreateNoteModal,
      openCreateFolderModal,
    )

    dom.searchEl.addEventListener('input', () => {
      if (searchDebounce !== null) clearTimeout(searchDebounce)
      searchDebounce = setTimeout(() => {
        searchDebounce = null
        void runSearch()
      }, SEARCH_DEBOUNCE_MS)
    })

    dom.newBtnEl.addEventListener('click', () => openCreateNoteModal(null))
    dom.newFolderBtnEl.addEventListener('click', () => openCreateFolderModal(null))

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
      editor.activeTitle = dom.noteTitleEl.value
      editor.markDirty()
    })

    dom.editorEl.addEventListener('input', () => {
      editor.activeContent = dom.editorEl.value
      editor.markDirty()
    })

    dom.toggleModeEl.addEventListener('click', () => {
      if (editor.isEditing) {
        editor.activeContent = stripLeadingHeading(dom.editorEl.value)
        dom.editorEl.value = editor.activeContent
      } else {
        editor.syncFromDom()
      }
      editor.isEditing = !editor.isEditing
      renderEditorArea(renderCtx())
    })

    dom.saveBtnEl.addEventListener('click', () => void editor.save())

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
        void editor.save()
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
      if (e.key !== 'Tab') return
      e.preventDefault()
      const start = dom.editorEl.selectionStart
      const end = dom.editorEl.selectionEnd
      dom.editorEl.value =
        dom.editorEl.value.slice(0, start) + '  ' + dom.editorEl.value.slice(end)
      dom.editorEl.selectionStart = dom.editorEl.selectionEnd = start + 2
      editor.activeContent = dom.editorEl.value
      editor.markDirty()
    })

    window.addEventListener('beforeunload', (e) => {
      if (editor.isDirty) e.preventDefault()
    })
  }

  bindEvents()
  refreshIcons()
  void loadNotes()
}
