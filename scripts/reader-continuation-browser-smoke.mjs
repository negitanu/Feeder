// Isolated generated demo database; never targets a running installation.
import assert from 'node:assert/strict'
import { spawn } from 'node:child_process'
import { mkdtemp, rm } from 'node:fs/promises'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import { createRequire } from 'node:module'
import { createServer } from 'node:net'
const require = createRequire(new URL('../frontend/package.json', import.meta.url))
const { chromium } = require('@playwright/test')
const dir = await mkdtemp(join(tmpdir(), 'feeder-continuation-'))
const reservation = createServer()
await new Promise((resolve, reject) => reservation.once('error', reject).listen(0, '127.0.0.1', resolve))
const port = reservation.address().port
await new Promise(resolve => reservation.close(resolve))
const origin = `http://127.0.0.1:${port}`
const server = spawn(process.env.FEEDER_SMOKE_BINARY || './backend/target/debug/feeder', [], {
  env: { PATH: process.env.PATH, FEEDER_DB: join(dir, 'demo.sqlite3'), FEEDER_BIND: `127.0.0.1:${port}`, FEEDER_ALLOWED_HOSTS: `127.0.0.1:${port}`, FEEDER_ACCESS_MODE: 'local', FEEDER_DEMO: '1' }, stdio: 'ignore'
})
let browser
try {
  let ready = false
  for (let i = 0; i < 100; i++) {
    if (server.exitCode !== null) throw new Error(`isolated server exited ${server.exitCode}`)
    try { if ((await fetch(`${origin}/healthz`)).ok) { ready = true; break } } catch {}
    await new Promise(resolve => setTimeout(resolve, 100))
  }
  assert.ok(ready, 'isolated server did not start')
  browser = await chromium.launch({ headless: true, ...(process.env.FEEDER_CHROMIUM_PATH ? { executablePath: process.env.FEEDER_CHROMIUM_PATH } : {}) })
  for (const width of [1280, 390]) for (const theme of ['light', 'dark']) {
    const context = await browser.newContext({ viewport: { width, height: 844 }, hasTouch: width === 390 })
    await context.addInitScript(theme => { delete window.IntersectionObserver; localStorage.setItem('feeder-theme', theme) }, theme)
    const page = await context.newPage()
    const errors = []; page.on('pageerror', error => errors.push(error.message))
    await page.route('**/api/articles/*/hero', route => route.fulfill({ status: 404, body: '' }))
    await page.goto(origin)
    await page.waitForFunction(() => document.querySelectorAll('.article-card').length === 24 && document.querySelector('.article-list')?.getAttribute('aria-busy') === 'false')
    await page.locator('.article-select').first().check()
    let requests = 0
    await page.route('**/api/articles?*', async route => {
      if (new URL(route.request().url()).searchParams.has('cursor') && requests++ === 0) return route.fulfill({ status: 503, body: 'unavailable' })
      return route.continue()
    })
    await page.getByRole('button', { name: 'さらに読み込む' }).click()
    const retry = page.getByRole('button', { name: '続きの読込を再試行' })
    await retry.waitFor()
    assert.equal(await page.locator('.article-card').count(), 24)
    assert.equal(await page.locator('.article-select:checked').count(), 1)
    assert.equal(requests, 1)
    await retry.focus(); await retry.press('Enter')
    await page.waitForFunction(() => document.querySelectorAll('.article-card').length === 36)
    assert.equal(await page.locator('.article-select:checked').count(), 1)
    assert.equal(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth), true)
    assert.deepEqual(errors, [])
    await context.close()
  }
  // A conflicting cursor preserves the loaded selection until the user explicitly refreshes.
  const context = await browser.newContext()
  await context.addInitScript(() => { delete window.IntersectionObserver })
  const page = await context.newPage()
  await page.goto(origin)
  await page.waitForFunction(() => document.querySelectorAll('.article-card').length === 24)
  await page.locator('.article-select').first().check()
  await page.route('**/api/articles?*', route => new URL(route.request().url()).searchParams.has('cursor') ? route.fulfill({ status: 409, body: 'conflict' }) : route.continue())
  await page.getByRole('button', { name: 'さらに読み込む' }).click()
  const refresh = page.getByRole('button', { name: '最新の一覧を読み直す' })
  await refresh.waitFor()
  assert.equal(await page.locator('.article-select:checked').count(), 1)
  assert.equal(await page.locator('.article-card').count(), 24)
  await refresh.click()
  await page.waitForFunction(() => document.querySelectorAll('.article-card').length === 24 && !document.querySelector('.error-message'))
  await context.close()
  console.log('Continuation browser passed: 503 retry and 409 reload; selection preserved; desktop/mobile and light/dark.')
} finally {
  if (browser) await browser.close()
  server.kill('SIGTERM')
  await new Promise(resolve => { if (server.exitCode !== null) resolve(); else server.once('exit', resolve) })
  await rm(dir, { recursive: true, force: true })
}
