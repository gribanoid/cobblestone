// SPDX-License-Identifier: MIT

import type { NoteGraph, NoteInfo, VaultNode } from '@cobblestone/api'
import { marked } from 'marked'

import type { DomRefs } from './dom'
import { refreshIcons } from './icons'
import {
  escHtml,
  folderHint,
  noteParentFolder,
  bodyForPreview,
} from './utils'

marked.use({ gfm: true, breaks: false })

export interface RenderCtx {
  dom: DomRefs
  activeSlug: string | null
  activeTitle: string
  activeContent: string
  notes: NoteInfo[]
  tree: VaultNode[]
  graph: NoteGraph | null
  expandedFolders: Set<string>
  isEditing: boolean
  isDirty: boolean
  searchQuery: string
  onOpenNote: (slug: string) => void
  onToggleFolder: (path: string) => void
  onCreateNoteInFolder: (path: string | null) => void
  onCreateFolderInFolder: (path: string | null) => void
  onCopyNote: (slug: string) => void
  onCopyFolder: (path: string) => void
  onRenameNote: (slug: string) => void
  onRenameFolder: (path: string) => void
  onDeleteNote: (slug: string) => void
  onDeleteFolder: (path: string) => void
  onMoveNoteToRoot: () => void
}

type ContextMenuItem =
  | { kind: 'action'; label: string; action: () => void; danger?: boolean }
  | { kind: 'separator' }

let folderContextMenuEl: HTMLDivElement | null = null
let dismissFolderContextMenu: ((e: Event) => void) | null = null

export function closeFolderContextMenu() {
  folderContextMenuEl?.remove()
  folderContextMenuEl = null
  if (dismissFolderContextMenu) {
    document.removeEventListener('pointerdown', dismissFolderContextMenu, true)
    document.removeEventListener('keydown', dismissFolderContextMenu, true)
    dismissFolderContextMenu = null
  }
}

function showTreeContextMenu(x: number, y: number, items: ContextMenuItem[]) {
  closeFolderContextMenu()

  const menu = document.createElement('div')
  menu.className = 'tree-context-menu'

  for (const item of items) {
    if (item.kind === 'separator') {
      const hr = document.createElement('hr')
      hr.className = 'tree-context-separator'
      menu.appendChild(hr)
      continue
    }
    const btn = document.createElement('button')
    btn.type = 'button'
    btn.className = `tree-context-item${item.danger ? ' danger' : ''}`
    btn.textContent = item.label
    btn.addEventListener('click', () => {
      closeFolderContextMenu()
      item.action()
    })
    menu.appendChild(btn)
  }

  document.body.appendChild(menu)
  folderContextMenuEl = menu

  const rect = menu.getBoundingClientRect()
  const left = Math.min(x, window.innerWidth - rect.width - 8)
  const top = Math.min(y, window.innerHeight - rect.height - 8)
  menu.style.left = `${Math.max(8, left)}px`
  menu.style.top = `${Math.max(8, top)}px`

  dismissFolderContextMenu = (e: Event) => {
    if (e.type === 'keydown' && (e as KeyboardEvent).key !== 'Escape') return
    if (
      e.type === 'pointerdown' &&
      folderContextMenuEl?.contains(e.target as Node)
    ) {
      return
    }
    closeFolderContextMenu()
  }
  requestAnimationFrame(() => {
    if (dismissFolderContextMenu) {
      document.addEventListener('pointerdown', dismissFolderContextMenu, true)
      document.addEventListener('keydown', dismissFolderContextMenu, true)
    }
  })
}

function rootMenuItems(
  onCreateNoteInFolder: (path: string | null) => void,
  onCreateFolderInFolder: (path: string | null) => void,
): ContextMenuItem[] {
  return [
    { kind: 'action', label: 'New note', action: () => onCreateNoteInFolder(null) },
    { kind: 'action', label: 'New folder', action: () => onCreateFolderInFolder(null) },
  ]
}

