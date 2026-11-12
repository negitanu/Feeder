const { createRequire } = require('node:module');
const { chromium } = createRequire(require('node:path').resolve('frontend/package.json'))('@playwright/test');
const origin = process.env.FEEDER_BASE_URL || 'http://127.0.0.1:4178';
const base = new URL(origin);
if (base.protocol !== 'http:' || base.hostname !== '127.0.0.1' || base.port === '8080') throw new Error('Use a local test server, not the production instance');
const assert = require('node:assert/strict');
(async()=>{
 const browser=await chromium.launch({headless:true,...(process.env.FEEDER_TEST_CHROME ? {executablePath:process.env.FEEDER_TEST_CHROME} : {})});
 try {
  for(const standalone of [true,false]){
   const page=await browser.newPage({viewport:{width:390,height:844},isMobile:true,hasTouch:true});
   if(standalone) await page.addInitScript(()=>Object.defineProperty(navigator,'standalone',{value:true}));
   let loads=0; page.on('load',()=>loads++);
   await page.route('**/api/**',async route=>{
    const url=new URL(route.request().url()), p=url.pathname;
    let body={};
    if(p==='/api/session') body={csrf_token:'a'.repeat(64)};
    else if(p==='/api/reader/settings') body={display_timezone:'UTC',default_view:'all'};
    else if(p==='/api/library') body={folders:Array.from({length:50},(_,i)=>({id:i+1,name:`Folder ${i+1}`})),feeds:[]};
    else if(p==='/api/library/unread') body={total:0,folders:[]};
    else if(p==='/api/articles') body={items:Array.from({length:24},(_,i)=>({id:i+1,title:`Article ${i+1}`,content:'Example content',url:`https://example.com/${i}`,published:'2026-11-11T12:00:00Z',is_read:false,is_starred:false,is_saved_for_later:false})),total:24,has_more:false,next_cursor:null};
    await route.fulfill({status:200,contentType:'application/json',body:JSON.stringify(body)});
   });
   await page.goto(origin + '/'); await page.locator('.article-card').first().waitFor();
   const cdp=await page.context().newCDPSession(page);
   const start=async(x,y)=>cdp.send('Input.dispatchTouchEvent',{type:'touchStart',touchPoints:[{x,y}]});
   const move=async(x,y)=>cdp.send('Input.dispatchTouchEvent',{type:'touchMove',touchPoints:[{x,y}]});
   const end=async()=>cdp.send('Input.dispatchTouchEvent',{type:'touchEnd',touchPoints:[]});
   await start(200,160); await move(201,200); await end(); await page.waitForTimeout(200); assert.equal(loads,1,'short pull');
   await start(180,160); await move(270,170); await end(); await page.waitForTimeout(200); assert.equal(loads,1,'horizontal gesture');
   if(standalone){
    await page.getByRole('button',{name:'メニューを開く'}).click();
    await start(180,160); await move(180,270); await end(); await page.waitForTimeout(200); assert.equal(loads,1,'drawer gesture');
    await page.locator('.mobile-sidebar-backdrop').click({position:{x:370,y:400}});
    await page.evaluate(()=>scrollTo(0,600)); await start(200,200); await move(200,320); await end(); await page.waitForTimeout(200); assert.equal(loads,1,'scrolled page');
    await page.evaluate(()=>scrollTo(0,0));
   }
   await start(200,160); await move(200,195); await move(200,265);
   if(standalone) await page.getByText('離して更新',{exact:true}).waitFor();
   await end();
   if(standalone) { await page.waitForLoadState('load'); await page.waitForTimeout(300); assert.equal(loads,2,'refresh on release'); }
   else { await page.waitForTimeout(200); assert.equal(loads,1,'normal browser untouched'); }
   console.log(`${standalone?'standalone':'browser'}: gesture checks passed`); await page.close();
  }
 } finally {await browser.close()}
})().catch(e=>{console.error(e);process.exit(1)});
