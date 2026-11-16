import { browserSession } from './browser-session.mjs'
import assert from 'node:assert/strict'
import { createRequire } from 'node:module'
const require = createRequire(new URL('../frontend/package.json', import.meta.url))
const { chromium } = require('@playwright/test')
const browser = await chromium.launch({ headless: true, ...(process.env.FEEDER_CHROMIUM_PATH ? { executablePath: process.env.FEEDER_CHROMIUM_PATH } : {}) })
const origin = process.env.FEEDER_URL || 'http://127.0.0.1:8080'
try {
  const context = await browser.newContext({ viewport: { width: 1280, height: 800 } })
  await context.addInitScript(() => { delete window.IntersectionObserver })
  const page = await context.newPage(), errors = []
  page.on('pageerror', e => errors.push(e.message)); await browserSession(page, origin)
  assert.equal((await page.request.patch(origin + '/api/reader/settings', { data: { default_view: 'all', display_timezone: 'Asia/Tokyo' } })).status(), 200)
  const items = (await (await page.request.get(origin + '/api/articles?limit=100')).json()).items
  assert.ok(items.length >= 24)
  assert.equal((await page.request.post(origin + '/api/articles/bulk-state', { data: { ids: items.map(a => a.id), read: false, starred: false, filters: {} } })).status(), 200)
  await page.goto(origin); await page.locator('.article-card').first().waitFor()
  assert.equal(await page.locator('.article-card').count(), 24)
  const first = page.locator('.article-open').nth(0), second = page.locator('.article-open').nth(1)
  const firstId = await first.getAttribute('data-article-id'), secondId = await second.getAttribute('data-article-id')
  const firstTitle = await first.locator('strong').innerText(), secondTitle = await second.locator('strong').innerText()
  const beforeWidth = await page.locator('.article-list').evaluate(el => el.getBoundingClientRect().width)
  await page.locator('.article-select').nth(1).check({ force: true }); await first.click(); await page.locator('.article-detail h1').waitFor()
  assert.equal(await page.locator('.article-card').count(), 24); assert.equal(await page.locator('.article-select:checked').count(), 1)
  assert.ok(await page.locator('.article-list').evaluate(el => el.getBoundingClientRect().width) < beforeWidth)
  assert.match(await page.locator('.article-detail time').innerText(), /^\d{4}-\d{2}-\d{2} \d{2}:\d{2} JST$/)
  assert.deepEqual(await page.locator('.article-body').evaluate(el => [...el.children].filter(child => child.tagName !== 'IMG' || !child.hidden).map(child => child.tagName)), ['H1', 'DIV', 'HR', 'DIV', 'HR'])
  const toolbar = page.locator('.detail-toolbar-navigation')
  const previous = toolbar.getByRole('button', { name: '前の記事' }), next = toolbar.getByRole('button', { name: '次の記事' })
  assert.equal(await previous.isDisabled(), true)
  await page.route(`**/api/articles/${secondId}/visit`, route => route.fulfill({ status: 503, body: 'unavailable' }), { times: 1 })
  await next.click(); await page.locator('.article-detail [role="alert"]').waitFor(); assert.equal(await page.locator('.article-detail h1').innerText(), firstTitle)
  await next.click(); await page.locator('.article-detail h1').filter({ hasText: secondTitle }).waitFor(); assert.equal(await page.locator('.article-select:checked').count(), 1)
  await previous.click(); await page.locator('.article-detail h1').filter({ hasText: firstTitle }).waitFor()
  await page.getByRole('button', { name: '記事を閉じる', exact: true }).focus()
  for (const options of [{ ctrlKey: true }, { altKey: true }, { metaKey: true }, { shiftKey: true }, { repeat: true }, { isComposing: true }]) assert.equal(await page.evaluate(options => document.activeElement.dispatchEvent(new KeyboardEvent('keydown', { key: 'ArrowRight', bubbles: true, cancelable: true, ...options })), options), true)
  assert.equal(await page.evaluate(() => { const input = document.createElement('input'); document.querySelector('.article-detail').append(input); input.focus(); const result = input.dispatchEvent(new KeyboardEvent('keydown', { key: 'ArrowRight', bubbles: true, cancelable: true })); input.remove(); return result }), true)
  await page.getByRole('button', { name: '記事を閉じる', exact: true }).focus(); await page.keyboard.press('ArrowRight'); await page.locator('.article-detail h1').filter({ hasText: secondTitle }).waitFor()
  await page.keyboard.press('ArrowLeft'); await page.locator('.article-detail h1').filter({ hasText: firstTitle }).waitFor()
  await page.getByRole('button', { name: '記事を閉じる', exact: true }).click(); await page.locator('.article-detail').waitFor({ state: 'detached' })
  assert.equal(await page.locator('.article-select:checked').count(), 1)
  // A delayed visit must not reopen a dismissed detail.
  let release, arrived; const gate = new Promise(resolve => release = resolve), started = new Promise(resolve => arrived = resolve)
  await page.route(`**/api/articles/${firstId}/visit`, async route => { const response = await route.fetch(); arrived(); await gate; await route.fulfill({ response }) }, { times: 1 })
  await first.click(); await started; await page.getByText('記事を読み込んでいます...', { exact: true }).waitFor(); await page.keyboard.press('Escape'); await page.locator('.article-detail').waitFor({ state: 'detached' }); release(); await page.unrouteAll({ behavior: 'wait' }); assert.equal(await page.locator('.article-detail').count(), 0)
  // A later visit wins even if an earlier response arrives last.
  let releaseEarlier, earlierArrived
  const earlierGate = new Promise(resolve => releaseEarlier = resolve), earlierStarted = new Promise(resolve => earlierArrived = resolve)
  await page.route(`**/api/articles/${firstId}/visit`, async route => { const response = await route.fetch(); earlierArrived(); await earlierGate; await route.fulfill({ response }) }, { times: 1 })
  await first.click(); await earlierStarted; await second.focus(); await second.press('Enter'); await page.locator('.article-detail h1').filter({ hasText: secondTitle }).waitFor()
  releaseEarlier(); await page.unrouteAll({ behavior: 'wait' }); assert.equal(await page.locator('.article-detail h1').innerText(), secondTitle)
  await page.getByRole('button', { name: '記事を閉じる', exact: true }).click(); await page.locator('.article-detail').waitFor({ state: 'detached' })
  await page.waitForFunction(id => document.activeElement?.getAttribute('data-article-id') === id, secondId)
  // At tablet width, an outside pointer or keyboard action closes the floating detail
  // without swallowing the action that caused dismissal.
  await page.setViewportSize({ width: 900, height: 800 })
  const help = page.locator('.shortcut-help')
  const summary = help.locator('summary')
  for (const method of ['pointer', 'keyboard']) {
    await first.scrollIntoViewIfNeeded(); await first.click(); await page.locator('.article-detail h1').waitFor()
    if (method === 'pointer') await summary.click()
    else { await summary.focus(); await page.keyboard.press('Enter') }
    await page.locator('.article-detail').waitFor({ state: 'detached' })
    assert.equal(await help.evaluate(el => el.open), true, `${method} action was swallowed`)
    await summary.click(); assert.equal(await help.evaluate(el => el.open), false)
  }
  await page.setViewportSize({ width: 390, height: 844 }); await first.scrollIntoViewIfNeeded(); await first.click(); await page.locator('.article-detail h1').waitFor()
  assert.equal(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth), true)
  await page.keyboard.press('Escape'); await page.locator('.article-detail').waitFor({ state: 'detached' })
  assert.deepEqual(errors, []); await context.close()
  console.log('Detail browser passed: split layout, navigation, failures, keyboard, selection, delayed visit and mobile.')
} finally { await browser.close() }
