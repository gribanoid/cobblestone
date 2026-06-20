// SPDX-License-Identifier: GPL-3.0-or-later

import { marked } from 'marked'
import { api, NoteInfo, NoteGraph, VaultNode } from './api'

// ── Markdown setup ────────────────────────────────────────────────────────

marked.use({ gfm: true, breaks: false })

// ── State ─────────────────────────────────────────────────────────────────

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
let suppressTreeClick = false
let activeDropEl: HTMLElement | null = null
const DRAG_THRESHOLD_PX = 5

// ── DOM helpers ───────────────────────────────────────────────────────────

const el = <T extends HTMLElement>(id: string) => document.getElementById(id) as T

const noteListEl     = el<HTMLDivElement>('note-list')
const searchEl       = el<HTMLInputElement>('search')
const newBtnEl       = el<HTMLButtonElement>('new-btn')
const newFolderBtnEl = el<HTMLButtonElement>('new-folder-btn')
const welcomeEl      = el<HTMLDivElement>('welcome')
const editorAreaEl   = el<HTMLDivElement>('editor-area')
const noteTitleEl    = el<HTMLInputElement>('note-title')
const editorEl       = el<HTMLTextAreaElement>('editor')
const previewEl      = el<HTMLDivElement>('preview')
const saveIndicatorEl= el<HTMLSpanElement>('save-indicator')
const toggleModeEl   = el<HTMLButtonElement>('toggle-mode-btn')
const saveBtnEl      = el<HTMLButtonElement>('save-btn')
const deleteBtnEl    = el<HTMLButtonElement>('delete-btn')
const themeBtnEl     = el<HTMLButtonElement>('theme-btn')
const confirmModalEl = el<HTMLDivElement>('confirm-modal')
const cancelDeleteEl = el<HTMLButtonElement>('cancel-delete-btn')
const confirmDeleteEl= el<HTMLButtonElement>('confirm-delete-btn')
const folderModalEl  = el<HTMLDivElement>('folder-modal')
const folderModalHintEl = el<HTMLParagraphElement>('folder-modal-hint')
const folderNameInputEl = el<HTMLInputElement>('folder-name-input')
const cancelFolderEl = el<HTMLButtonElement>('cancel-folder-btn')
const confirmFolderEl= el<HTMLButtonElement>('confirm-folder-btn')
const noteModalEl    = el<HTMLDivElement>('note-modal')
const noteModalHintEl= el<HTMLParagraphElement>('note-modal-hint')
const noteTitleInputEl = el<HTMLInputElement>('note-title-input')
const cancelNoteModalEl = el<HTMLButtonElement>('cancel-note-btn')
const confirmNoteModalEl= el<HTMLButtonElement>('confirm-note-btn')
const errorToastEl   = el<HTMLDivElement>('error-toast')
const panelContentEl = el<HTMLDivElement>('panel-content')

function escHtml(s: string): string {
  return s
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;')
}

function folderHint(): string {
  return 'Root'
}

function noteParentFolder(slug: string): string | null {
  const idx = slug.lastIndexOf('/')
  return idx >= 0 ? slug.slice(0, idx) : null
}

function toggleFolder(path: string) {
  if (expandedFolders.has(path)) expandedFolders.delete(path)
  else expandedFolders.add(path)
  renderVaultTree()
}

function moveSectionHtml(): string {
  if (!activeSlug || noteParentFolder(activeSlug) === null) return ''
  return `<section class="panel-section">
      <h3>Move</h3>
      <div class="panel-link-list">
        <button type="button" class="panel-link" id="move-note-to-root-btn">Move note to root</button>
      </div>
    </section>`
}

// ── Render ────────────────────────────────────────────────────────────────

function flattenTree(nodes: VaultNode[]): NoteInfo[] {
  const out: NoteInfo[] = []
  for (const node of nodes) {
    if (node.kind === 'note') {
      out.push({
        slug: node.slug,
        title: node.title,
        modified: node.modified,
        size: node.size,
        preview: node.preview,
        tags: node.tags,
      })
    } else {
      out.push(...flattenTree(node.children))
    }
  }
  return out
}

