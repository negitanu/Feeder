import {browserSession} from './browser-session.mjs';
import assert from 'node:assert/strict';
import {createRequire} from 'node:module';
import {mkdir} from 'node:fs/promises';
const require=createRequire(new URL('../frontend/package.json',import.meta.url));
const {chromium}=require('@playwright/test');
const browser=await chromium.launch({headless:true});
const origin=process.env.FEEDER_URL||'http://127.0.0.1:8080';
try {
 const context=await browser.newContext({viewport:{width:1280,height:900},timezoneId:'America/Los_Angeles'}),page=await context.newPage(),errors=[];
 page.on('pageerror',e=>errors.push(e.message));
 await browserSession(page,origin);
 const get=async path=>await(await page.request.get(origin+path)).json();
 const patch=async data=>await page.request.patch(origin+'/api/reader/settings',{data});
 assert.equal((await patch({display_timezone:'UTC',default_view:'all'})).status(),200);
 await page.goto(origin);await page.locator('.article-card').first().waitFor();
 await page.locator('.load-sentinel').scrollIntoViewIfNeeded();await page.waitForFunction(()=>document.querySelectorAll('.article-card').length===36);
 await page.locator('.article-select').first().check();await page.locator('.article-select').last().check();
 const cardIds=await page.locator('.article-open').evaluateAll(els=>els.map(el=>el.dataset.articleId));
 for(const layout of ['line','tiles','standard']){await page.getByRole('combobox',{name:'記事の表示形式',exact:true}).selectOption(layout);assert.deepEqual(await page.locator('.article-open').evaluateAll(els=>els.map(el=>el.dataset.articleId)),cardIds);assert.equal(await page.locator('.article-select:checked').count(),2);}
 await page.getByRole('combobox',{name:'記事の表示形式',exact:true}).selectOption('tiles');
 const title=await page.locator('.article-card strong').last().innerText();await page.locator('.article-open').last().click();await page.getByRole('heading',{name:title,exact:true}).waitFor();await page.getByRole('button',{name:/一覧に戻る/}).click();assert.equal(await page.locator('.article-card').count(),36);assert.equal(await page.locator('.article-select:checked').count(),2);assert.equal(await page.locator('.article-list').getAttribute('data-layout'),'tiles');
 await page.reload();await page.locator('.article-card').first().waitFor();assert.equal(await page.getByRole('combobox',{name:'記事の表示形式',exact:true}).inputValue(),'tiles');
 await page.getByRole('button',{name:'集める',exact:true}).click();
 await page.getByRole('combobox',{name:'表示タイムゾーン',exact:true}).selectOption('Asia/Tokyo');await page.getByRole('combobox',{name:'初期表示',exact:true}).selectOption('unread');
 const saved=page.waitForResponse(r=>r.url().endsWith('/api/reader/settings')&&r.request().method()==='PATCH');await page.getByRole('button',{name:'表示設定を保存',exact:true}).click();assert.equal((await saved).status(),200);await page.getByRole('status').filter({hasText:'表示設定を保存しました。'}).waitFor();
 assert.deepEqual(await get('/api/reader/settings'),{display_timezone:'Asia/Tokyo',default_view:'unread'});
 const other=await browser.newPage({timezoneId:'America/Los_Angeles'});await other.goto(origin);await other.getByRole('heading',{name:'未読',exact:true}).waitFor();await other.locator('.timezone-label').filter({hasText:'日本時間'}).waitFor();await other.close();
 // Initial response must not override an explicit view clicked during settings loading.
 const slow=await context.newPage();let release;const gate=new Promise(resolve=>release=resolve);await slow.route('**/api/reader/settings',async route=>{await gate;await route.continue()});await slow.goto(origin);await slow.getByRole('button',{name:/スター付き/}).click();release();await slow.locator('.timezone-label').filter({hasText:'日本時間'}).waitFor();await slow.getByRole('heading',{name:'スター付き',exact:true}).waitFor();await slow.close();
 // Use synthetic query credentials; imports remain paused and do not fetch the URL.
 const canary='FEEDER_EXPORT_TEST_CANARY';const xml=`<opml><body><outline text="Bearer ${canary}" xmlUrl="https://example.org/settings-export.xml?token=${canary}"/></body></opml>`;
 assert.equal((await page.request.post(origin+'/api/opml/import',{data:{xml}})).status(),200);
 await page.reload();await page.getByRole('button',{name:'集める',exact:true}).click();await page.getByRole('combobox',{name:'設定の記事表示形式',exact:true}).selectOption('line');await page.getByRole('combobox',{name:'設定のテーマ',exact:true}).selectOption('dark');
 await page.getByRole('separator',{name:'サイドバーの幅'}).press('Home');await page.getByRole('separator',{name:'サイドバーの幅'}).press('ArrowRight');
 const libraryBefore=await get('/api/library'),articlesBefore=await get('/api/articles?limit=100'),historyBefore=await get('/api/articles?history=true&limit=100'),collectionBefore=await get('/api/collection/settings');
 const dl=page.waitForEvent('download'),exportResponse=page.waitForResponse(r=>r.url().endsWith('/api/configuration/export'));
 await page.getByRole('button',{name:'設定をエクスポート',exact:true}).click();const download=await dl,response=await exportResponse;assert.equal(response.status(),200);assert.match(response.headers()['cache-control'],/no-store/);assert.match(response.headers()['content-disposition'],/attachment/);assert.equal(download.suggestedFilename(),'feeder-settings.json');
 const chunks=[];for await(const chunk of await download.createReadStream())chunks.push(chunk);const bytes=Buffer.concat(chunks).toString('utf8'),data=JSON.parse(bytes);
 assert.equal(data.format,'feeder-settings');assert.equal(data.format_version,1);assert.equal(data.schema_version,8);assert.deepEqual(data.settings.reader,{display_timezone:'Asia/Tokyo',default_view:'unread'});assert.equal(data.settings.presentation.layout,'line');assert.equal(data.settings.presentation.appearance,'dark');assert.equal(data.settings.presentation.width,210);assert.ok(data.omissions.length>=2);assert.ok(Array.isArray(data.unsupported_sections));assert.ok(!bytes.includes(canary));assert.ok(!bytes.includes('これは Feeder の表示を確認'));assert.ok(!bytes.includes('last_error'));assert.deepEqual(await get('/api/library'),libraryBefore);assert.deepEqual(await get('/api/articles?limit=100'),articlesBefore);assert.deepEqual(await get('/api/articles?history=true&limit=100'),historyBefore);assert.deepEqual(await get('/api/collection/settings'),collectionBefore);
 assert.deepEqual(data.settings.folders.map(f=>f.id),libraryBefore.folders.map(f=>f.id));assert.deepEqual(data.settings.subscriptions.map(f=>f.id),libraryBefore.feeds.map(f=>f.id));assert.deepEqual(data.settings.subscriptions.map(f=>f.poll_interval),libraryBefore.feeds.map(f=>f.poll_interval));
 const invalid=await page.request.post(origin+'/api/configuration/export',{data:{presentation:{appearance:'dark',width:999,layout:'line',collapsed_folders:[]}}});assert.ok(invalid.status()>=400);assert.doesNotMatch(invalid.headers()['content-disposition']||'',/attachment/);
 assert.equal((await patch({display_timezone:'Europe/London'})).status(),400);assert.equal((await page.request.patch(origin+'/api/reader/settings',{headers:{Origin:'https://attacker.invalid'},data:{default_view:'all'}})).status(),403);
 // Displayed day must be independent from the OS/browser timezone.
 await page.getByRole('button',{name:'読む',exact:true}).click();await page.getByRole('button',{name:/すべて/}).click();
 await page.route('**/api/articles?*',async route=>{const r=await route.fetch();const list=await r.json();if(list.items.length)list.items[0].published='2026-10-31T23:30:00Z';await route.fulfill({response:r,json:list})});
 await page.getByRole('textbox',{name:'記事を検索'}).fill('サンプル');await page.getByRole('textbox',{name:'記事を検索'}).press('Enter');await page.waitForFunction(()=>document.querySelector('.article-card time')?.textContent==='2026/11/01');
 await page.setViewportSize({width:390,height:844});for(const layout of ['standard','line','tiles']){await page.getByRole('combobox',{name:'記事の表示形式',exact:true}).selectOption(layout);assert.equal(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth),true)}
 if(process.env.FEEDER_SCREENSHOT_DIR){await mkdir(process.env.FEEDER_SCREENSHOT_DIR,{recursive:true});await page.screenshot({path:process.env.FEEDER_SCREENSHOT_DIR+'/settings-tiles-mobile.png'});await page.getByRole('button',{name:'集める',exact:true}).click();await page.getByRole('heading',{name:'表示設定',exact:true}).scrollIntoViewIfNeeded();await page.screenshot({path:process.env.FEEDER_SCREENSHOT_DIR+'/settings-mobile.png'});}
 // Refresh shared timezone from another writer when this window regains focus.
 assert.equal((await patch({display_timezone:'UTC'})).status(),200);await page.evaluate(()=>window.dispatchEvent(new Event('focus')));await page.getByRole('button',{name:'読む',exact:true}).click();await page.waitForFunction(()=>document.querySelector('.timezone-label')?.textContent==='日付: UTC');
 assert.equal((await patch({display_timezone:'Asia/Tokyo'})).status(),200);await page.evaluate(()=>window.dispatchEvent(new Event('focus')));await page.waitForFunction(()=>document.querySelector('.timezone-label')?.textContent?.includes('日本時間'));
 // A setting changed by another session between focus events cannot silently change a bulk action's scope.
 const beforeConflict=await get('/api/articles?limit=100');
 assert.equal((await patch({display_timezone:'UTC'})).status(),200);
 let bulkRequests=0;page.on('request',r=>{if(r.url().endsWith('/api/articles/read-all')&&r.method()==='POST')bulkRequests++});
 const conflict=page.waitForResponse(r=>r.url().endsWith('/api/articles/read-all')&&r.request().method()==='POST');
 await page.getByRole('button',{name:'全て既読',exact:true}).click();assert.equal((await conflict).status(),409);
 await page.getByRole('status').filter({hasText:'操作を中止しました'}).waitFor();await page.waitForFunction(()=>document.querySelector('.timezone-label')?.textContent==='日付: UTC');
 assert.equal(bulkRequests,1);assert.deepEqual(await get('/api/articles?limit=100'),beforeConflict);
 await context.close();
 const retryContext=await browser.newContext();const retryPage=await retryContext.newPage();await retryPage.route('**/api/reader/settings',route=>route.fulfill({status:503,body:'unavailable'}),{times:1});await retryPage.goto(origin);await retryPage.getByRole('alert').waitFor();await retryPage.getByRole('button',{name:'再試行',exact:true}).click();await retryPage.locator('.article-card').first().waitFor();await retryContext.close();
 // Browser settings are taken from live state even if storage is inaccessible.
 const denied=await browser.newContext({viewport:{width:1280,height:900}});await denied.addInitScript(()=>Object.defineProperty(window,'localStorage',{get(){throw new Error('denied')}}));const d=await denied.newPage();await d.goto(origin);await d.getByRole('button',{name:'集める',exact:true}).click();await d.getByRole('combobox',{name:'設定の記事表示形式',exact:true}).selectOption('tiles');await d.getByRole('combobox',{name:'設定のテーマ',exact:true}).selectOption('dark');await d.getByRole('separator').press('End');const er=d.waitForResponse(r=>r.url().endsWith('/api/configuration/export'));await d.getByRole('button',{name:'設定をエクスポート',exact:true}).click();const body=await(await er).json();assert.equal(body.settings.presentation.layout,'tiles');assert.equal(body.settings.presentation.appearance,'dark');assert.equal(body.settings.presentation.width,480);await denied.close();
 const invalidContext=await browser.newContext();await invalidContext.addInitScript(()=>localStorage.setItem('feeder-layout','invalid'));const ip=await invalidContext.newPage();await ip.goto(origin);assert.equal(await ip.getByRole('combobox',{name:'記事の表示形式',exact:true}).inputValue(),'standard');await invalidContext.close();
 // No observer: failed continuation is retryable without losing cards, selection, or visits.
 const manualContext=await browser.newContext();await manualContext.addInitScript(()=>{delete window.IntersectionObserver});const manual=await manualContext.newPage();
 await manual.goto(origin);await manual.getByRole('button',{name:/すべて/}).click();await manual.waitForFunction(()=>document.querySelectorAll('.article-card').length===24);
 assert.equal(await manual.locator('.article-card').count(),24);await manual.locator('.article-select').first().check();
 const visitsBefore=await(await manual.request.get(origin+'/api/articles?history=true&limit=100')).json();
 let attempts=0;await manual.route('**/api/articles?*',async route=>{const cursor=new URL(route.request().url()).searchParams.get('cursor');if(cursor&&attempts++===0)return route.fulfill({status:503,body:'unavailable'});return route.continue()});
 await manual.getByRole('button',{name:'さらに読み込む'}).click();await manual.getByRole('button',{name:'続きの読込を再試行'}).waitFor();
 assert.equal(await manual.locator('.article-card').count(),24);assert.equal(await manual.locator('.article-select:checked').count(),1);assert.equal(attempts,1);
 await manual.getByRole('button',{name:'続きの読込を再試行'}).press('Enter');await manual.waitForFunction(()=>document.querySelectorAll('.article-card').length===36);
 assert.equal(await manual.locator('.article-select:checked').count(),1);assert.deepEqual(await(await manual.request.get(origin+'/api/articles?history=true&limit=100')).json(),visitsBefore);
 // A delayed failure for an old scope must not surface in the newly selected scope.
 await manual.reload();await manual.getByRole('button',{name:/すべて/}).click();await manual.waitForFunction(()=>document.querySelectorAll('.article-card').length===24);
 let releaseOld, arrivedOld;const oldArrived=new Promise(resolve=>arrivedOld=resolve),oldGate=new Promise(resolve=>releaseOld=resolve);
 await manual.unrouteAll({behavior:'wait'});await manual.route('**/api/articles?*',async route=>{if(new URL(route.request().url()).searchParams.has('cursor')){arrivedOld();await oldGate;return route.fulfill({status:503,body:'unavailable'})}return route.continue()});
 await manual.getByRole('button',{name:'さらに読み込む'}).click();await oldArrived;
 await manual.getByRole('textbox',{name:'記事を検索'}).fill('サンプル');await manual.getByRole('textbox',{name:'記事を検索'}).press('Enter');await manual.getByText('「サンプル」の検索結果').waitFor();
 releaseOld();await manual.unrouteAll({behavior:'wait'});assert.equal(await manual.getByRole('button',{name:'続きの読込を再試行'}).count(),0);
 await manual.reload();await manual.getByRole('button',{name:/すべて/}).click();await manual.waitForFunction(()=>document.querySelectorAll('.article-card').length===24);await manual.locator('.article-select').first().check();
 await browserSession(manual,origin);assert.equal((await manual.request.patch(origin+'/api/reader/settings',{data:{display_timezone:'Asia/Tokyo'}})).status(),200);
 await manual.getByRole('button',{name:'さらに読み込む'}).click();await manual.getByRole('button',{name:'最新の一覧を読み直す'}).waitFor();assert.equal(await manual.locator('.article-select:checked').count(),1);
 await manual.getByRole('button',{name:'最新の一覧を読み直す'}).click();await manual.waitForFunction(()=>document.querySelector('.timezone-label')?.textContent?.includes('日本時間'));await manual.locator('.article-card').first().waitFor();assert.equal(await manual.getByRole('button',{name:'最新の一覧を読み直す'}).count(),0);await manualContext.close();
 assert.deepEqual(errors,[]);console.log('Settings browser passed: layouts retain appended selection, shared defaults and timezone, late-response protection, safe read-only download, invalid inputs, mobile and storage denial.');
}finally{await browser.close();}
