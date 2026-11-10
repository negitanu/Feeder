// Uses a disposable Feeder instance; collection responses are synthetic.
import assert from 'node:assert/strict'
import { createRequire } from 'node:module'
import { browserSession } from './browser-session.mjs'
const require = createRequire(new URL('../frontend/package.json', import.meta.url))
const { chromium } = require('@playwright/test')
const origin = process.env.FEEDER_URL || 'http://127.0.0.1:18772'
const browser = await chromium.launch({ headless: true, ...(process.env.FEEDER_CHROMIUM_PATH ? { executablePath: process.env.FEEDER_CHROMIUM_PATH } : {}) })
try {
  const page = await browser.newPage()
  await browserSession(page, origin)
  let state = 'idle', reads = 0
  await page.route('**/api/collection/health', async route => {
    reads++
    const waiting = state === 'waiting'
    await route.fulfill({ json: { global_enabled: true, global_interval_seconds: 300, truncated: false, feeds: [{ id: 999, name: '合成フィード', url: 'https://example.invalid/feed', paused: false, last_checked: null, last_success: null, error: waiting ? 'Refresh failed (502); diagnostic=upstream_http_429' : null, next_due: null, consecutive_failures: waiting ? 1 : 0, stale: false, individual_override: false, poll_interval_seconds: null, collection_state: waiting ? 'waiting' : 'idle', next_retry: waiting ? '2099-01-01 00:00:00' : null, more_pending: false, gap_possible: false }] } })
  })
  await page.route('**/api/feeds/999/refresh', async route => { state = 'waiting'; await route.fulfill({ status: 502, body: '' }) })
  await page.goto(origin)
  await page.getByRole('button', { name: '設定', exact: true }).click()
  const row = page.getByRole('row').filter({ hasText: '合成フィード' })
  await row.getByRole('button', { name: '再試行' }).click()
  await row.getByText(/HTTP 429/).waitFor()
  assert.ok(reads >= 2)
  assert.equal(await row.getByRole('button', { name: '再試行' }).isDisabled(), true)
  assert.ok((await row.innerText()).includes('待機中'))
  console.log('failed retry refreshes diagnostics and disables retry while waiting: passed')
} finally { await browser.close() }