function renderSearchResults(items: NoteInfo[]) {
  if (items.length === 0) {
    noteListEl.innerHTML =
      '<div class="empty-state">No matching notes.</div>'
    return
  }
  noteListEl.innerHTML = items
    .map(
      (n) => `
      <div class="note-item${n.slug === activeSlug ? ' active' : ''}" data-slug="${escHtml(n.slug)}">
        <div class="note-item-title">${escHtml(n.title)}</div>
        <div class="note-item-meta">${escHtml(n.modified)} · ${escHtml(n.slug)}</div>
        ${n.tags.length > 0
          ? `<div class="note-item-tags">${n.tags.map((t) => `<span class="tag">#${escHtml(t)}</span>`).join('')}</div>`
          : ''}
      </div>`,
    )
    .join('')

  noteListEl.querySelectorAll<HTMLDivElement>('.note-item').forEach((div) => {
    div.addEventListener('click', () => openNote(div.dataset.slug!))
  })
}

function renderTreeNode(node: VaultNode): string {
  if (node.kind === 'note') {
    const active = node.slug === activeSlug ? ' active' : ''
    return `
      <div class="tree-note note-item${active}" data-slug="${escHtml(node.slug)}">
        <span class="tree-spacer"></span>
        <span class="tree-note-title">${escHtml(node.title)}</span>
      </div>`
  }

  const expanded = expandedFolders.has(node.path)
  const children = expanded
    ? `<div class="tree-children">${node.children.map((c) => renderTreeNode(c)).join('')}</div>`
    : ''

  return `
    <div class="tree-folder-wrap" data-drop-folder="${escHtml(node.path)}">
      <div class="tree-folder${expanded ? ' expanded' : ''}" data-path="${escHtml(node.path)}">
        <span class="tree-chevron" aria-hidden="true">${expanded ? '▾' : '▸'}</span>
        <span class="tree-folder-icon"></span>
        <span class="tree-folder-name">${escHtml(node.name)}</span>
      </div>
      ${children}
    </div>`
}

function folderDestPath(from: string, destParent: string | null): string {
  const name = from.split('/').pop()!
  return destParent ? `${destParent}/${name}` : name
}

function remapPath(path: string, from: string, to: string): string {
  if (path === from) return to
  if (path.startsWith(`${from}/`)) return to + path.slice(from.length)
  return path
}

type TreeDrag =
  | { kind: 'note'; slug: string; el: HTMLElement }
  | { kind: 'folder'; path: string; el: HTMLElement }

function canDrop(payload: TreeDrag, destFolder: string | null): boolean {
  if (destFolder === null) {
    if (payload.kind === 'note') {
      return noteParentFolder(payload.slug) !== null
    }
    return payload.path.includes('/')
  }
  if (payload.kind === 'note') {
    return noteParentFolder(payload.slug) !== destFolder
  }
  const from = payload.path
  const newPath = folderDestPath(from, destFolder)
  if (newPath === from) return false
  if (destFolder === from) return false
  if (destFolder.startsWith(`${from}/`)) return false
  return true
}

function executeDrop(payload: TreeDrag, destFolder: string | null) {
  if (!canDrop(payload, destFolder)) return
  if (payload.kind === 'note') {
    void moveNoteToFolder(payload.slug, destFolder)
  } else {
    void moveFolderToParent(payload.path, destFolder)
  }
}

