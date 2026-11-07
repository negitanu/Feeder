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
 const firstTitle=await first.locator('strong').innerText(),nextTitle=await second.locator('strong').innerText(),secondId=await second.getAttribute('data-article-id');
 assert.match(await page.locator('.article-detail time').innerText(), /^\d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{2} JST$/);
 assert.deepEqual(await page.locator('.article-body').evaluate(el=>[...el.children].filter(child=>child.tagName!=='IMG' || !child.hidden).map(child=>child.tagName)),['H1','DIV','HR','DIV','HR']);
 const navigation=page.getByRole('navigation',{name:'記事の移動'});assert.equal(await navigation.getByRole('button',{name:'前の記事'}).isDisabled(),true);assert.equal(await navigation.locator('span').innerText(),'1 / 24');
 await page.route(`**/api/articles/${secondId}/visit`,route=>route.fulfill({status:503,body:'unavailable'}),{times:1});await navigation.getByRole('button',{name:'次の記事'}).press('Enter');await page.locator('.article-detail [role="alert"]').waitFor();assert.equal(await page.locator('.article-detail h1').innerText(),firstTitle);
 await navigation.getByRole('button',{name:'次の記事'}).press('Enter');await page.locator('.article-detail h1').filter({hasText:nextTitle}).waitFor();assert.equal(await navigation.locator('span').innerText(),'2 / 24');await page.waitForFunction(()=>document.activeElement?.textContent==='次の記事');assert.equal(await page.locator('.article-select:checked').count(),1);assert.equal(await page.locator('.article-card').count(),24);
 await navigation.getByRole('button',{name:'前の記事'}).press('Enter');await page.locator('.article-detail h1').filter({hasText:firstTitle}).waitFor();assert.equal(await navigation.locator('span').innerText(),'1 / 24');
 // Arrow keys are scoped to the detail and leave editing, modifiers, repeats and boundaries alone.
 await page.getByRole('button',{name:/一覧に戻る/}).focus();
 assert.equal(await page.evaluate(()=>document.activeElement.dispatchEvent(new KeyboardEvent('keydown',{key:'ArrowLeft',bubbles:true,cancelable:true}))),true);
 for(const options of [{ctrlKey:true},{altKey:true},{metaKey:true},{shiftKey:true},{repeat:true},{isComposing:true}])assert.equal(await page.evaluate(options=>document.activeElement.dispatchEvent(new KeyboardEvent('keydown',{key:'ArrowRight',bubbles:true,cancelable:true,...options})),options),true);
 assert.equal(await page.evaluate(()=>{const input=document.createElement('input');document.querySelector('.article-detail').append(input);input.focus();const result=input.dispatchEvent(new KeyboardEvent('keydown',{key:'ArrowRight',bubbles:true,cancelable:true}));input.remove();return result}),true);
 await second.focus();assert.equal(await page.evaluate(()=>document.activeElement.dispatchEvent(new KeyboardEvent('keydown',{key:'ArrowRight',bubbles:true,cancelable:true}))),true);assert.equal(await page.locator('.article-detail h1').innerText(),firstTitle);
 await page.getByRole('button',{name:/一覧に戻る/}).focus();await page.keyboard.press('ArrowRight');await page.locator('.article-detail h1').filter({hasText:nextTitle}).waitFor();await page.keyboard.press('ArrowLeft');await page.locator('.article-detail h1').filter({hasText:firstTitle}).waitFor();
 let bounds=await page.evaluate(()=>{const d=document.querySelector('.article-detail').getBoundingClientRect(),h=document.querySelector('.topbar').getBoundingClientRect();return {top:d.top,bottom:d.bottom,header:h.bottom,height:innerHeight}});
 assert.ok(bounds.top>=bounds.header&&bounds.bottom<=bounds.height);
 await page.locator('.list-heading').dispatchEvent('click',{button:0,detail:1});await page.locator('.detail-presence.is-leaving').waitFor();assert.equal(await page.locator('.detail-presence.is-leaving').getAttribute('inert'),'');await page.locator('.article-detail').waitFor({state:'detached'});await first.click();await page.locator('.article-detail h1').waitFor();
 // Clicking outside must close the detail and complete the checkbox operation.
 await page.locator('.article-select').first().check();await page.locator('.article-detail').waitFor({state:'detached'});assert.equal(await page.locator('.article-detail').count(),0);assert.equal(await page.locator('.article-select:checked').count(),2);
 await page.locator('.article-open').last().focus();await page.locator('.article-open').last().press('Enter');await page.locator('.article-detail h1').waitFor();assert.equal(await page.getByRole('navigation',{name:'記事の移動'}).locator('span').innerText(),'24 / 24');assert.equal(await page.getByRole('button',{name:'次の記事'}).isDisabled(),true);await page.getByRole('button',{name:/一覧に戻る/}).click();
 await first.click();await page.locator('.article-detail h1').waitFor();const outsideCheck=page.locator('.article-select').nth(1);await outsideCheck.focus();await outsideCheck.press('Space');await page.locator('.article-detail').waitFor({state:'detached'});assert.equal(await outsideCheck.isChecked(),false);
 // Delay an already committed visit, dismiss its pending panel, then deliver it.
 assert.equal((await page.request.post(origin+`/api/articles/${firstId}/state`,{data:{read:false}})).status(),200);await page.reload();await first.waitFor();assert.equal(await first.getAttribute('data-article-id'),firstId);assert.equal(await first.locator('xpath=..').evaluate(el=>el.classList.contains('read')),false);
 let release,arrived;let gate=new Promise(resolve=>release=resolve),started=new Promise(resolve=>arrived=resolve);
 await page.route(`**/api/articles/${firstId}/visit`,async route=>{const response=await route.fetch();arrived();await gate;await route.fulfill({response})},{times:1});
 await first.click();await started;await page.getByText('記事を読み込んでいます...',{exact:true}).waitFor();await page.keyboard.press('Escape');await page.locator('.article-detail').waitFor({state:'detached'});assert.equal(await page.locator('.article-detail').count(),0);release();await page.unrouteAll({behavior:'wait'});await page.waitForFunction(id=>document.querySelector(`[data-article-id="${id}"]`)?.parentElement?.classList.contains('read'),firstId);await page.locator('.article-detail').waitFor({state:'detached'});assert.equal(await page.locator('.article-detail').count(),0);
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
 // In unread mode the captured sequence can revisit articles removed from the live filter.
 await page.setViewportSize({width:1280,height:800});const unreadLoaded=page.waitForResponse(response=>response.url().includes('/api/articles?')&&response.url().includes('pagination=cursor')&&response.url().includes('unread=true'));await page.locator('.filters button').filter({hasText:'未読'}).click();await unreadLoaded;await page.locator('.article-list[aria-busy="false"]').waitFor();await page.getByRole('heading',{name:'未読',exact:true}).waitFor();await page.locator('.article-open').first().waitFor();
 const unreadBefore=(await(await page.request.get(origin+'/api/articles?unread=true&limit=1')).json()).total;const unreadFirst=await page.locator('.article-open').first().locator('strong').innerText();const unreadSecond=await page.locator('.article-open').nth(1).locator('strong').innerText();
 await page.locator('.article-open').first().click();await page.locator('.article-detail h1').filter({hasText:unreadFirst}).waitFor();await page.getByRole('button',{name:'次の記事'}).click();await page.locator('.article-detail h1').filter({hasText:unreadSecond}).waitFor();await page.getByRole('button',{name:'前の記事'}).click();await page.locator('.article-detail h1').filter({hasText:unreadFirst}).waitFor();assert.equal((await(await page.request.get(origin+'/api/articles?unread=true&limit=1')).json()).total,unreadBefore-2);await page.getByRole('button',{name:/一覧に戻る/}).click();
 const touchContext=await browser.newContext({viewport:{width:390,height:844},hasTouch:true,isMobile:true});const touch=await touchContext.newPage();await touch.goto(origin);await touch.locator('.article-open').first().tap();await touch.locator('.article-detail h1').waitFor();await touch.getByRole('button',{name:'設定',exact:true}).tap();await touch.getByRole('heading',{name:'フィードを集める'}).waitFor();assert.equal(await touch.locator('.article-detail').count(),0);await touchContext.close();
 for(const read of [false,true])for(const starred of [false,true]){const restore=items.items.filter(item=>item.is_read===read&&item.is_starred===starred).map(item=>item.id);if(restore.length)assert.equal((await page.request.post(origin+'/api/articles/bulk-state',{data:{ids:restore,read,starred,filters:{}}})).status(),200)}
 assert.deepEqual(errors,[]);await context.close();console.log('Floating detail passed: outside actions, delayed visits, switching, focus, retained list and responsive bounds.');
}finally{await browser.close()}
