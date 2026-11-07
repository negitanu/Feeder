import {browserSession} from './browser-session.mjs';
import assert from 'node:assert/strict';
import {mkdir} from 'node:fs/promises';
import {createRequire} from 'node:module';
const require=createRequire(new URL('../frontend/package.json',import.meta.url));
const {chromium}=require('@playwright/test');
const browser=await chromium.launch({headless:true});
const origin=process.env.FEEDER_URL||'http://127.0.0.1:8080';
try {
 const context=await browser.newContext({viewport:{width:1280,height:900}});
 const page=await context.newPage(),errors=[];page.on('pageerror',e=>errors.push(e.message));
 await browserSession(page,origin);
 const library=async()=>await(await page.request.get(origin+'/api/library')).json();
 const names=['配置検証A','配置検証B','配置検証C'];
 for(const name of names)assert.equal((await page.request.post(origin+'/api/folders',{data:{name}})).status(),200);
 const xml=`<opml version="2.0"><body><outline text="${names[0]}"><outline text="並びA" xmlUrl="https://example.org/sidebar-a.xml"/><outline text="並びB" xmlUrl="https://example.org/sidebar-b.xml"/></outline><outline text="並びC" xmlUrl="https://example.org/sidebar-c.xml"/></body></opml>`;
 assert.equal((await page.request.post(origin+'/api/opml/import',{data:{xml}})).status(),200);
 const start=await library(), folders=names.map(n=>start.folders.find(f=>f.name===n)), feedA=start.feeds.find(f=>f.title==='並びA'),feedB=start.feeds.find(f=>f.title==='並びB'),feedC=start.feeds.find(f=>f.title==='並びC');
 const handle=(kind,id)=>page.locator(`[data-order-handle="${kind}-${id}"]`);
 async function keyboard(kind,id,key){const response=page.waitForResponse(r=>r.url().endsWith('/api/library/order'));await handle(kind,id).press(key);assert.equal((await response).status(),204);await page.getByRole('status').filter({hasText:'順序を保存しました。'}).waitFor();await page.waitForFunction(key=>document.activeElement?.getAttribute('data-order-handle')===key,`${kind}-${id}`);}
 await page.goto(origin);await handle('folder',folders[2].id).waitFor();
 await page.locator('.article-select').first().check();
 await keyboard('folder',folders[2].id,'Home');assert.equal(await page.locator('.article-select:checked').count(),1);assert.equal((await library()).folders[0].id,folders[2].id);
 await keyboard('feed',feedB.id,'Home');assert.equal((await library()).feeds.filter(f=>f.folder_id===folders[0].id)[0].id,feedB.id);
 await page.reload();await handle('feed',feedB.id).waitFor();
 assert.equal(await page.locator(`[data-order-target="folder"][data-order-id="${folders[0].id}"]`).locator('..').locator('[data-order-target="feed"]').first().getAttribute('data-order-id'),String(feedB.id));
 // An independent browser gets the DB order without any copied browser storage.
 const other=await browser.newPage();await other.goto(origin);await other.locator(`[data-order-target="folder"][data-order-id="${folders[2].id}"]`).waitFor();assert.equal(await other.locator('[data-order-target="folder"]').first().getAttribute('data-order-id'),String(folders[2].id));await other.close();
 await page.getByRole('button',{name:`${names[1]}を折りたたむ`,exact:true}).click();
 async function drag(source,target,cancel=false){
  const a=await source.boundingBox(), b=await target.boundingBox();assert.ok(a&&b);
  await page.mouse.move(a.x+a.width/2,a.y+a.height/2);await page.mouse.down();await page.mouse.move(b.x+b.width/2,b.y+b.height/2,{steps:10});
  await page.locator('[data-drop]').first().waitFor();
  if(cancel){await page.evaluate(()=>{document.querySelector('[data-order-handle]:focus')?.dispatchEvent(new PointerEvent('pointercancel',{bubbles:true,pointerId:1,isPrimary:true}));});await page.mouse.up();}
  else{const response=page.waitForResponse(r=>r.url().endsWith('/api/library/order'));await page.mouse.up();assert.equal((await response).status(),204);await page.getByRole('status').filter({hasText:'順序を保存しました。'}).waitFor();}
 }
 await page.getByRole('button',{name:`${names[0]}`,exact:true}).click();await page.getByRole('heading',{name:names[0],exact:true}).waitFor();
 const folderRefetch=page.waitForResponse(r=>r.url().includes('/api/articles?')&&new URL(r.url()).searchParams.get('folder_id')===String(folders[0].id));
 await drag(handle('feed',feedA.id),page.locator(`[data-order-target="folder"][data-order-id="${folders[1].id}"]`));
 assert.equal((await folderRefetch).status(),200);await page.getByRole('heading',{name:names[0],exact:true}).waitFor();
 assert.equal((await library()).feeds.find(f=>f.id===feedA.id).folder_id,folders[1].id);
 await page.getByRole('button',{name:/すべて/}).click();
 await page.getByRole('button',{name:`${names[1]}を展開`,exact:true}).waitFor();
 const beforeCancel=await library();await drag(handle('feed',feedC.id),page.locator(`[data-order-target="folder"][data-order-id="${folders[0].id}"]`),true);assert.deepEqual(await library(),beforeCancel);
 // Menus offer folder movement without dragging.
 await page.getByRole('button',{name:'並びCの操作',exact:true}).click();await page.getByRole('button',{name:'フォルダーを移動',exact:true}).click();await page.getByRole('dialog').getByRole('combobox').selectOption(String(folders[0].id));await page.getByRole('button',{name:'保存',exact:true}).click();await page.getByRole('dialog').waitFor({state:'hidden'});assert.equal((await library()).feeds.filter(f=>f.folder_id===folders[0].id).at(-1).id,feedC.id);
 // Repair a feed address in place; duplicate rejection preserves the dialog and data.
 const articlesBefore=await(await page.request.get(origin+'/api/articles?limit=100')).json();
 await page.getByRole('button',{name:'並びCの操作',exact:true}).click();await page.getByRole('button',{name:'編集',exact:true}).click();const editor=page.getByRole('dialog');
 await editor.getByLabel('フィードURL').fill(feedB.url);const duplicate=page.waitForResponse(r=>r.url().endsWith(`/api/feeds/${feedC.id}`)&&r.request().method()==='PATCH');await editor.getByRole('button',{name:'保存'}).click();assert.equal((await duplicate).status(),409);await editor.getByRole('alert').filter({hasText:'登録済み'}).waitFor();assert.equal((await library()).feeds.find(f=>f.id===feedC.id).url,feedC.url);
 await editor.getByLabel('フィードURL').fill('https://example.org/sidebar-c-new.xml#fragment');await editor.getByLabel('個別の収集間隔（秒）').fill('300');const changed=page.waitForResponse(r=>r.url().endsWith(`/api/feeds/${feedC.id}`)&&r.request().method()==='PATCH');await editor.getByRole('button',{name:'保存'}).press('Enter');assert.equal((await changed).status(),200);await editor.waitFor({state:'hidden'});assert.equal((await library()).feeds.find(f=>f.id===feedC.id).url,'https://example.org/sidebar-c-new.xml');assert.equal((await library()).feeds.find(f=>f.id===feedC.id).poll_interval,300);assert.deepEqual(await(await page.request.get(origin+'/api/articles?limit=100')).json(),articlesBefore);
 // Simulate a competing session edit just before submission; UI must recover from409.
 await page.route('**/api/library/order',async route=>{
  const {expected}=route.request().postDataJSON();const order=structuredClone(expected);order.folders.reverse();order.feeds=[...order.folders,null].flatMap(id=>order.feeds.filter(f=>f.folder_id===id));
  assert.equal((await page.request.post(origin+'/api/library/order',{data:{expected,order}})).status(),204);
  await route.continue();
 },{times:1});
 const conflict=page.waitForResponse(r=>r.url().endsWith('/api/library/order')&&r.status()===409);
 await handle('folder',folders[0].id).press('Home');await conflict;await page.getByRole('status').filter({hasText:'別の操作で構成が変わりました。'}).waitFor();
 const authoritative=await library();await page.waitForFunction(id=>document.querySelector('[data-order-target="folder"]')?.getAttribute('data-order-id')===String(id),authoritative.folders[0].id);
 // Desktop pointer/keyboard sizing, reload restoration, bounds and mobile.
 const separator=page.getByRole('separator',{name:'サイドバーの幅'});await separator.press('Home');assert.equal(await separator.getAttribute('aria-valuenow'),'200');await separator.press('ArrowRight');assert.equal(await separator.getAttribute('aria-valuenow'),'210');
 const rect=await separator.boundingBox();await page.mouse.move(rect.x+3,rect.y+100);await page.mouse.down();await page.mouse.move(rect.x+83,rect.y+100);await page.mouse.up();const width=await separator.getAttribute('aria-valuenow');assert.equal(width,'290');await page.reload();assert.equal(await separator.getAttribute('aria-valuenow'),width);
 if(process.env.FEEDER_SCREENSHOT_DIR){await mkdir(process.env.FEEDER_SCREENSHOT_DIR,{recursive:true});await page.screenshot({path:process.env.FEEDER_SCREENSHOT_DIR+'/sidebar-desktop.png',fullPage:false});}
 await page.setViewportSize({width:780,height:900});assert.ok(Number(await separator.getAttribute('aria-valuenow'))<=414);assert.equal(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth),true);
 await page.setViewportSize({width:390,height:844});assert.equal(await separator.isVisible(),false);assert.equal(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth),true);
 await page.getByLabel('テーマ',{exact:true}).selectOption('dark');assert.equal(await page.evaluate(()=>document.documentElement.dataset.theme),'dark');
 // True touch pointer dispatch through Chromium, with normal touch action off handles.
 const mobile=await browser.newContext({viewport:{width:390,height:844},hasTouch:true,isMobile:true});const touch=await mobile.newPage();await touch.goto(origin);await touch.locator('[data-order-target="folder"]').first().waitFor();
 await touch.getByRole('button',{name:'並びCの操作',exact:true}).click();await touch.getByRole('button',{name:'編集',exact:true}).click();assert.equal(await touch.getByRole('dialog').getByLabel('フィードURL').inputValue(),'https://example.org/sidebar-c-new.xml');assert.equal(await touch.getByRole('dialog').getByLabel('個別の収集間隔（秒）').inputValue(),'300');assert.equal(await touch.evaluate(()=>document.documentElement.scrollWidth<=innerWidth),true);await touch.getByRole('dialog').getByRole('button',{name:'キャンセル'}).click();
 const touchFolders=await library(),moving=touchFolders.folders[0],dest=touchFolders.folders[1];
 const th=touch.locator(`[data-order-handle="folder-${moving.id}"]`),td=touch.locator(`[data-order-target="folder"][data-order-id="${dest.id}"]`);await th.scrollIntoViewIfNeeded();const ta=await th.boundingBox(),tb=await td.boundingBox();
 const cdp=await mobile.newCDPSession(touch);const touchResponse=touch.waitForResponse(r=>r.url().endsWith('/api/library/order'));
 await cdp.send('Input.dispatchTouchEvent',{type:'touchStart',touchPoints:[{x:ta.x+ta.width/2,y:ta.y+ta.height/2}]});await cdp.send('Input.dispatchTouchEvent',{type:'touchMove',touchPoints:[{x:tb.x+tb.width/2,y:tb.y+tb.height-2}]});await cdp.send('Input.dispatchTouchEvent',{type:'touchEnd',touchPoints:[]});assert.equal((await touchResponse).status(),204);
 if(process.env.FEEDER_SCREENSHOT_DIR)await touch.screenshot({path:process.env.FEEDER_SCREENSHOT_DIR+'/sidebar-mobile.png',fullPage:false});
 assert.equal(await touch.locator('.source-button').first().evaluate(el=>getComputedStyle(el).touchAction),'auto');await mobile.close();
 const denied=await browser.newContext();await denied.addInitScript(()=>{Object.defineProperty(window,'localStorage',{get(){throw new Error('storage denied')}})});const deniedPage=await denied.newPage();await deniedPage.goto(origin);await deniedPage.getByRole('separator').press('ArrowRight');await deniedPage.locator('.article-card').first().waitFor();assert.equal(await deniedPage.getByRole('separator').getAttribute('aria-valuenow'),'260');await denied.close();
 const invalidContext=await browser.newContext({viewport:{width:1280,height:800}});await invalidContext.addInitScript(()=>localStorage.setItem('feeder-sidebar-width','999999'));const invalidPage=await invalidContext.newPage();await invalidPage.goto(origin);await invalidPage.getByRole('separator').waitFor();assert.equal(await invalidPage.getByRole('separator').getAttribute('aria-valuenow'),'250');await invalidContext.close();
 const globalUnread=page.locator('.filters button').filter({hasText:'未読'});await globalUnread.click();await globalUnread.locator('.count').waitFor();const unreadCount=await globalUnread.locator('.count').textContent();await page.getByRole('button',{name:`${names[0]}`,exact:true}).click();await page.getByRole('heading',{name:names[0],exact:true}).waitFor();assert.equal(await globalUnread.getAttribute('aria-current'),null);assert.equal(await globalUnread.locator('.count').textContent(),unreadCount);await globalUnread.click();assert.equal(await globalUnread.getAttribute('aria-current'),'page');
 assert.deepEqual(errors,[]);console.log('Sidebar browser passed: keyboard/mouse/touch reorder, collapsed destination, cancellation, menu movement, cross-session order, conflict recovery, resize restoration, mobile and denied storage.');
 await page.route('**/api/library',async route=>{const response=await route.fetch();const data=await response.json();data.folders=data.folders.map(f=>({...f,name:'非常に長いフォルダー名'.repeat(12)}));data.feeds=data.feeds.map(f=>({...f,title:'VeryLongUnbrokenFeedName'.repeat(12),paused:true}));await route.fulfill({response,json:data})});
 await page.evaluate(()=>{localStorage.setItem('feeder-sidebar-width','200');localStorage.setItem('feeder-collapsed-folders','[]')});await page.reload();await page.locator('.feed-row').first().waitFor();
 for(const theme of ['light','dark']){await page.getByRole('combobox',{name:'テーマ',exact:true}).selectOption(theme);assert.equal(await page.locator('.sidebar').evaluate(sidebar=>{const bounds=sidebar.getBoundingClientRect();return [...sidebar.querySelectorAll('.folder-row,.mini-menu')].every(el=>{const r=el.getBoundingClientRect();return r.left>=bounds.left&&r.right<=bounds.right+1})}),true);assert.equal(await page.locator('.feed-source span').first().evaluate(el=>el.scrollWidth>el.clientWidth),true)}
 await context.close();
}finally{await browser.close();}