function startTreeDrag(payload: TreeDrag, startX: number, startY: number) {
  const dragEl = payload.el
  let dragging = false

  const onMove = (ev: PointerEvent) => {
    if (!dragging) {
      const dx = ev.clientX - startX
      const dy = ev.clientY - startY
      if (dx * dx + dy * dy < DRAG_THRESHOLD_PX * DRAG_THRESHOLD_PX) return
      dragging = true
      dragEl.classList.add('dragging')
      dragEl.style.pointerEvents = 'none'
    }
    ev.preventDefault()
    const target = findDropTarget(ev.clientX, ev.clientY)
    if (target && canDrop(payload, target.folder)) {
      setDropHighlight(target.el)
    } else {
      setDropHighlight(null)
    }
  }

  const onUp = (ev: PointerEvent) => {
    document.removeEventListener('pointermove', onMove)
    document.removeEventListener('pointerup', onUp)
    document.removeEventListener('pointercancel', onUp)

    dragEl.style.pointerEvents = ''
    if (dragging) {
      dragEl.classList.remove('dragging')
      const target = findDropTarget(ev.clientX, ev.clientY)
      clearDropHighlight()
      if (target && canDrop(payload, target.folder)) {
        executeDrop(payload, target.folder)
      }
      suppressTreeClick = true
    } else {
      clearDropHighlight()
    }
  }

  document.addEventListener('pointermove', onMove)
  document.addEventListener('pointerup', onUp)
  document.addEventListener('pointercancel', onUp)
}

function clearDropHighlight() {
  if (activeDropEl) {
    activeDropEl.classList.remove('drag-over')
    activeDropEl = null
  }
}

function setDropHighlight(el: HTMLElement | null) {
  if (activeDropEl === el) return
  clearDropHighlight()
  if (el) {
    el.classList.add('drag-over')
    activeDropEl = el
  }
}

function findDropTarget(clientX: number, clientY: number): { el: HTMLElement; folder: string | null } | null {
  const elements = document.elementsFromPoint(clientX, clientY)
  for (const el of elements) {
    const wrap = (el as HTMLElement).closest('[data-drop-folder]') as HTMLElement | null
    if (!wrap || !noteListEl.contains(wrap)) continue
    const path = wrap.dataset.dropFolder ?? ''
    if (path.length > 0) return { el: wrap, folder: path }
  }
  for (const el of elements) {
    const html = el as HTMLElement
    if (!noteListEl.contains(html)) continue
    if (html.closest('.tree-note, .tree-folder')) continue
    const root = html.closest('#note-list, .vault-tree') as HTMLElement | null
    if (root) return { el: root, folder: null }
  }
  return null
}

function bindTreeDragDrop() {
  if (noteListEl.dataset.dragBound === '1') return
  noteListEl.dataset.dragBound = '1'

  noteListEl.addEventListener(
    'click',
    (e) => {
      if (suppressTreeClick) {
        suppressTreeClick = false
        e.preventDefault()
        e.stopPropagation()
      }
    },
    true,
  )

  noteListEl.addEventListener('pointerdown', (e) => {
    if (e.button !== 0) return

    const noteEl = (e.target as HTMLElement).closest('.tree-note') as HTMLElement | null
    const folderEl = (e.target as HTMLElement).closest('.tree-folder') as HTMLElement | null

    let payload: TreeDrag | null = null
    if (noteEl) {
      payload = { kind: 'note', slug: noteEl.dataset.slug!, el: noteEl }
    } else if (folderEl) {
      payload = { kind: 'folder', path: folderEl.dataset.path!, el: folderEl }
    }
    if (!payload) return

    startTreeDrag(payload, e.clientX, e.clientY)
  })
}

function renderVaultTree() {
  const items = tree.map((n) => renderTreeNode(n)).join('')
  const emptyHint = tree.length === 0
    ? '<div class="empty-state tree-empty-hint">No folders yet.<br>Use the folder icon above to start.</div>'
    : ''

  noteListEl.innerHTML = `
    <div class="vault-tree">
      ${items}
      ${emptyHint}
    </div>`

  noteListEl.querySelectorAll<HTMLDivElement>('.tree-folder').forEach((div) => {
    div.addEventListener('click', () => {
      toggleFolder(div.dataset.path!)
    })
  })

  noteListEl.querySelectorAll<HTMLDivElement>('.tree-note').forEach((div) => {
    const slug = div.dataset.slug!
    div.addEventListener('click', () => openNote(slug))
  })
}

