import {browserSession} from './browser-session.mjs';
import assert from 'node:assert/strict';
import {createRequire} from 'node:module';
const require=createRequire(new URL('../frontend/package.json',import.meta.url));const {chromium}=require('@playwright/test');const browser=await chromium.launch();const origin=process.env.FEEDER_URL||'http://127.0.0.1:8080';
try {
 const context=await browser.newContext({viewport:{width:390,height:844}});const page=await context.newPage();await browserSession(page,origin);await page.goto(origin);await page.getByRole('button',{name:'設定',exact:true}).click();await page.getByRole('slider',{name:/文字サイズ/}).press('End');await page.getByRole('slider',{name:/行間/}).press('End');await page.getByRole('slider',{name:/本文の最大幅/}).press('End');
 await page.reload();await page.locator('.article-open').first().waitFor();await page.locator('.article-open').first().click();await page.locator('.article-text').waitFor();assert.equal(await page.locator('.article-text').evaluate(el=>getComputedStyle(el).fontSize),'22px');assert.equal(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth),true);
 await page.getByRole('button',{name:'設定',exact:true}).click();await page.getByRole('button',{name:'初期値に戻す',exact:true}).click();assert.equal(await page.getByRole('slider',{name:/文字サイズ/}).inputValue(),'16');await context.close();
 const denied=await browser.newContext();await denied.addInitScript(()=>Object.defineProperty(window,'localStorage',{get(){throw Error('denied')}}));const blocked=await denied.newPage();await browserSession(blocked,origin);await blocked.goto(origin);await blocked.getByRole('button',{name:'設定',exact:true}).click();await blocked.getByRole('slider',{name:/文字サイズ/}).press('End');assert.match(await blocked.locator('.readability-status').textContent(),/保存できません/);await denied.close();console.log('Readability passed: bounded sliders, persistence, reset, mobile bounds and denied storage.');
}finally{await browser.close()}
