// SPDX-License-Identifier: MIT

import assert from 'node:assert/strict'
import { test } from 'node:test'

import {
  contentWithTitle,
  noteParentFolder,
  remapPath,
  stripLeadingHeading,
} from './utils.ts'

test('stripLeadingHeading removes first h1', () => {
  assert.equal(stripLeadingHeading('# Title\n\nbody'), 'body')
})

test('stripLeadingHeading keeps content without heading', () => {
  assert.equal(stripLeadingHeading('plain text'), 'plain text')
})

test('contentWithTitle prepends heading for editor body', () => {
  assert.equal(contentWithTitle('hello', 'My Note'), '# My Note\n\nhello')
})

test('contentWithTitle updates existing heading line', () => {
  assert.equal(
    contentWithTitle('# Old\n\nbody', 'New'),
    '# New\n\nbody',
  )
})

test('remapPath replaces folder prefix', () => {
  assert.equal(remapPath('ideas/a', 'ideas', 'archive'), 'archive/a')
})

test('noteParentFolder returns parent path', () => {
  assert.equal(noteParentFolder('ideas/note'), 'ideas')
  assert.equal(noteParentFolder('root-note'), null)
})
