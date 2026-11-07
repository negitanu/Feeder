import assert from 'node:assert/strict'
import { createRequire } from 'node:module'
const require = createRequire(new URL('../frontend/package.json', import.meta.url))
const { chromium } = require('@playwright/test')
const browser = await chromium.launch({ headless: true })
const origin = process.env.FEEDER_URL || 'http://127.0.0.1:5173'
const article = id => ({ id, title: `Synthetic article ${id}`, content: 'fixture', url: 'https://example.test/', published: '2025-01-01T00:00:00Z', is_read: false, is_starred: false })
const csrf = 'a'.repeat(64) // synthetic token, never a real session secret
let visits = 0
try {
  const page = await browser.newPage()
  await page.route('**/api/**', async route => {
    const url = new URL(route.request().url())
    let body = {}
    if (url.pathname === '/api/session') body = { csrf_token: csrf }
    else if (url.pathname === '/api/reader/settings') body = { display_timezone: 'UTC', default_view: 'all' }
    else if (url.pathname === '/api/smart-folders') body = []
    else if (url.pathname === '/api/read-later') body = {items:[]}
    else if (url.pathname === '/api/read-later/settings') body = {auto_remove:false}
    else if (url.pathname === '/api/duplicates') body = {groups:[],group_count:0,article_count:0,scanned:0,truncated:false,offset:0,limit:20,has_more:false}
    else if (url.pathname === '/api/digest') body = {hours:24,total:0,sources:[]}
    else if (url.pathname === '/api/library') body = { folders: [], feeds: [] }
    else if (/\/api\/articles\/\d+\/visit$/.test(url.pathname)) { assert.equal(route.request().headers()['x-feeder-csrf'], csrf); visits++; body = { ...article(Number(url.pathname.split('/')[3])), is_read: true } }
    else if (url.pathname === '/api/articles') body = { items: [article(1), article(2)], total: 2, has_more: false, next_cursor: null }
    await route.fulfill({ json: body })
  })
  await page.goto(`${origin}/?view=all&search=fixture&article=1`)
  await page.locator('[data-article-id="1"]').waitFor()
  await page.waitForURL(/article=1/)
  await page.waitForTimeout(300)
  assert.equal(visits, 1)
  await page.keyboard.press('Escape')
  await page.waitForURL(url => !url.searchParams.has('article'))
  await page.goBack()
  await page.waitForURL(/article=1/)
  assert.equal(visits, 1)
  await page.goForward()
  await page.waitForURL(url => !url.searchParams.has('article'))
  assert.equal(visits, 1)
  assert.equal(await page.locator('.article-card').count(), 2)
  console.log('Reader history synthetic smoke passed')
} finally { await browser.close() }
