import {browserSession} from './browser-session.mjs';
import assert from 'node:assert/strict';
import {createRequire} from 'node:module';
const require=createRequire(new URL('../frontend/package.json',import.meta.url));
const {chromium}=require('@playwright/test');
const browser=await chromium.launch();
const origin=process.env.FEEDER_URL||'http://127.0.0.1:8080';
try {
 const context=await browser.newContext({viewport:{width:1280,height:800}});const page=await context.newPage();await browserSession(page,origin);await page.goto(origin);await page.locator('.article-card').first().waitFor();
 const search=page.getByRole('textbox',{name:'記事を検索',exact:true});await search.fill('サンプル');await page.getByRole('button',{name:'検索',exact:true}).click();
 const saved=page.locator('.saved-searches');await saved.locator('summary').click();await saved.getByLabel('検索の名前').fill('サンプル検索');await saved.getByRole('button',{name:'現在の検索を保存'}).click();await saved.getByRole('button',{name:'サンプル検索',exact:true}).waitFor();
 await page.reload();await page.locator('.article-card').first().waitFor();await page.locator('.article-select').first().check();await saved.locator('summary').click();await saved.getByRole('button',{name:'サンプル検索',exact:true}).click();assert.equal(await search.inputValue(),'サンプル');assert.equal(await page.locator('.article-select:checked').count(),0);
 await saved.getByRole('button',{name:'サンプル検索を削除'}).click();assert.equal(await saved.getByRole('button',{name:'サンプル検索',exact:true}).count(),0);
 await page.evaluate(()=>localStorage.setItem('feeder-saved-searches-v1','{"version":1,"items":[null,{"name":"invalid"}]}'));await page.reload();await saved.locator('summary').click();assert.equal(await saved.locator('li').count(),0);
 await context.close();console.log('Saved search passed: named save, reload, apply, selection reset, deletion and malformed storage.');
} finally {await browser.close()}
