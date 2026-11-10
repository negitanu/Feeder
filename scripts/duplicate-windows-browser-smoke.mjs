// Requires a disposable Feeder instance; duplicate responses are synthetic.
import assert from 'node:assert/strict'
import { createRequire } from 'node:module'
import { browserSession } from './browser-session.mjs'
const require = createRequire(new URL('../frontend/package.json', import.meta.url))
const { chromium } = require('@playwright/test')
const origin = process.env.FEEDER_URL || 'http://127.0.0.1:18775'
const browser = await chromium.launch({headless: true, ...(process.env.FEEDER_CHROMIUM_PATH ? {executablePath: process.env.FEEDER_CHROMIUM_PATH} : {})})
try {
  const page = await browser.newPage({viewport: {width: 390, height: 844}})
  await browserSession(page, origin)
  await page.route('**/api/duplicates/members?*', async route => {
    assert.equal(new URL(route.request().url()).searchParams.get('before_id'), '101')
    await route.fulfill({json:{articles:[{id:100,title:'続きの記事',url:'https://example.invalid/a',feed_id:2,feed_title:'Fixture',is_read:false,is_starred:false}],next_before_id:null}})
  })
  await page.route('**/api/duplicates?*', async route => {
    const older = new URL(route.request().url()).searchParams.get('before_id') === '3'
    await route.fulfill({json:{groups: older ? [{canonical_url:'https://example.invalid/a',match_count:3,members_truncated:true,articles:[{id:102,title:'古い重複',url:'https://example.invalid/a',feed_id:1,feed_title:'Fixture',is_read:false,is_starred:false},{id:101,title:'古い重複',url:'https://example.invalid/a',feed_id:2,feed_title:'Fixture',is_read:false,is_starred:false}]}] : [],group_count:older?1:0,article_count:older?3:0,scanned:older?2:1000,truncated:!older,offset:0,limit:20,has_more:false,next_before_id:older?null:3}})
  })
  await page.goto(origin)
  await page.locator('summary').filter({hasText:'重複記事'}).click()
  await page.getByRole('button',{name:'重複記事を表示'}).click()
  await page.getByText('さらに古い記事があります',{exact:false}).waitFor()
  await page.getByRole('button',{name:'古い範囲へ'}).click()
  await page.getByRole('heading',{name:/古い重複/}).waitFor()
  await page.getByRole('button',{name:'続きを表示'}).click()
  await page.getByRole('button',{name:'続きの記事'}).waitFor()
  assert.equal(await page.getByRole('button',{name:'続きを表示'}).count(),0)
  assert.equal(await page.getByRole('button',{name:'古い範囲へ'}).isDisabled(),true)
  await page.getByRole('button',{name:'新しい範囲へ'}).click()
  await page.getByText('さらに古い記事があります',{exact:false}).waitFor()
  assert.equal(await page.evaluate(()=>document.documentElement.scrollWidth <= innerWidth),true)
  console.log('duplicate older and newer windows, mobile layout: passed')
} finally { await browser.close() }
