import assert from 'node:assert/strict'
import { createRequire } from 'node:module'
import { fileURLToPath } from 'node:url'
const require = createRequire(new URL('../frontend/package.json', import.meta.url))
const { chromium } = require('@playwright/test')
const { createServer } = await import(fileURLToPath(new URL('../frontend/node_modules/vite/dist/node/index.js', import.meta.url)))
const root = fileURLToPath(new URL('../frontend', import.meta.url))
const server = await createServer({ root, server: { port: 0 } }); await server.listen()
let browser
try {
 browser = await chromium.launch({headless: true, ...(process.env.FEEDER_CHROMIUM_PATH ? {executablePath: process.env.FEEDER_CHROMIUM_PATH} : {})})
 for (const width of [1920, 1024, 768, 390, 320]) {
  const page = await browser.newPage({viewport:{width,height:850}})
  await page.route('**/api/**', route => {
   const path = new URL(route.request().url()).pathname
   const fixtures = {'/api/discover':{candidates:[]}, '/api/session':{csrf_token:'a'.repeat(64)}, '/api/library':{feeds:[{id:1,title:'Synthetic feed',url:'https://example.test/a-very-long-feed-url-without-spaces',folder_id:1,paused:false,poll_interval:null}],folders:[{id:1,name:'Research / News'}]}, '/api/reader/settings':{display_timezone:'UTC',default_view:'all'}, '/api/articles':{items:[{id:1,title:'Synthetic article',content:'Body',url:'https://example.test/article',published:'2026-01-01T00:00:00Z',is_read:false,is_starred:false,is_saved_for_later:false}],total:1,has_more:false,next_cursor:null}, '/api/library/unread':{count:0,folders:[]}, '/api/collection/settings':{enabled:false,interval_seconds:3600}, '/api/collection/health':{feeds:[],global_enabled:false,global_interval_seconds:3600}, '/api/read-later/settings':{auto_remove:false}, '/api/keys':{keys:[]}, '/api/keys/settings':{rate_limit:60}, '/api/mute-rules':{rules:[]}, '/api/social/sources':{sources:[]}}
   return route.fulfill({json:fixtures[path] ?? {}})
  })
  const origin=server.resolvedUrls.local[0]
  await page.goto(origin); await page.locator('.article-card').first().waitFor()
  if (width < 768) await page.getByRole('button',{name:'選択',exact:true}).click()
  await page.getByLabel('記事を選択: Synthetic article').check({force:true})
  await page.getByRole('button',{name:'設定',exact:true}).click()
  const table = page.locator('.subscription-table')
  await table.getByText('Synthetic feed').waitFor()
  const dimensions = await page.evaluate(() => { const wrap=document.querySelector('.subscription-table-wrap'); const table=document.querySelector('.subscription-table'); const button=document.querySelector('.subscription-row-actions button:last-child'); const box=wrap.getBoundingClientRect(); const b=button.getBoundingClientRect(); return {page:document.documentElement.scrollWidth, viewport:innerWidth, width:box.width, table:table.scrollWidth, left:b.left, right:b.right, wrapLeft:box.left, wrapRight:box.right} })
  assert.ok(dimensions.page <= dimensions.viewport, JSON.stringify(dimensions))
  assert.ok(dimensions.table <= dimensions.width + 1, JSON.stringify(dimensions))
  assert.ok(dimensions.left >= dimensions.wrapLeft && dimensions.right <= dimensions.wrapRight, JSON.stringify(dimensions))
  if (width === 1920) assert.ok(dimensions.width > 960, JSON.stringify(dimensions))
  await page.emulateMedia({ colorScheme: width === 390 || width === 1920 ? 'light' : 'dark' })
  await page.getByLabel('Synthetic feedを選択').focus()
  await page.keyboard.press('Space')
  assert.equal(await page.getByLabel('Synthetic feedを選択').isChecked(), true)
  await page.getByLabel('Synthetic feedを選択').uncheck()
  await page.getByLabel('Synthetic feedを選択').check()
  assert.equal(await page.getByLabel('Synthetic feedを選択').isChecked(), true)
  await page.getByText('収集状態の詳細').click()
  assert.equal(await page.locator('.subscription-row details').evaluate(el => el.open), true)
  await page.getByLabel('サイトまたはフィード URL').fill('https://example.test/draft')
  await page.getByRole('button',{name:'表示・読書とデータ'}).click()
  assert.equal(new URL(page.url()).hash,'#settings-category-reading')
  await page.getByRole('button',{name:'連携・ミュート'}).click()
  assert.equal(new URL(page.url()).hash,'#settings-category-integration')
  await page.goBack(); await page.getByRole('heading',{name:'表示設定',exact:true}).waitFor()
  await page.goBack(); await page.getByRole('heading',{name:'購読管理・収集状態',exact:true}).waitFor()
  assert.equal(await page.getByLabel('サイトまたはフィード URL').inputValue(), 'https://example.test/draft')
  await page.goBack(); await page.getByRole('heading',{name:'設定',exact:true}).waitFor({state:'hidden'})
  await page.waitForFunction(() => document.querySelector('.article-select')?.checked === true)
  assert.equal(await page.getByLabel('記事を選択: Synthetic article').isChecked(), true)
  await page.goForward(); await page.getByRole('heading',{name:'購読管理・収集状態',exact:true}).waitFor()
  await page.goForward(); await page.getByRole('heading',{name:'表示設定',exact:true}).waitFor()
  await page.goto(origin+'#settings-category-integration'); await page.getByRole('heading',{name:'APIキー',exact:true}).waitFor()
  await page.reload(); await page.getByRole('heading',{name:'APIキー',exact:true}).waitFor()
  await page.getByRole('link',{name:'Feeder ホーム'}).click(); await page.getByRole('heading',{name:'設定',exact:true}).waitFor({state:'hidden'}); await page.waitForFunction(() => !location.hash.startsWith('#settings-category-'))
  console.log(`${width}px: category history and direct URL passed`); await page.close()
 }
} finally { if(browser) await browser.close(); await server.close() }
