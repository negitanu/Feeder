import {browserSession} from './browser-session.mjs';
import assert from 'node:assert/strict';
import {createRequire} from 'node:module';
const require=createRequire(new URL('../frontend/package.json',import.meta.url));
const {chromium}=require('@playwright/test');
const browser=await chromium.launch({headless:true});
const origin=process.env.FEEDER_URL||'http://127.0.0.1:8080';
try {
 const context=await browser.newContext({viewport:{width:1280,height:800}});
 await context.addInitScript(()=>{delete window.IntersectionObserver});
 const page=await context.newPage(),errors=[];page.on('pageerror',e=>errors.push(e.message));await browserSession(page,origin);
 const items=await(await page.request.get(origin+'/api/articles?limit=100')).json();const ids=items.items.map(a=>a.id);
 assert.equal((await page.request.post(origin+'/api/articles/bulk-state',{data:{ids,read:false,starred:false,filters:{}}})).status(),200);
 await page.goto(origin);await page.locator('.article-card').first().waitFor();assert.equal(await page.locator('.article-card').count(),24);
 const first=page.locator('.article-open').nth(0),second=page.locator('.article-open').nth(1);const firstId=await first.getAttribute('data-article-id');
 const beforeWidth=await page.locator('.article-list').evaluate(el=>el.getBoundingClientRect().width);
 await page.locator('.article-select').nth(1).check();await first.click();await page.locator('.article-detail h1').waitFor();
 assert.equal(await page.locator('.article-card').count(),24);assert.equal(await page.locator('.article-select:checked').count(),1);
 assert.equal(await page.locator('.article-list').evaluate(el=>el.getBoundingClientRect().width),beforeWidth);
 let bounds=await page.evaluate(()=>{const d=document.querySelector('.article-detail').getBoundingClientRect(),h=document.querySelector('.topbar').getBoundingClientRect();return {top:d.top,bottom:d.bottom,header:h.bottom,height:innerHeight}});
 assert.ok(bounds.top>=bounds.header&&bounds.bottom<=bounds.height);
 // Clicking outside must close the detail and complete the checkbox operation.
 await page.locator('.article-select').first().check();assert.equal(await page.locator('.article-detail').count(),0);assert.equal(await page.locator('.article-select:checked').count(),2);
 await first.click();await page.locator('.article-detail h1').waitFor();const outsideCheck=page.locator('.article-select').nth(1);await outsideCheck.focus();await outsideCheck.press('Space');await page.locator('.article-detail').waitFor({state:'detached'});assert.equal(await outsideCheck.isChecked(),false);
 // Delay an already committed visit, dismiss its pending panel, then deliver it.
 assert.equal((await page.request.post(origin+`/api/articles/${firstId}/state`,{data:{read:false}})).status(),200);await page.reload();await first.waitFor();assert.equal(await first.getAttribute('data-article-id'),firstId);assert.equal(await first.locator('xpath=..').evaluate(el=>el.classList.contains('read')),false);
 let release,arrived;let gate=new Promise(resolve=>release=resolve),started=new Promise(resolve=>arrived=resolve);
 await page.route(`**/api/articles/${firstId}/visit`,async route=>{const response=await route.fetch();arrived();await gate;await route.fulfill({response})},{times:1});
 await first.click();await started;await page.getByText('記事を読み込んでいます...',{exact:true}).waitFor();await page.keyboard.press('Escape');assert.equal(await page.locator('.article-detail').count(),0);release();await page.unrouteAll({behavior:'wait'});assert.equal(await page.locator('.article-detail').count(),0);assert.ok(await first.locator('xpath=..').evaluate(el=>el.classList.contains('read')));
 // A later request wins even when an earlier article's response arrives last.
 gate=new Promise(resolve=>release=resolve);started=new Promise(resolve=>arrived=resolve);
 await page.route(`**/api/articles/${firstId}/visit`,async route=>{const response=await route.fetch();arrived();await gate;await route.fulfill({response})},{times:1});
 await first.click();await started;const secondTitle=await second.locator('strong').innerText();await second.focus();await second.press('Enter');await page.locator('.article-detail h1').filter({hasText:secondTitle}).waitFor();release();await page.unrouteAll({behavior:'wait'});assert.equal(await page.locator('.article-detail h1').innerText(),secondTitle);
 await page.getByRole('button',{name:/一覧に戻る/}).click();await page.waitForFunction(id=>document.activeElement?.getAttribute('data-article-id')===id,await second.getAttribute('data-article-id'));
 // The panel remains within the viewport after scroll/resize and does not jump the list on close.
 await page.setViewportSize({width:390,height:844});await page.getByRole('combobox',{name:'テーマ'}).selectOption('dark');await first.scrollIntoViewIfNeeded();await first.click();await page.locator('.article-detail h1').waitFor();
 bounds=await page.evaluate(()=>{const d=document.querySelector('.article-detail').getBoundingClientRect(),h=document.querySelector('.topbar').getBoundingClientRect();return {top:d.top,bottom:d.bottom,header:h.bottom,height:innerHeight}});assert.ok(bounds.top>=bounds.header&&bounds.bottom<=bounds.height);
 await page.evaluate(()=>window.scrollTo(0,0));await page.evaluate(()=>{const sidebar=document.querySelector('.sidebar');sidebar.style.minHeight=`${sidebar.getBoundingClientRect().height+40}px`});
 await page.waitForFunction(()=>{const panel=document.querySelector('.article-detail').getBoundingClientRect(),main=document.querySelector('.main-area').getBoundingClientRect(),header=document.querySelector('.topbar').getBoundingClientRect();return Math.abs(panel.top-Math.min(innerHeight-24,Math.max(0,main.top,header.bottom)+8))<2});
 const y=await page.evaluate(()=>window.scrollY);await page.getByRole('button',{name:/一覧に戻る/}).click();assert.ok(Math.abs((await page.evaluate(()=>window.scrollY))-y)<2);assert.equal(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth),true);
 const touchContext=await browser.newContext({viewport:{width:390,height:844},hasTouch:true,isMobile:true});const touch=await touchContext.newPage();await touch.goto(origin);await touch.locator('.article-open').first().tap();await touch.locator('.article-detail h1').waitFor();await touch.getByRole('button',{name:'集める',exact:true}).tap();await touch.getByRole('heading',{name:'フィードを集める'}).waitFor();assert.equal(await touch.locator('.article-detail').count(),0);await touchContext.close();
 for(const read of [false,true])for(const starred of [false,true]){const restore=items.items.filter(item=>item.is_read===read&&item.is_starred===starred).map(item=>item.id);if(restore.length)assert.equal((await page.request.post(origin+'/api/articles/bulk-state',{data:{ids:restore,read,starred,filters:{}}})).status(),200)}
 assert.deepEqual(errors,[]);await context.close();console.log('Floating detail passed: outside actions, delayed visits, switching, focus, retained list and responsive bounds.');
}finally{await browser.close()}
