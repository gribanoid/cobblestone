// SPDX-License-Identifier: GPL-3.0-or-later

import { marked } from 'marked'
import { api, NoteInfo, NoteGraph } from './api'

// ── Markdown setup ────────────────────────────────────────────────────────

marked.use({ gfm: true, breaks: false })

// ── State ─────────────────────────────────────────────────────────────────

let notes: NoteInfo[] = []
let activeSlug: string | null = null
let activeTitle = ''
let activeContent = ''
let graph: NoteGraph | null = null
let isEditing = false
let isDirty = false
let saveTimer: ReturnType<typeof setTimeout> | null = null

// ── DOM helpers ───────────────────────────────────────────────────────────

const el = <T extends HTMLElement>(id: string) => document.getElementById(id) as T

const noteListEl     = el<HTMLDivElement>('note-list')
const searchEl       = el<HTMLInputElement>('search')
const newBtnEl       = el<HTMLButtonElement>('new-btn')
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
const errorToastEl   = el<HTMLDivElement>('error-toast')
const panelContentEl = el<HTMLDivElement>('panel-content')

function escHtml(s: string): string {
  return s
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;')
}

// ── Render ────────────────────────────────────────────────────────────────

function renderNoteList(items: NoteInfo[]) {
  if (items.length === 0) {
    noteListEl.innerHTML =
      '<div class="empty-state">No notes yet.<br>Click "+ New note" to start.</div>'
    return
  }
  noteListEl.innerHTML = items
    .map(
      (n) => `
      <div class="note-item${n.slug === activeSlug ? ' active' : ''}" data-slug="${escHtml(n.slug)}">
        <div class="note-item-title">${escHtml(n.title)}</div>
        <div class="note-item-meta">${escHtml(n.modified)}${n.size > 0 ? ` · ${n.size} B` : ''}</div>
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
    ${linkSection('Backlinks', 'No backlinks yet', g.backlinks)}`

  panelContentEl.querySelectorAll<HTMLButtonElement>('[data-slug]').forEach((btn) => {
    btn.addEventListener('click', () => openNote(btn.dataset.slug!))
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
    notes = await api.listNotes()
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
  const title = titleFromSearch || `Note ${Date.now()}`
  try {
    const slug = await api.createNote(title)
    searchEl.value = ''
    await loadNotes()
    await openNote(slug)
    isEditing = true
    renderEditorArea()
  } catch (e) {
    showError(String(e))
  }
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

newBtnEl.addEventListener('click', createNote)

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
  if (e.key === 'Escape' && confirmModalEl.style.display !== 'none') {
    confirmModalEl.style.display = 'none'
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

loadNotes()
