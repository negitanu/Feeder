import {browserSession} from './browser-session.mjs';
import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
const require=createRequire(new URL('../frontend/package.json',import.meta.url));
const {chromium}=require('@playwright/test');
const browser=await chromium.launch({headless:true});
try {
 const page=await browser.newPage({viewport:{width:1280,height:800}});
 const origin=process.env.FEEDER_URL||'http://127.0.0.1:8080';
 await browserSession(page,origin);
 const errors=[];page.on('pageerror',e=>errors.push(e.message));
 await page.goto(origin);await page.getByRole('button',{name:'設定',exact:true}).click();
 const interval=page.getByRole('textbox',{name:'更新間隔（秒）'});await interval.fill('');assert.equal(await interval.inputValue(),'');assert.equal(await page.getByRole('button',{name:'設定を保存',exact:true}).isDisabled(),true);await interval.pressSequentially('120');assert.equal(await interval.inputValue(),'120');await interval.fill('59');assert.equal(await page.getByRole('button',{name:'設定を保存',exact:true}).isDisabled(),true);await interval.fill('86401');assert.equal(await page.getByRole('button',{name:'設定を保存',exact:true}).isDisabled(),true);await interval.fill('60');
 await page.getByRole('checkbox',{name:'自動収集を有効にする'}).check();
 const saved=page.waitForResponse(r=>r.url().endsWith('/api/collection/settings')&&r.request().method()==='PATCH');
 await page.getByRole('button',{name:'設定を保存',exact:true}).click();assert.equal((await saved).status(),200);
 let settings=await(await page.request.get(origin+'/api/collection/settings')).json();assert.equal(settings.interval_seconds,60);assert.equal(settings.enabled,true);
 await page.reload();await page.getByRole('button',{name:'設定',exact:true}).click();await page.getByRole('checkbox',{name:'自動収集を有効にする'}).waitFor();
 await page.waitForFunction(()=>document.querySelector('input[type=checkbox]')?.checked===true);
 // Keep the synthetic OPML URLs away from the worker; import preserves this explicit global choice.
 assert.equal((await page.request.patch(origin+'/api/collection/settings',{data:{enabled:false}})).status(),200);
 const before=(await(await page.request.get(origin+'/api/library')).json()).feeds.length;
 const xml='<?xml version="1.0"?><opml version="2.0"><body><outline text="調査"><outline text="Web"><outline text="A &amp; B" xmlUrl="https://example.org/feeder-onboarding.xml?a=1&amp;b=2"/><outline text="duplicate" xmlUrl="https://example.org/feeder-onboarding.xml?a=1&amp;b=2#fragment"/><outline text="unsafe" xmlUrl="http://127.0.0.1/private"/></outline></outline></body></opml>';
 const preview=page.waitForResponse(r=>r.url().endsWith('/api/opml/preview'));
 await page.getByLabel('OPML ファイル',{exact:true}).setInputFiles({name:'subscriptions.opml',mimeType:'application/xml',buffer:Buffer.from(xml)});
 const pr=await preview;assert.equal(pr.status(),200);const summary=await pr.json();assert.equal(summary.new,1);assert.equal(summary.duplicates,1);assert.equal(summary.invalid,1);
 assert.equal((await(await page.request.get(origin+'/api/library')).json()).feeds.length,before);
 const imported=page.waitForResponse(r=>r.url().endsWith('/api/opml/import'));
 await page.getByRole('button',{name:'確認して読み込む',exact:true}).click();assert.equal((await imported).status(),200);
 const library=await(await page.request.get(origin+'/api/library')).json();assert.equal(library.feeds.length,before+1);
 const added=library.feeds.find(f=>f.url.startsWith('https://example.org/feeder-onboarding.xml'));assert.equal(added.paused,false);
 assert.equal((await(await page.request.get(origin+'/api/collection/settings')).json()).enabled,false);
 assert.equal(library.folders.find(f=>f.id===added.folder_id).name,'調査/Web');
 const exported=await page.request.get(origin+'/api/opml/export');assert.equal(exported.status(),200);assert.match(await exported.text(),/A &amp; B/);
 const invalid=await page.request.post(origin+'/api/opml/preview',{data:{xml:'<!DOCTYPE opml [<!ENTITY x SYSTEM "file:///etc/passwd">]><opml><body/></opml>'}});assert.ok(invalid.status()>=400);
 await page.setViewportSize({width:390,height:844});assert.equal(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth),true);assert.deepEqual(errors,[]);
 console.log('Onboarding browser passed: persisted worker settings, explicit OPML preview/import, duplicate and unsafe input, active imports, export, mobile.');
} finally {await browser.close();}
