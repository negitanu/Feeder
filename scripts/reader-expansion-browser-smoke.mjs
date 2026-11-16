// Integrated smoke: owns its backend process and temporary database. Never point it
// at an existing Feeder deployment. Build frontend/dist and backend first.
import assert from 'node:assert/strict'
import { createRequire } from 'node:module'
import { spawn, spawnSync } from 'node:child_process'
import { mkdtemp, rm } from 'node:fs/promises'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import { setTimeout as delay } from 'node:timers/promises'
import { browserSession } from './browser-session.mjs'

const require = createRequire(new URL('../frontend/package.json', import.meta.url))
const { chromium } = require('@playwright/test')
const base = new URL(process.env.FEEDER_BASE_URL || 'http://127.0.0.1:18763')
assert(base.protocol === 'http:' && base.hostname === '127.0.0.1' && base.port && base.pathname === '/' && !base.search && !base.hash, 'FEEDER_BASE_URL must be an unused http://127.0.0.1:PORT/')
const dir = await mkdtemp(join(tmpdir(), 'feeder-expansion-'))
const db = join(dir, 'fixture.sqlite3')
let server, browser
try {
  // A separate process on an exclusive port prevents accidental use of a live DB.
  server = spawn(process.env.FEEDER_SMOKE_BINARY || './backend/target/debug/feeder', [], {
    env: { PATH: process.env.PATH, HOME: dir, FEEDER_DB: db, FEEDER_BIND: `${base.hostname}:${base.port}`, FEEDER_DEMO: '0', FEEDER_ACCESS_MODE: 'local' }, stdio: ['ignore', 'ignore', 'pipe'],
  })
  let stderr = ''; server.stderr.on('data', chunk => { stderr += chunk })
  let exited = false; server.on('exit', () => { exited = true })
  for (let i = 0; i < 100 && !stderr.includes('Feeder listening'); i++) {
    if (exited) throw new Error(`Fixture backend failed: ${stderr}`)
    await delay(100)
  }
  assert(stderr.includes('Feeder listening'), `Fixture backend did not start: ${stderr}`)
  const seed = spawnSync('python3', ['-c', `
import sqlite3, sys
c=sqlite3.connect(sys.argv[1])
c.execute("INSERT INTO feeds(id,url,title,paused) VALUES(1,'https://one.example.invalid/feed','Synthetic Feed Alpha',0)")
c.execute("INSERT INTO feeds(id,url,title,paused) VALUES(2,'https://two.example.invalid/feed','Synthetic Feed Beta',1)")
c.execute("INSERT INTO feed_poll_intervals VALUES(1,120)")
for i in range(1,15):
 title = ('Synthetic mute target' if i == 1 else 'Synthetic duplicate' if i in (2,3) else 'Synthetic article %02d'%i)
 url = ('https://article.example.invalid/duplicate?utm_source=x' if i==2 else 'https://article.example.invalid/duplicate#copy' if i==3 else 'https://article.example.invalid/%d'%i)
 date = 'invalid-date' if i==14 else c.execute("SELECT datetime('now','-1 hour')").fetchone()[0]
 c.execute('INSERT INTO articles(id,title,content,url,published,feed_id,is_read,is_starred) VALUES(?,?,?,?,?,?,?,?)',(i,title,'Synthetic fixture body',url,date,1 if i != 3 else 2,0,1 if i==1 else 0))
c.commit()
`, db], { encoding: 'utf8' })
  assert.equal(seed.status, 0, `Fixture seed failed: ${seed.stderr}`)
  browser = await chromium.launch({ headless: true, ...(process.env.FEEDER_CHROMIUM_PATH ? { executablePath: process.env.FEEDER_CHROMIUM_PATH } : {}) })
  const origin = base.origin
  const ctx = await browser.newContext({ viewport: { width: 1280, height: 800 } })
  const page = await ctx.newPage()
  const errors = []; page.on('pageerror', e => errors.push(e.message))
  // Block all remote traffic including article artwork and external social fetches.
  await ctx.route('**/*', route => new URL(route.request().url()).origin === origin ? route.continue() : route.abort())
  await browserSession(page, origin)
  const get = async path => { const r = await page.request.get(origin + path); assert.equal(r.status(), 200, path); return r.json() }
  const send = async (path, method, data, status = 200) => { const r = await page.request.fetch(origin + path, { method, data }); assert.equal(r.status(), status, `${method} ${path}: ${await r.text()}`); return status === 204 ? null : r.json() }
  await page.goto(origin)
  await page.locator('.article-card').first().waitFor()
  const row = page.locator('[data-article-id="1"]')
  await row.waitFor(); assert.match(await row.innerText(), /Synthetic Feed Alpha/)
  assert.match(await row.innerText(), /Synthetic mute target/)
  assert.match(await row.locator('time').getAttribute('aria-label'), /\d{4}-\d\d-\d\d.*(?:UTC|JST)/)
  await row.focus(); await page.keyboard.press('Enter')
  await page.getByRole('button', { name: '記事を閉じる' }).click()
  // Exercise the row bookmark in both directions, then the filtered visit flow.
  const bookmarkRow = page.locator('[data-article-id="2"]').locator('..')
  await bookmarkRow.getByRole('button', { name: 'あとで読むに保存' }).click()
  await bookmarkRow.getByRole('button', { name: 'あとで読むから削除' }).click()
  await bookmarkRow.getByRole('button', { name: 'あとで読むに保存' }).click()
  await page.locator('.filters').getByRole('button', { name: 'あとで読む', exact: true }).click()
  await page.locator('[data-article-id="2"]').waitFor()
  await page.waitForFunction(() => document.querySelectorAll('.article-card').length === 1)
  await page.locator('[data-article-id="2"]').click()
  await page.getByRole('button', { name: '記事を閉じる' }).click()
  assert((await get('/api/read-later')).items.some(a => a.id === 2))
  await page.getByRole('button', { name: '設定', exact: true }).click()
  await page.getByRole('button', { name: '表示・読書とデータ' }).click()
  let [patch] = await Promise.all([page.waitForResponse(r => r.url().endsWith('/api/read-later/settings') && r.request().method() === 'PATCH'), page.getByLabel('記事を開いたときにあとで読むから自動で削除する').click()]); assert.equal(patch.status(), 200)
  await page.getByRole('button', { name: '読む', exact: true }).click()
  await page.locator('[data-article-id="2"]').click()
  await page.getByRole('button', { name: '記事を閉じる' }).click()
  await page.waitForFunction(() => !document.querySelector('[data-article-id="2"]'))
  assert(!(await get('/api/read-later')).items.some(a => a.id === 2))
  await page.getByRole('button', { name: '設定', exact: true }).click()
  ;[patch] = await Promise.all([page.waitForResponse(r => r.url().endsWith('/api/read-later/settings') && r.request().method() === 'PATCH'), page.getByLabel('記事を開いたときにあとで読むから自動で削除する').click()]); assert.equal(patch.status(), 200)
  await page.getByRole('button', { name: '読む', exact: true }).click()
  await page.locator('.filters button').filter({ hasText: 'すべて' }).first().click()
  await page.locator('[data-article-id="1"]').waitFor()
  // Saved-search and shared-search UI are no longer present in the reader.
  // Search the current reader before continuing with mute and collection checks.
  const search = page.getByRole('textbox', { name: '記事を検索', exact: true })
  await search.fill('Synthetic'); await search.press('Enter')
  await send('/api/articles/1/state', 'POST', { read: false })
  const rule = await send('/api/mute-rules', 'POST', { name: 'Synthetic rule', term: 'mute target', field: 'title', feed_id: null, enabled: true }, 201)
  assert((await get('/api/articles/1/mute-reasons')).reasons.some(x => x.id === rule.id))
  assert(!(await get('/api/articles?limit=100')).items.some(x => x.id === 1))
  assert((await get('/api/articles?limit=100&include_muted=true')).items.some(x => x.id === 1))
  await page.reload(); await page.locator('.article-card').first().waitFor()
  await page.getByRole('button', { name: '全て既読', exact: true }).click()
  await page.waitForFunction(() => document.querySelector('.article-card.read'))
  let hidden = await get('/api/articles/1'); assert.equal(hidden.is_read, false) // bulk excludes the muted row
  assert.equal(hidden.is_starred, true)
  await page.getByLabel('ミュートした記事も表示').check()
  await page.locator('[data-article-id="1"]').waitFor()
  await page.locator('[data-article-id="1"]').click()
  await page.locator('.detail-mute-reasons').getByText('ミュート条件: Synthetic rule').waitFor()
  await page.getByRole('button', {name: '記事を閉じる'}).click()
  await page.getByLabel('ミュートした記事も表示').uncheck()
  await page.locator('[data-article-id="1"]').waitFor({ state: 'detached' })
  const duplicates = await get('/api/duplicates?offset=0&limit=20')
  assert(duplicates.groups.some(g => [2, 3].every(id => g.articles.some(a => a.id === id))))
  assert.equal((await get('/api/articles/2')).url, 'https://article.example.invalid/duplicate?utm_source=x')
  assert.equal((await get('/api/articles/3')).url, 'https://article.example.invalid/duplicate#copy')
  const digest = await get('/api/digest?hours=24')
  assert.equal(digest.total, 12); assert.equal(digest.sources.reduce((n, s) => n + s.count, 0), 12)
  assert(!digest.sources.flatMap(s => s.items).some(a => a.id === 1))
  assert(!digest.sources.flatMap(s => s.items).some(a => a.id === 14))
  await page.getByRole('navigation', { name: 'メインナビゲーション' }).getByRole('button', { name: '設定' }).click()
  await page.getByRole('button', { name: '連携・ミュート' }).click()
  const social = page.locator('.social-sources')
  await social.getByLabel('サーバー URL').fill('https://social.example.invalid')
  await social.getByLabel('アカウント').fill('')
  await social.getByRole('button', { name: '購読を追加' }).click()
  assert.equal(await social.getByLabel('アカウント').evaluate(e => e.validity.valueMissing), true)
  assert.deepEqual((await get('/api/social/sources')).sources, [])
  await page.getByRole('button', { name: '購読・収集', exact: true }).click()
  const healthPanel = page.locator('.subscription-manager')
  await healthPanel.locator('li').filter({ hasText: 'Synthetic Feed Beta' }).getByText('停止中', { exact: false }).waitFor()
  const alpha = healthPanel.locator('li').filter({ hasText: 'Synthetic Feed Alpha' })
  await alpha.getByText('収集状態の詳細').click()
  await alpha.getByText('120 秒（個別）').waitFor()
  const health = await get('/api/collection/health')
  assert.equal(health.feeds.find(f => f.id === 2).next_due, null)
  assert.equal(health.feeds.find(f => f.id === 1).poll_interval_seconds, 120)
  await send('/api/articles/1/read-later', 'POST', { saved: true })
  assert.equal((await get('/api/read-later/settings')).auto_remove, false)
  await send('/api/articles/1/visit', 'POST', {})
  assert((await get('/api/read-later')).items.some(a => a.id === 1))
  await send('/api/read-later/settings', 'PATCH', { auto_remove: true })
  await send('/api/articles/1/visit', 'POST', {})
  assert(!(await get('/api/read-later')).items.some(a => a.id === 1))
  hidden = await get('/api/articles/1'); assert(hidden.is_read && hidden.is_starred)
  await page.getByRole('navigation', { name: 'メインナビゲーション' }).getByRole('button', { name: '読む', exact: true }).click()
  await page.locator('.article-open').first().waitFor()
  await page.getByLabel('記事の表示形式').selectOption('line')
  const columns = await page.locator('.article-row-content').first().evaluate(e => { const [source,title,time]=e.children; return [source.getBoundingClientRect().x,title.getBoundingClientRect().x,time.getBoundingClientRect().x] })
  assert(columns[0] < columns[1] && columns[1] < columns[2], 'Feed, title, absolute time column order')
  for (const viewport of [{ width: 1280, height: 800 }, { width: 390, height: 844 }]) {
    await page.setViewportSize(viewport)
    for (const theme of ['light', 'dark']) {
      await page.emulateMedia({ colorScheme: theme }); await page.reload(); await page.locator('.article-card').first().waitFor()
      assert(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth), `${viewport.width}px ${theme} overflow`)
    }
  }
  assert.deepEqual(errors, [])
  console.log('Reader expansion smoke passed on an isolated synthetic database.')
} finally {
  await browser?.close()
  if (server && server.exitCode === null) { server.kill('SIGTERM'); await Promise.race([new Promise(resolve => server.once('exit', resolve)), delay(3000)]) }
  await rm(dir, { recursive: true, force: true })
}
