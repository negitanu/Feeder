import assert from 'node:assert/strict'
import { createRequire } from 'node:module'
import { fileURLToPath } from 'node:url'

const require = createRequire(new URL('../frontend/package.json', import.meta.url))
const { chromium } = require('@playwright/test')
const { createServer } = await import(fileURLToPath(new URL('../frontend/node_modules/vite/dist/node/index.js', import.meta.url)))
const root = fileURLToPath(new URL('../frontend', import.meta.url))
const server = await createServer({ root, server: { port: 0 } })
await server.listen()
let browser
try {
  browser = await chromium.launch({ headless: true, ...(process.env.FEEDER_CHROMIUM_PATH ? { executablePath: process.env.FEEDER_CHROMIUM_PATH } : {}) })
  const article = { id: 1, title: 'Synthetic article', content: 'Synthetic content '.repeat(80), url: 'https://example.test/article', published: '2026-01-01T00:00:00Z', is_read: false, is_starred: false, is_saved_for_later: false }
  for (const width of [1920, 390]) {
    const context = await browser.newContext({ viewport: { width, height: 900 } })
    const page = await context.newPage()
    await page.route('**/api/**', route => {
      const path = new URL(route.request().url()).pathname
      const fixtures = { '/api/session': { csrf_token: 'a'.repeat(64) }, '/api/library': { feeds: [], folders: [] }, '/api/reader/settings': { display_timezone: 'UTC', default_view: 'all' }, '/api/articles': { items: [article], total: 1, has_more: false, next_cursor: null }, '/api/articles/1/visit': article, '/api/library/unread': { count: 0, folders: [] }, '/api/collection/settings': { enabled: false, interval_seconds: 3600 }, '/api/collection/health': { feeds: [], global_enabled: false, global_interval_seconds: 3600 }, '/api/read-later/settings': { auto_remove: false }, '/api/keys': { keys: [] }, '/api/keys/settings': { rate_limit: 60 }, '/api/mute-rules': { rules: [] }, '/api/social/sources': { sources: [] }, '/api/articles/1/revisions': { items: [] }, '/api/articles/1/related': { items: [] } }
      return route.fulfill({ json: fixtures[path] ?? {} })
    })
    async function settings() {
      await page.getByRole('button', { name: '設定', exact: true }).click()
      await page.getByRole('button', { name: '表示・読書とデータ' }).click()
    }
    async function open() {
      await page.getByRole('button', { name: '読む', exact: true }).click()
      await page.locator('.article-open').first().click()
      await page.locator('.article-text').waitFor()
    }
    async function values() {
      return page.locator('.article-body').evaluate(el => {
        const text = el.querySelector('.article-text')
        const computed = getComputedStyle(text)
        const probe = document.createElement('div')
        probe.style.cssText = 'position:absolute;visibility:hidden;max-width:min(100%, 90ch)'
        el.append(probe); const expectedMax = getComputedStyle(probe).maxWidth; probe.remove()
        return { fontSize: parseFloat(computed.fontSize), lineHeight: parseFloat(computed.lineHeight), bodyWidth: el.getBoundingClientRect().width, maxWidth: getComputedStyle(el).maxWidth, expectedMax, parentWidth: el.parentElement.clientWidth }
      })
    }
    await page.goto(server.resolvedUrls.local[0]); await settings()
    await page.getByRole('slider', { name: /文字サイズ/ }).press('Home')
    await page.getByRole('slider', { name: /行間/ }).press('Home')
    await page.getByRole('slider', { name: /本文の最大幅/ }).press('Home')
    await open(); const small = await values()
    assert.equal(small.fontSize, 13); assert.ok(Math.abs(small.lineHeight - 19.5) < 0.1)
    assert.ok(small.bodyWidth <= small.parentWidth)
    await page.locator('.article-detail').getByRole('button', { name: width < 768 ? '一覧に戻る' : '記事を閉じる', exact: true }).click()
    await settings()
    await page.getByRole('slider', { name: /文字サイズ/ }).press('End')
    await page.getByRole('slider', { name: /行間/ }).press('End')
    await page.getByRole('slider', { name: /本文の最大幅/ }).press('End')
    await open(); const large = await values()
    assert.equal(large.fontSize, 22); assert.ok(Math.abs(large.lineHeight - 52.8) < 0.1)
    assert.ok(large.bodyWidth <= large.parentWidth)
    assert.equal(large.maxWidth, large.expectedMax)
    if (width >= 1024) assert.ok(large.bodyWidth > small.bodyWidth * 1.5, `${small.bodyWidth} -> ${large.bodyWidth}`)
    assert.equal(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth), true)
    await page.reload(); await page.locator('.article-text').waitFor(); const reloaded = await values()
    assert.equal(reloaded.fontSize, 22); assert.ok(Math.abs(reloaded.lineHeight - 52.8) < 0.1); assert.ok(Math.abs(reloaded.bodyWidth - large.bodyWidth) < 1)
    await page.locator('.article-detail').getByRole('button', { name: width < 768 ? '一覧に戻る' : '記事を閉じる', exact: true }).click()
    await settings(); await page.getByRole('button', { name: '初期値に戻す' }).click(); await open()
    const reset = await values(); assert.equal(reset.fontSize, 16); assert.ok(Math.abs(reset.lineHeight - 30.4) < 0.1)
    assert.ok(reset.bodyWidth <= reset.parentWidth)
    if (width >= 1024) assert.ok(reset.bodyWidth > small.bodyWidth && reset.bodyWidth < large.bodyWidth)
    console.log(`${width}px: computed size, line height, width, persistence and reset passed`)
    await context.close()
  }
} finally { if (browser) await browser.close(); await server.close() }
