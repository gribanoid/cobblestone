// SPDX-License-Identifier: MIT

import { el } from './utils'

export interface DomRefs {
  noteListEl: HTMLDivElement
  searchEl: HTMLInputElement
  newBtnEl: HTMLButtonElement
  newFolderBtnEl: HTMLButtonElement
  welcomeEl: HTMLDivElement
  editorAreaEl: HTMLDivElement
  noteTitleEl: HTMLInputElement
  editorEl: HTMLTextAreaElement
  previewEl: HTMLDivElement
  saveIndicatorEl: HTMLSpanElement
  toggleModeEl: HTMLButtonElement
  saveBtnEl: HTMLButtonElement
  themeBtnEl: HTMLButtonElement
  confirmModalEl: HTMLDivElement
  confirmModalTitleEl: HTMLHeadingElement
  confirmModalMessageEl: HTMLParagraphElement
  cancelDeleteEl: HTMLButtonElement
  confirmDeleteEl: HTMLButtonElement
  renameModalEl: HTMLDivElement
  renameModalTitleEl: HTMLHeadingElement
  renameInputEl: HTMLInputElement
  cancelRenameEl: HTMLButtonElement
  confirmRenameEl: HTMLButtonElement
  folderModalEl: HTMLDivElement
  folderModalHintEl: HTMLParagraphElement
  folderNameInputEl: HTMLInputElement
  cancelFolderEl: HTMLButtonElement
  confirmFolderEl: HTMLButtonElement
  noteModalEl: HTMLDivElement
  noteModalHintEl: HTMLParagraphElement
  noteTitleInputEl: HTMLInputElement
  cancelNoteModalEl: HTMLButtonElement
  confirmNoteModalEl: HTMLButtonElement
  errorToastEl: HTMLDivElement
  panelContentEl: HTMLDivElement
}

export function getDomRefs(): DomRefs {
  return {
    noteListEl: el('note-list'),
    searchEl: el('search'),
    newBtnEl: el('new-btn'),
    newFolderBtnEl: el('new-folder-btn'),
    welcomeEl: el('welcome'),
    editorAreaEl: el('editor-area'),
    noteTitleEl: el('note-title'),
    editorEl: el('editor'),
    previewEl: el('preview'),
    saveIndicatorEl: el('save-indicator'),
    toggleModeEl: el('toggle-mode-btn'),
    saveBtnEl: el('save-btn'),
    themeBtnEl: el('theme-btn'),
    confirmModalEl: el('confirm-modal'),
    confirmModalTitleEl: el('confirm-modal-title'),
    confirmModalMessageEl: el('confirm-modal-message'),
    cancelDeleteEl: el('cancel-delete-btn'),
    confirmDeleteEl: el('confirm-delete-btn'),
    renameModalEl: el('rename-modal'),
    renameModalTitleEl: el('rename-modal-title'),
    renameInputEl: el('rename-input'),
    cancelRenameEl: el('cancel-rename-btn'),
    confirmRenameEl: el('confirm-rename-btn'),
    folderModalEl: el('folder-modal'),
    folderModalHintEl: el('folder-modal-hint'),
    folderNameInputEl: el('folder-name-input'),
    cancelFolderEl: el('cancel-folder-btn'),
    confirmFolderEl: el('confirm-folder-btn'),
    noteModalEl: el('note-modal'),
    noteModalHintEl: el('note-modal-hint'),
    noteTitleInputEl: el('note-title-input'),
    cancelNoteModalEl: el('cancel-note-btn'),
    confirmNoteModalEl: el('confirm-note-btn'),
    errorToastEl: el('error-toast'),
    panelContentEl: el('panel-content'),
  }
}
