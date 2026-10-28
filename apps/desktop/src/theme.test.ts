import { test } from 'node:test';
import assert from 'node:assert/strict';
import { parseTheme, themes } from './theme';
test('built-in themes survive validation', () => {
  for (const theme of Object.values(themes)) assert.deepEqual(parseTheme(theme), theme);
});
test('rejects CSS injection and malformed themes', () => {
  assert.throws(() => parseTheme({ ...themes.dark, colors: { ...themes.dark.colors, accent: 'url(https://example.com)' } }));
  assert.throws(() => parseTheme({ ...themes.dark, version: 2 }));
  assert.throws(() => parseTheme(null));
});
