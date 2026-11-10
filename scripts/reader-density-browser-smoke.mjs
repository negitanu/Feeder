// Run only against a disposable demo instance with 36 generated articles.
import assert from 'node:assert/strict'
import { createRequire } from 'node:module'
import { browserSession } from './browser-session.mjs'
const require = createRequire(new URL('../frontend/package.json', import.meta.url))
const { chromium } = require('@playwright/test')
const origin = process.env.FEEDER_URL || 'http://127.0.0.1:18777'
const browser = await chromium.launch({headless:true,...(process.env.FEEDER_CHROMIUM_PATH?{executablePath:process.env.FEEDER_CHROMIUM_PATH}:{})})
try {
  for (const width of [320, 390, 1280]) for (const theme of ['light', 'dark']) {
    const context = await browser.newContext({viewport:{width,height:844},hasTouch:width<760})
    await context.addInitScript(theme => localStorage.setItem('feeder-theme',theme),theme)
    const page = await context.newPage(); await browserSession(page, origin); await page.goto(origin)
    await page.locator('.article-card').first().waitFor()
    for (const layout of ['standard','line','tiles']) {
      await page.getByRole('combobox',{name:'記事の表示形式'}).selectOption(layout)
      const row=page.locator('.article-card').first(), time=row.locator('time.card-meta')
      const full=await time.getAttribute('aria-label')
      assert.match(full,/^\d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{2} (?:JST|UTC)$/)
      assert.equal(await time.locator('.article-date-compact').isVisible(),width<=760)
      assert.equal(await time.locator('.article-date-full').isVisible(),width>760)
      assert.equal(await time.locator('.article-date-compact').textContent(),full.slice(5,10))
      const styles=await row.evaluate(el=>{const s=getComputedStyle(el);return {left:s.borderLeftWidth,bottom:s.borderBottomWidth,radius:s.borderTopLeftRadius,margin:s.marginBottom}})
      assert.deepEqual(styles,{left:'0px',bottom:'1px',radius:'0px',margin:'0px'})
      assert.equal(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth),true)
      if (layout==='line' && width===390 && theme==='dark') await row.screenshot({path:'/tmp/feeder-density-mobile.png'})
      if (layout==='line' && width===1280 && theme==='light') await page.locator('.article-list').screenshot({path:'/tmp/feeder-density-desktop.png'})
    }
    await page.getByRole('combobox',{name:'記事の表示形式'}).selectOption('line')
    const checkbox=page.getByRole('checkbox',{name:/記事を選択:/}).first()
    if (width <= 760) await checkbox.tap(); else await checkbox.check()
    assert.equal(await checkbox.isChecked(),true)
    if (width <= 760) {
      const actions=await page.locator('.article-card').first().locator('.read-later-button-compact, .card-star').evaluateAll(elements=>elements.map(element=>element.getBoundingClientRect().height))
      assert.ok(actions.every(height=>height>=44))
    }
    const article=page.locator('.article-open').first(); await article.focus(); await page.keyboard.press('Enter'); await page.getByRole('button',{name:/一覧に戻る/}).waitFor()
    await context.close()
  }
  console.log('reader density: PC/mobile, themes, layouts, dates, selection and keyboard passed')
} finally { await browser.close() }
