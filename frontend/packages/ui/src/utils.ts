// SPDX-License-Identifier: MIT

import type { NoteInfo, VaultNode } from '@cobblestone/api'

export function escHtml(s: string): string {
  return s
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;')
}

export function folderHint(): string {
  return 'Root'
}

export function noteParentFolder(slug: string): string | null {
  const idx = slug.lastIndexOf('/')
  return idx >= 0 ? slug.slice(0, idx) : null
}

export function flattenTree(nodes: VaultNode[]): NoteInfo[] {
  const out: NoteInfo[] = []
  for (const node of nodes) {
    if (node.kind === 'note') {
      out.push({
        slug: node.slug,
        title: node.title,
        created: node.created,
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

export function folderDestPath(from: string, destParent: string | null): string {
  const name = from.split('/').pop()!
  return destParent ? `${destParent}/${name}` : name
}

export function remapPath(path: string, from: string, to: string): string {
  if (path === from) return to
  if (path.startsWith(`${from}/`)) return to + path.slice(from.length)
  return path
}

export function contentWithTitle(content: string, title: string): string {
  const t = title.trim()
  if (!t) return content
  const lines = content.split('\n')
  if (lines[0]?.startsWith('# ')) {
    const rest = lines.slice(1).join('\n')
    return rest ? `# ${t}\n${rest}` : `# ${t}`
  }
  return `# ${t}\n\n${content}`
}

export const DRAG_THRESHOLD_PX = 5

export const el = <T extends HTMLElement>(id: string) =>
  document.getElementById(id) as T
