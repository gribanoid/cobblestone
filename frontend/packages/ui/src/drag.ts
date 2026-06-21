// SPDX-License-Identifier: MIT

import type { DomRefs } from './dom'
import { DRAG_THRESHOLD_PX, noteParentFolder } from './utils'

export type TreeDrag =
  | { kind: 'note'; slug: string; el: HTMLElement }
  | { kind: 'folder'; path: string; el: HTMLElement }

export interface DragHandlers {
  canDrop: (payload: TreeDrag, destFolder: string | null) => boolean
  onDrop: (payload: TreeDrag, destFolder: string | null) => void
}

let suppressTreeClick = false
let activeDropEl: HTMLElement | null = null

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

function findDropTarget(
  noteListEl: HTMLElement,
  clientX: number,
  clientY: number,
): { el: HTMLElement; folder: string | null } | null {
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

function startTreeDrag(
  payload: TreeDrag,
  startX: number,
  startY: number,
  noteListEl: HTMLElement,
  handlers: DragHandlers,
) {
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
    const target = findDropTarget(noteListEl, ev.clientX, ev.clientY)
    if (target && handlers.canDrop(payload, target.folder)) {
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
      const target = findDropTarget(noteListEl, ev.clientX, ev.clientY)
      clearDropHighlight()
      if (target && handlers.canDrop(payload, target.folder)) {
        handlers.onDrop(payload, target.folder)
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

export function canDropNoteOrFolder(
  payload: TreeDrag,
  destFolder: string | null,
): boolean {
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
  const name = from.split('/').pop()!
  const newPath = destFolder ? `${destFolder}/${name}` : name
  if (newPath === from) return false
  if (destFolder === from) return false
  if (destFolder.startsWith(`${from}/`)) return false
  return true
}

export function bindTreeDragDrop(dom: DomRefs, handlers: DragHandlers) {
  const { noteListEl } = dom
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
    if (e.ctrlKey) return

    const noteEl = (e.target as HTMLElement).closest('.tree-note') as HTMLElement | null
    const folderEl = (e.target as HTMLElement).closest('.tree-folder') as HTMLElement | null

    let payload: TreeDrag | null = null
    if (noteEl) {
      payload = { kind: 'note', slug: noteEl.dataset.slug!, el: noteEl }
    } else if (folderEl) {
      payload = { kind: 'folder', path: folderEl.dataset.path!, el: folderEl }
    }
    if (!payload) return

    startTreeDrag(payload, e.clientX, e.clientY, noteListEl, handlers)
  })
}