function folderMenuItems(
  path: string,
  actions: Pick<
    RenderCtx,
    | 'onCreateNoteInFolder'
    | 'onCreateFolderInFolder'
    | 'onCopyFolder'
    | 'onRenameFolder'
    | 'onDeleteFolder'
  >,
): ContextMenuItem[] {
  return [
    { kind: 'action', label: 'New note', action: () => actions.onCreateNoteInFolder(path) },
    { kind: 'action', label: 'New folder', action: () => actions.onCreateFolderInFolder(path) },
    { kind: 'separator' },
    { kind: 'action', label: 'Copy', action: () => actions.onCopyFolder(path) },
    { kind: 'action', label: 'Rename…', action: () => actions.onRenameFolder(path) },
    { kind: 'action', label: 'Delete', action: () => actions.onDeleteFolder(path), danger: true },
  ]
}

function noteMenuItems(
  slug: string,
  actions: Pick<RenderCtx, 'onCopyNote' | 'onRenameNote' | 'onDeleteNote'>,
): ContextMenuItem[] {
  return [
    { kind: 'action', label: 'Copy', action: () => actions.onCopyNote(slug) },
    { kind: 'separator' },
    { kind: 'action', label: 'Rename…', action: () => actions.onRenameNote(slug) },
    { kind: 'action', label: 'Delete', action: () => actions.onDeleteNote(slug), danger: true },
  ]
}

function bindVaultRootMenu(
  el: HTMLElement,
  isTreeView: () => boolean,
  onCreateNoteInFolder: (path: string | null) => void,
  onCreateFolderInFolder: (path: string | null) => void,
) {
  if (el.dataset.rootMenuBound === '1') return
  el.dataset.rootMenuBound = '1'

  const openMenu = (e: MouseEvent) => {
    if (!isTreeView()) return
    if (!e.ctrlKey) return
    if (!(e.target as HTMLElement).closest('.vault-tree')) return
    if ((e.target as HTMLElement).closest('.tree-folder, .tree-note')) return
    e.preventDefault()
    e.stopPropagation()
    showTreeContextMenu(
      e.clientX,
      e.clientY,
      rootMenuItems(onCreateNoteInFolder, onCreateFolderInFolder),
    )
  }

  el.addEventListener('click', openMenu)
  el.addEventListener('contextmenu', openMenu)
}

export function bindVaultRootMenuHandlers(
  el: HTMLElement,
  isTreeView: () => boolean,
  onCreateNoteInFolder: (path: string | null) => void,
  onCreateFolderInFolder: (path: string | null) => void,
) {
  bindVaultRootMenu(el, isTreeView, onCreateNoteInFolder, onCreateFolderInFolder)
}

function bindFolderCtrlClick(div: HTMLDivElement, path: string, ctx: RenderCtx) {
  const openMenu = (e: MouseEvent) => {
    if (!e.ctrlKey) return
    e.preventDefault()
    e.stopPropagation()
    showTreeContextMenu(e.clientX, e.clientY, folderMenuItems(path, ctx))
  }

  div.addEventListener('click', (e) => {
    if (e.ctrlKey) {
      openMenu(e)
      return
    }
    ctx.onToggleFolder(path)
  })

  div.addEventListener('contextmenu', openMenu)
}

function bindNoteCtrlClick(div: HTMLDivElement, slug: string, ctx: RenderCtx) {
  const openMenu = (e: MouseEvent) => {
    if (!e.ctrlKey) return
    e.preventDefault()
    e.stopPropagation()
    showTreeContextMenu(e.clientX, e.clientY, noteMenuItems(slug, ctx))
  }

  div.addEventListener('click', (e) => {
    if (e.ctrlKey) {
      openMenu(e)
      return
    }
    ctx.onOpenNote(slug)
  })

  div.addEventListener('contextmenu', openMenu)
}

export function renderSearchResults(ctx: RenderCtx, items: NoteInfo[]) {
  const { dom, activeSlug, onOpenNote } = ctx
  if (items.length === 0) {
    dom.noteListEl.innerHTML = '<div class="empty-state">No matching notes.</div>'
    return
  }
  dom.noteListEl.innerHTML = items
    .map(
      (n) => `
      <div class="note-item${n.slug === activeSlug ? ' active' : ''}" data-slug="${escHtml(n.slug)}">
        <div class="note-item-title">${escHtml(n.title)}</div>
        <div class="note-item-meta">${escHtml(n.modified)}</div>
        ${n.tags.length > 0
          ? `<div class="note-item-tags">${n.tags.map((t) => `<span class="tag">#${escHtml(t)}</span>`).join('')}</div>`
          : ''}
      </div>`,
    )
    .join('')

  dom.noteListEl.querySelectorAll<HTMLDivElement>('.note-item').forEach((div) => {
    div.addEventListener('click', () => onOpenNote(div.dataset.slug!))
  })
}

