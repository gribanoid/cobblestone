// SPDX-License-Identifier: MIT

import {
  ChevronRight,
  createIcons,
  FilePlus,
  FileText,
  Folder,
  FolderPlus,
  SunMoon,
} from 'lucide'

const icons = {
  ChevronRight,
  FilePlus,
  FileText,
  Folder,
  FolderPlus,
  SunMoon,
}

/** Replace `[data-lucide]` placeholders with Lucide SVG icons (ISC). */
export function refreshIcons(root: ParentNode = document): void {
  createIcons({
    icons,
    root: root as HTMLElement,
    attrs: {
      'stroke-width': '1.75',
      'aria-hidden': 'true',
    },
  })
}
