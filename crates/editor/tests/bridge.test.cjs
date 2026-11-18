// SPDX-License-Identifier: MIT

const { readFileSync } = require('node:fs');
const { join } = require('node:path');
const { runInNewContext } = require('node:vm');
const { test } = require('node:test');
const assert = require('node:assert/strict');

function bridge(value, start, end = start) {
  const messages = [];
  const handlers = {};
  const root = { classList: { contains: () => false } };
  const input = {
    tagName: 'TEXTAREA', value, selectionStart: start, selectionEnd: end,
    classList: { contains: name => name === 'cb-md-input' },
    closest: () => root,
  };
  runInNewContext(readFileSync(join(__dirname, '../src/bridge.js'), 'utf8'), {
    window: {}, desktopClipboard: true, dioxus: { send: op => messages.push(op) },
    document: {
      activeElement: input,
      querySelector: () => root,
      addEventListener: (name, callback) => { handlers[name] = callback; },
    },
  });
  return {
    messages,
    key: properties => handlers.keydown({
      target: input, preventDefault() {}, stopPropagation() {}, ...properties,
    }),
  };
}

test('Enter sends scalar offsets after emoji instead of UTF-16 offsets', () => {
  const editor = bridge('😀 текст', 3, 7);
  editor.key({ key: 'Enter' });
  assert.deepEqual(editor.messages, ['enter|2|6']);
});

test('Cut copies the selected emoji and deletes exactly that scalar', () => {
  const editor = bridge('a😀b', 1, 3);
  editor.key({ code: 'KeyX', metaKey: true });
  assert.deepEqual(editor.messages, ['copy\n😀', 'cutline|1|2']);
});

test('composition Enter stays with the native input method', () => {
  const editor = bridge('😀漢', 3);
  editor.key({ key: 'Enter', isComposing: true });
  assert.deepEqual(editor.messages, []);
});
