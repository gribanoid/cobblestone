// Capture-phase shortcuts for cobblestone-editor (injected via document::eval).
// Sticky bind survives remounts; __cbMdShortcutSend is rebound each mount.
window.__cbMdShortcutSend = (op) => dioxus.send(op);
if (!window.__cbMdShortcutBound) {
  window.__cbMdShortcutBound = true;
  const send = (op) => {
    if (typeof window.__cbMdShortcutSend === 'function')
      window.__cbMdShortcutSend(op);
  };
  const lineSel = () => {
    const el = document.activeElement;
    if (!el || el.tagName !== 'TEXTAREA') return null;
    if (!el.classList.contains('cb-md-input') &&
        !el.classList.contains('cb-md-textarea')) return null;
    const a = el.selectionStart|0, b = el.selectionEnd|0;
    return { el, a, b, v: el.value || '' };
  };
  const copyText = () => {
    const root = document.querySelector('.cb-md-editor');
    if (root && root.classList.contains('cb-md-has-sel'))
      return window.__cbMdDocSelText || '';
    const s = lineSel();
    if (s && s.a !== s.b) return s.v.slice(s.a, s.b);
    return '';
  };
  const editorRoot = (e) => {
    const t = e.target;
    const inside = t && t.closest && t.closest('.cb-md-editor');
    if (inside) return inside;
    if (!t || t === document.body || t === document.documentElement) {
      const r = document.querySelector('.cb-md-editor');
      if (r && (r.classList.contains('cb-md-has-sel') ||
                r.querySelector('.cb-md-block-active'))) return r;
    }
    return null;
  };
  const isField = (t) => !!(t && (t.tagName === 'INPUT' ||
    t.tagName === 'TEXTAREA' || t.isContentEditable));
  document.addEventListener('keydown', (e) => {
    const root = editorRoot(e);
    if (!root) {
      if ((e.metaKey || e.ctrlKey) && e.code === 'KeyA' &&
          !isField(e.target) && document.querySelector('.cb-md-editor')) {
        e.preventDefault(); e.stopPropagation(); send('selectall');
      }
      return;
    }
    const mod = e.metaKey || e.ctrlKey;
    if (!mod) {
      if (e.key === 'Enter' && !e.isComposing && !e.altKey &&
          !root.classList.contains('cb-md-has-sel')) {
        const s = lineSel();
        if (s && s.el.classList.contains('cb-md-input')) {
          e.preventDefault(); e.stopPropagation();
          send('enter|' + s.a + '|' + s.b);
          return;
        }
      }
      if (root.classList.contains('cb-md-has-sel') &&
          (e.code === 'Backspace' || e.code === 'Delete')) {
        e.preventDefault(); e.stopPropagation();
        send('delete');
      }
      return;
    }
    const code = e.code || '';
    const hasSel = root.classList.contains('cb-md-has-sel');
    if (code === 'KeyZ' && !e.shiftKey) {
      e.preventDefault(); e.stopPropagation(); send('undo');
    } else if ((code === 'KeyZ' && e.shiftKey) || code === 'KeyY') {
      e.preventDefault(); e.stopPropagation(); send('redo');
    } else if (code === 'KeyA') {
      e.preventDefault(); e.stopPropagation(); send('selectall');
    } else if (code === 'KeyC') {
      const text = copyText();
      if (!text && !hasSel) return;
      e.preventDefault(); e.stopPropagation();
      send('copy\n' + text);
    } else if (code === 'KeyX') {
      const text = copyText();
      if (!text && !hasSel) return;
      e.preventDefault(); e.stopPropagation();
      send('copy\n' + text);
      if (hasSel) send('cut');
      else {
        const s = lineSel();
        if (s && s.a !== s.b) send('cutline|' + s.a + '|' + s.b);
      }
    } else if (code === 'KeyV') {
      e.preventDefault(); e.stopPropagation();
      send('paste_req');
    }
  }, true);
  const onCopyCut = (isCut) => (e) => {
    const root = editorRoot(e);
    if (!root) return;
    const text = copyText();
    if (!text && !root.classList.contains('cb-md-has-sel')) return;
    e.preventDefault();
    e.stopPropagation();
    send('copy\n' + text);
    if (isCut) {
      if (root.classList.contains('cb-md-has-sel')) send('cut');
      else {
        const s = lineSel();
        if (s && s.a !== s.b) send('cutline|' + s.a + '|' + s.b);
      }
    }
  };
  document.addEventListener('copy', onCopyCut(false), true);
  document.addEventListener('cut', onCopyCut(true), true);
  document.addEventListener('paste', (e) => {
    const root = editorRoot(e);
    if (!root) return;
    const items = (e.clipboardData && e.clipboardData.items) || [];
    for (let i = 0; i < items.length; i++) {
      if (items[i].type && items[i].type.indexOf('image/') === 0) return;
    }
    e.preventDefault();
    e.stopPropagation();
    const text = (e.clipboardData && e.clipboardData.getData('text/plain')) || '';
    if (text) send('paste\n' + text);
    else send('paste_req');
  }, true);
  document.addEventListener('mouseup', () => send('dragend'), true);
}