function renderNoteList(items: NoteInfo[]) {
  const q = searchEl.value.trim()
  if (q) {
    renderSearchResults(items)
    return
  }
  renderVaultTree()
}

function renderEditorArea() {
  const open = activeSlug !== null
  welcomeEl.style.display = open ? 'none' : 'flex'
  editorAreaEl.style.display = open ? 'flex' : 'none'

  if (!open) return

  // Don't reset inputs while the user is actively typing in them
  if (document.activeElement !== noteTitleEl) {
    noteTitleEl.value = activeTitle
  }
  if (document.activeElement !== editorEl) {
    editorEl.value = activeContent
  }

  if (isDirty) {
    saveIndicatorEl.textContent = 'unsaved'
    saveIndicatorEl.className = 'edit-indicator unsaved'
  } else {
    saveIndicatorEl.textContent = 'saved'
    saveIndicatorEl.className = 'edit-indicator'
  }

  if (isEditing) {
    editorEl.classList.remove('hidden')
    previewEl.classList.add('hidden')
    toggleModeEl.textContent = 'Preview'
    toggleModeEl.className = 'tb-btn'
  } else {
    editorEl.classList.add('hidden')
    previewEl.classList.remove('hidden')
    previewEl.innerHTML = marked.parse(activeContent) as string
    toggleModeEl.textContent = 'Edit'
    toggleModeEl.className = 'tb-btn active'
  }
}

function renderRightPanel() {
  if (!activeSlug) {
    panelContentEl.innerHTML = `
      <div class="panel-empty">
        <h3>Note info</h3>
        <p>Open a note to see tags, metadata, wikilinks, and backlinks.</p>
      </div>`
    return
  }

  const info = notes.find((n) => n.slug === activeSlug)
  const g: NoteGraph = graph ?? { outgoing: [], backlinks: [] }

  panelContentEl.innerHTML = `
    <section class="panel-section">
      <h3>Metadata</h3>
      <dl class="meta-list">
        <div><dt>Modified</dt><dd>${escHtml(info?.modified ?? '')}</dd></div>
        <div><dt>Size</dt><dd>${info?.size ?? 0} B</dd></div>
        <div><dt>Slug</dt><dd>${escHtml(activeSlug)}</dd></div>
      </dl>
    </section>
    <section class="panel-section">
      <h3>Tags</h3>
      ${info?.tags.length
        ? `<div class="panel-tags">${info.tags.map((t) => `<span class="tag">#${escHtml(t)}</span>`).join('')}</div>`
        : '<p class="panel-muted">No tags yet</p>'}
    </section>
    ${linkSection('Outgoing links', 'No wikilinks found', g.outgoing)}
    ${linkSection('Backlinks', 'No backlinks yet', g.backlinks)}
    ${moveSectionHtml()}`

  panelContentEl.querySelectorAll<HTMLButtonElement>('[data-slug]').forEach((btn) => {
    btn.addEventListener('click', () => openNote(btn.dataset.slug!))
  })

  bindMoveButtons()
}

function bindMoveButtons() {
  document.getElementById('move-note-to-root-btn')?.addEventListener('click', () => {
    if (activeSlug) void moveNoteToFolder(activeSlug, null)
  })
}

function linkSection(
  title: string,
  empty: string,
  links: { slug: string; title: string }[],
): string {
  return `
    <section class="panel-section">
      <h3>${title}</h3>
      ${links.length === 0
        ? `<p class="panel-muted">${empty}</p>`
        : `<div class="panel-link-list">${links
            .map(
              (l) =>
                `<button class="panel-link" data-slug="${escHtml(l.slug)}">${escHtml(l.title)}</button>`,
            )
            .join('')}</div>`}
    </section>`
}

// ── Actions ───────────────────────────────────────────────────────────────