function renderTreeNode(ctx: RenderCtx, node: VaultNode): string {
  const { activeSlug, expandedFolders } = ctx
  if (node.kind === 'note') {
    const active = node.slug === activeSlug ? ' active' : ''
    return `
      <div class="tree-note note-item${active}" data-slug="${escHtml(node.slug)}">
        <span class="tree-spacer"></span>
        <i data-lucide="file-text" class="tree-note-icon"></i>
        <span class="tree-note-title">${escHtml(node.title)}</span>
      </div>`
  }

  const expanded = expandedFolders.has(node.path)
  const children = expanded
    ? `<div class="tree-children">${node.children.map((c) => renderTreeNode(ctx, c)).join('')}</div>`
    : ''

  return `
    <div class="tree-folder-wrap" data-drop-folder="${escHtml(node.path)}">
      <div class="tree-folder${expanded ? ' expanded' : ''}" data-path="${escHtml(node.path)}">
        <span class="tree-chevron"><i data-lucide="chevron-right"></i></span>
        <i data-lucide="folder" class="tree-folder-icon"></i>
        <span class="tree-folder-name">${escHtml(node.name)}</span>
      </div>
      ${children}
    </div>`
}

export function renderVaultTree(ctx: RenderCtx) {
  const { dom, tree } = ctx
  closeFolderContextMenu()
  const items = tree.map((n) => renderTreeNode(ctx, n)).join('')
  const emptyHint =
    tree.length === 0
      ? '<div class="empty-state tree-empty-hint">No folders yet.<br>Use the folder icon above to start.</div>'
      : ''

  dom.noteListEl.innerHTML = `
    <div class="vault-tree">
      ${items}
      ${emptyHint}
    </div>`

  dom.noteListEl.querySelectorAll<HTMLDivElement>('.tree-folder').forEach((div) => {
    bindFolderCtrlClick(div, div.dataset.path!, ctx)
  })

  dom.noteListEl.querySelectorAll<HTMLDivElement>('.tree-note').forEach((div) => {
    bindNoteCtrlClick(div, div.dataset.slug!, ctx)
  })

  refreshIcons(dom.noteListEl)
}

export function renderNoteList(ctx: RenderCtx) {
  if (ctx.searchQuery) {
    renderSearchResults(ctx, ctx.notes)
    return
  }
  renderVaultTree(ctx)
}

