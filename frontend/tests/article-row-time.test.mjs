import assert from 'node:assert/strict'
import fs from 'node:fs'
import vm from 'node:vm'
import { createRequire } from 'node:module'
import { transformWithOxc } from 'vite'
const require = createRequire(import.meta.url)
const source = fs.readFileSync(new URL('../src/ArticleRowContent.tsx', import.meta.url), 'utf8')
const result = await transformWithOxc(source, 'ArticleRowContent.tsx', { jsx: { runtime: 'automatic' } })
const compiled = result.code.replace(/^import .*;?$/gm, '').replace(/export function /g, 'function ') + '\nexports.formatArticlePublished = formatArticlePublished;'
const exportsObject = {}
vm.runInNewContext(compiled, { exports: exportsObject, require: name => name.endsWith('.css') ? {} : require(name), Intl, Date, URL })
const format = exportsObject.formatArticlePublished
assert.equal(format('2024-01-02 03:04:05', 'UTC'), '2024-01-02 03:04:05 UTC')
assert.equal(format('2024-01-02 03:04:05', 'Asia/Tokyo'), '2024-01-02 12:04:05 JST')
assert.equal(format('2024-01-02T03:04:05+09:00', 'UTC'), '2024-01-01 18:04:05 UTC')
assert.equal(format('bad', 'UTC'), '公開日不明')