async function loadNotes() {
  try {
    tree = await api.listTree()
    notes = flattenTree(tree)
    renderNoteList(notes)
  } catch (e) {
    showError(String(e))
  }
}

async function openNote(slug: string) {
  if (saveTimer !== null) {
    clearTimeout(saveTimer)
    saveTimer = null
  }
  // Persist unsaved changes in the background before switching notes
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

    editorEl.value = activeContent
    renderNoteList(notes)
    renderEditorArea()
    renderRightPanel()

    api
      .noteGraph(slug)
      .then((g) => {
        // Only apply if user hasn't switched away
        if (activeSlug === slug) {
          graph = g
          renderRightPanel()
        }
      })
      .catch(() => {})
  } catch (e) {
    showError(String(e))
  }
}

async function createNote() {
  const titleFromSearch = searchEl.value.trim()
  const title = noteTitleInputEl.value.trim() || titleFromSearch || `Note ${Date.now()}`
  closeNoteModal()
  try {
    const slug = await api.createNote(title, null)
    searchEl.value = ''
    await loadNotes()
    await openNote(slug)
    isEditing = true
    renderEditorArea()
  } catch (e) {
    showError(String(e))
  }
}

async function moveFolderToParent(folderPath: string, destParent: string | null) {
  if (!canDrop({ kind: 'folder', path: folderPath, el: document.body }, destParent)) return
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
    renderRightPanel()
  } catch (e) {
    showError(String(e))
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
      renderNoteList(notes)
      renderRightPanel()
    }
  } catch (e) {
    showError(String(e))
  }
}

function openNoteModal() {
  const titleFromSearch = searchEl.value.trim()
  noteTitleInputEl.value = titleFromSearch
  noteModalHintEl.textContent = folderHint()
  noteModalEl.style.display = 'flex'
  noteTitleInputEl.focus()
  noteTitleInputEl.select()
}

function closeNoteModal() {
  noteModalEl.style.display = 'none'
}

async function createFolder() {
  const name = folderNameInputEl.value.trim()
  if (!name) {
    folderNameInputEl.focus()
    return
  }
  const path = name
  closeFolderModal()
  try {
    await api.createFolder(path)
    expandedFolders.add(path)
    await loadNotes()
  } catch (e) {
    showError(String(e))
  }
}

function openFolderModal() {
  folderNameInputEl.value = ''
  folderModalHintEl.textContent = folderHint()
  folderModalEl.style.display = 'flex'
  folderNameInputEl.focus()
}

function closeFolderModal() {
  folderModalEl.style.display = 'none'
}

function contentWithTitle(content: string, title: string): string {
  const t = title.trim()
  if (!t) return content
  const lines = content.split('\n')
  if (lines[0]?.startsWith('# ')) {
    const rest = lines.slice(1).join('\n')
    return rest ? `# ${t}\n${rest}` : `# ${t}`
  }
  return `# ${t}\n\n${content}`
}

async function saveNote() {
  if (!activeSlug) return
  if (saveTimer !== null) {
    clearTimeout(saveTimer)
    saveTimer = null
  }
  // Capture slug and content at save time — user may switch notes during the await
  const slug = activeSlug
  const c = contentWithTitle(activeContent, activeTitle)
  try {
    await api.saveNote(slug, c)
    // Only update UI and reload list if user is still on the same note
    if (activeSlug === slug) {
      activeContent = c
      isDirty = false
      renderEditorArea()
      await loadNotes()
      api
        .noteGraph(slug)
        .then((g) => {
          if (activeSlug === slug) {
            graph = g
            renderRightPanel()
          }
        })
        .catch(() => {})
    }
  } catch (e) {
    showError(String(e))
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
    renderEditorArea()
    renderRightPanel()
    await loadNotes()
  } catch (e) {
    showError(String(e))
  }
}

function scheduleAutosave() {
  if (saveTimer !== null) clearTimeout(saveTimer)
  saveTimer = setTimeout(saveNote, 2000)
}

// ── Error toast ───────────────────────────────────────────────────────────

