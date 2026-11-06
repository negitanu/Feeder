import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
if (!process.env.FEEDER_LIVE_FEED) throw new Error('Explicit FEEDER_LIVE_FEED public URL required; uses a dedicated test DB.');
const require=createRequire(new URL('../frontend/package.json',import.meta.url));
const {chromium}=require('@playwright/test');const browser=await chromium.launch();
try{
 const context=await browser.newContext();const page=await context.newPage();const origin=process.env.FEEDER_URL||'http://127.0.0.1:8080';
 await page.goto(origin);await page.getByRole('button',{name:'集める',exact:true}).click();
 const before=(await(await context.request.get(origin+'/api/library')).json()).feeds.length;
 await page.getByRole('textbox',{name:'サイトまたはフィード URL'}).fill(process.env.FEEDER_LIVE_FEED);
 await page.getByRole('button',{name:'フィードを探す',exact:true}).click();
 await page.getByRole('combobox',{name:'フィード候補'}).selectOption({index:1});
 const previewResponse=page.waitForResponse(r=>r.url().endsWith('/api/preview'));
 await page.getByRole('button',{name:'内容をプレビュー',exact:true}).click();const preview=await(await previewResponse).json();assert.ok(preview.entries.length<=5);assert.ok(preview.token);
 assert.equal((await(await context.request.get(origin+'/api/library')).json()).feeds.length,before);
 const other=await browser.newContext();const denied=await other.request.post(origin+'/api/preview/confirm',{data:{token:preview.token}});assert.ok(denied.status()>=400);await other.close();
 const confirmed=page.waitForResponse(r=>r.url().endsWith('/api/preview/confirm'));
 await page.getByRole('button',{name:'このフィードを登録',exact:true}).click();assert.equal((await confirmed).status(),200);
 assert.equal((await(await context.request.get(origin+'/api/library')).json()).feeds.length,before+1);
 assert.ok((await context.request.post(origin+'/api/preview/confirm',{data:{token:preview.token}})).status()>=400);
 console.log('Live discovery passed: preview did not register, foreign session rejected, explicit confirm registered, replay rejected.');
}finally{await browser.close();}
