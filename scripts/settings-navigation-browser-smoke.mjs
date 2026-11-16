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
 for (const width of [1440, 390]) {
  const page = await browser.newPage({viewport:{width,height:850}})
  await page.route('**/api/**', route => {
   const path = new URL(route.request().url()).pathname
   const fixtures = {'/api/discover':{candidates:[]}, '/api/session':{csrf_token:'a'.repeat(64)}, '/api/library':{feeds:[],folders:[]}, '/api/reader/settings':{display_timezone:'UTC',default_view:'all'}, '/api/articles':{items:[{id:1,title:'Synthetic article',content:'Body',url:'https://example.test/article',published:'2026-01-01T00:00:00Z',is_read:false,is_starred:false,is_saved_for_later:false}],total:1,has_more:false,next_cursor:null}, '/api/library/unread':{count:0,folders:[]}, '/api/collection/settings':{enabled:false,interval_seconds:3600}, '/api/collection/health':{feeds:[],global_enabled:false,global_interval_seconds:3600}, '/api/read-later/settings':{auto_remove:false}, '/api/keys':{keys:[]}, '/api/keys/settings':{rate_limit:60}, '/api/mute-rules':{rules:[]}, '/api/social/sources':{sources:[]}}
   return route.fulfill({json:fixtures[path] ?? {}})
  })
  const origin=server.resolvedUrls.local[0]
  await page.goto(origin); await page.locator('.article-card').first().waitFor()
  if (width < 768) await page.getByRole('button',{name:'選択',exact:true}).click()
  await page.getByLabel('記事を選択: Synthetic article').check({force:true})
  await page.getByRole('button',{name:'設定',exact:true}).click()
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