function showError(msg: string) {
  errorToastEl.textContent = `⚠ ${msg}`
  errorToastEl.style.display = 'block'
  setTimeout(() => {
    errorToastEl.style.display = 'none'
  }, 5000)
}

// ── Theme ─────────────────────────────────────────────────────────────────

function toggleTheme() {
  const html = document.documentElement
  const next = html.dataset.theme === 'dark' ? 'light' : 'dark'
  html.dataset.theme = next
  localStorage.setItem('cb-theme', next)
}

// ── Event binding ─────────────────────────────────────────────────────────

searchEl.addEventListener('input', async () => {
  const q = searchEl.value.trim()
  if (!q) {
    await loadNotes()
    return
  }
  try {
    notes = await api.searchNotes(q)
    renderNoteList(notes)
  } catch (e) {
    showError(String(e))
  }
})

newBtnEl.addEventListener('click', openNoteModal)
newFolderBtnEl.addEventListener('click', openFolderModal)

cancelNoteModalEl.addEventListener('click', closeNoteModal)
confirmNoteModalEl.addEventListener('click', () => void createNote())
noteTitleInputEl.addEventListener('keydown', (e) => {
  if (e.key === 'Enter') {
    e.preventDefault()
    void createNote()
  }
})

cancelFolderEl.addEventListener('click', closeFolderModal)
confirmFolderEl.addEventListener('click', () => void createFolder())
folderNameInputEl.addEventListener('keydown', (e) => {
  if (e.key === 'Enter') {
    e.preventDefault()
    void createFolder()
  }
})

noteTitleEl.addEventListener('input', () => {
  activeTitle = noteTitleEl.value
  isDirty = true
  saveIndicatorEl.textContent = 'unsaved'
  saveIndicatorEl.className = 'edit-indicator unsaved'
  scheduleAutosave()
})

editorEl.addEventListener('input', () => {
  activeContent = editorEl.value
  isDirty = true
  saveIndicatorEl.textContent = 'unsaved'
  saveIndicatorEl.className = 'edit-indicator unsaved'
  scheduleAutosave()
})

toggleModeEl.addEventListener('click', () => {
  isEditing = !isEditing
  renderEditorArea()
})

saveBtnEl.addEventListener('click', saveNote)

deleteBtnEl.addEventListener('click', () => {
  if (activeSlug) confirmModalEl.style.display = 'flex'
})

cancelDeleteEl.addEventListener('click', () => {
  confirmModalEl.style.display = 'none'
})

confirmDeleteEl.addEventListener('click', async () => {
  confirmModalEl.style.display = 'none'
  await doDelete()
})

themeBtnEl.addEventListener('click', toggleTheme)

errorToastEl.addEventListener('click', () => {
  errorToastEl.style.display = 'none'
})

document.addEventListener('keydown', (e) => {
  if ((e.ctrlKey || e.metaKey) && e.key === 's') {
    e.preventDefault()
    saveNote()
  }
  if (e.key === 'Escape') {
    if (confirmModalEl.style.display !== 'none') confirmModalEl.style.display = 'none'
    if (folderModalEl.style.display !== 'none') closeFolderModal()
    if (noteModalEl.style.display !== 'none') closeNoteModal()
  }
})

editorEl.addEventListener('keydown', (e) => {
  if (e.key === 'Tab') {
    e.preventDefault()
    const start = editorEl.selectionStart
    const end = editorEl.selectionEnd
    editorEl.value = editorEl.value.slice(0, start) + '  ' + editorEl.value.slice(end)
    editorEl.selectionStart = editorEl.selectionEnd = start + 2
    activeContent = editorEl.value
    isDirty = true
    saveIndicatorEl.textContent = 'unsaved'
    saveIndicatorEl.className = 'edit-indicator unsaved'
    scheduleAutosave()
  }
})

// ── Init ──────────────────────────────────────────────────────────────────

bindTreeDragDrop()
loadNotes()