export function renderEditorArea(ctx: RenderCtx) {
  const { dom, activeSlug, activeTitle, activeContent, isDirty, isEditing } = ctx
  const open = activeSlug !== null
  dom.welcomeEl.style.display = open ? 'none' : 'flex'
  dom.editorAreaEl.style.display = open ? 'flex' : 'none'

  if (!open) return

  if (document.activeElement !== dom.noteTitleEl) {
    dom.noteTitleEl.value = activeTitle
  }
  if (document.activeElement !== dom.editorEl) {
    dom.editorEl.value = activeContent
  }

  if (isDirty) {
    dom.saveIndicatorEl.textContent = 'unsaved'
    dom.saveIndicatorEl.className = 'edit-indicator unsaved'
  } else {
    dom.saveIndicatorEl.textContent = 'saved'
    dom.saveIndicatorEl.className = 'edit-indicator'
  }

  if (isEditing) {
    dom.editorEl.classList.remove('hidden')
    dom.previewEl.classList.add('hidden')
    dom.toggleModeEl.textContent = 'Preview'
    dom.toggleModeEl.className = 'tb-btn'
  } else {
    dom.editorEl.classList.add('hidden')
    dom.previewEl.classList.remove('hidden')
    dom.previewEl.innerHTML = marked.parse(
      bodyForPreview(activeContent, activeTitle),
    ) as string
    dom.toggleModeEl.textContent = 'Edit'
    dom.toggleModeEl.className = 'tb-btn active'
  }
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

function moveSectionHtml(activeSlug: string | null): string {
  if (!activeSlug || noteParentFolder(activeSlug) === null) return ''
  return `<section class="panel-section">
      <h3>Move</h3>
      <div class="panel-link-list">
        <button type="button" class="panel-link" id="move-note-to-root-btn">Move note to root</button>
      </div>
    </section>`
}

export function renderRightPanel(ctx: RenderCtx) {
  const { dom, activeSlug, notes, graph, onOpenNote, onMoveNoteToRoot } = ctx

  if (!activeSlug) {
    dom.panelContentEl.innerHTML = `
      <div class="panel-empty">
        <h3>Note info</h3>
        <p>Open a note to see tags, metadata, wikilinks, and backlinks.</p>
      </div>`
    return
  }

  const info = notes.find((n) => n.slug === activeSlug)
  const g: NoteGraph = graph ?? { outgoing: [], backlinks: [] }

  dom.panelContentEl.innerHTML = `
    <section class="panel-section">
      <h3>Metadata</h3>
      <dl class="meta-list">
        <div><dt>Created</dt><dd>${escHtml(info?.created ?? '')}</dd></div>
        <div><dt>Modified</dt><dd>${escHtml(info?.modified ?? '')}</dd></div>
        <div><dt>Size</dt><dd>${info?.size ?? 0} B</dd></div>
        <div><dt>Path</dt><dd>${escHtml(activeSlug)}.md</dd></div>
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
    ${moveSectionHtml(activeSlug)}`

  dom.panelContentEl.querySelectorAll<HTMLButtonElement>('[data-slug]').forEach((btn) => {
    btn.addEventListener('click', () => onOpenNote(btn.dataset.slug!))
  })

  document.getElementById('move-note-to-root-btn')?.addEventListener('click', onMoveNoteToRoot)
}

export function showError(dom: DomRefs, msg: string) {
  dom.errorToastEl.textContent = `⚠ ${msg}`
  dom.errorToastEl.style.display = 'block'
  setTimeout(() => {
    dom.errorToastEl.style.display = 'none'
  }, 5000)
}

export function openNoteModal(dom: DomRefs, folder: string | null = null) {
  const titleFromSearch = dom.searchEl.value.trim()
  dom.noteTitleInputEl.value = titleFromSearch
  dom.noteModalHintEl.textContent = folder ?? folderHint()
  dom.noteModalEl.style.display = 'flex'
  dom.noteTitleInputEl.focus()
  dom.noteTitleInputEl.select()
}

export function closeNoteModal(dom: DomRefs) {
  dom.noteModalEl.style.display = 'none'
}

export function openFolderModal(dom: DomRefs, parent: string | null = null) {
  dom.folderNameInputEl.value = ''
  dom.folderModalHintEl.textContent = parent ?? folderHint()
  dom.folderModalEl.style.display = 'flex'
  dom.folderNameInputEl.focus()
}

export function closeFolderModal(dom: DomRefs) {
  dom.folderModalEl.style.display = 'none'
}

export function openRenameModal(dom: DomRefs, title: string, value: string) {
  dom.renameModalTitleEl.textContent = title
  dom.renameInputEl.value = value
  dom.renameModalEl.style.display = 'flex'
  dom.renameInputEl.focus()
  dom.renameInputEl.select()
}

export function closeRenameModal(dom: DomRefs) {
  dom.renameModalEl.style.display = 'none'
}

export function openDeleteConfirmModal(
  dom: DomRefs,
  title: string,
  message: string,
) {
  dom.confirmModalTitleEl.textContent = title
  dom.confirmModalMessageEl.textContent = message
  dom.confirmModalEl.style.display = 'flex'
}

export function closeDeleteConfirmModal(dom: DomRefs) {
  dom.confirmModalEl.style.display = 'none'
}

export function toggleTheme() {
  const html = document.documentElement
  const next = html.dataset.theme === 'dark' ? 'light' : 'dark'
  html.dataset.theme = next
  localStorage.setItem('cb-theme', next)
}
