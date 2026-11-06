import assert from 'node:assert/strict'
import { createRequire } from 'node:module'
import { fileURLToPath } from 'node:url'
const require = createRequire(new URL('../frontend/package.json', import.meta.url))
const { chromium } = require('@playwright/test')
const { createServer } = await import(fileURLToPath(new URL('../frontend/node_modules/vite/dist/node/index.js', import.meta.url)))
const react = (await import(fileURLToPath(new URL('../frontend/node_modules/@vitejs/plugin-react/dist/index.js', import.meta.url)))).default
const root = fileURLToPath(new URL('../frontend', import.meta.url))
const entry = `import React from 'react'; import {createRoot} from 'react-dom/client'; import Manager from '/src/SubscriptionManager.tsx';
const feeds=Array.from({length:101},(_,i)=>({id:i+1,title:'Feed '+(i+1),url:'https://example.test/'+(i+1),folder_id:1,paused:i===1,poll_interval:null,last_checked:i===0?'2026-01-01 12:00:00':null,last_error:i===0?'synthetic error':null}));
createRoot(document.getElementById('root')).render(React.createElement(Manager,{feeds,folders:[{id:1,name:'Research'}],onChanged:()=>{window.changed=(window.changed||0)+1},onEdit:f=>{window.edited=f.id}}));`
const server = await createServer({ root, configFile: false, server: { port: 0 }, plugins: [react({ fastRefresh: false }), { name:'smoke', resolveId(id) { if (id === 'virtual:smoke') return '\0virtual:smoke' }, load(id) { if (id === '\0virtual:smoke') return entry }, configureServer(s) { s.middlewares.use((req,res,next)=> { if(req.url==='/smoke.html'){res.setHeader('Content-Type','text/html');res.end('<div id="root"></div><script type="module">import RefreshRuntime from "/@react-refresh"; RefreshRuntime.injectIntoGlobalHook(window); window.$RefreshReg$=()=>{}; window.$RefreshSig$=()=>type=>type; window.__vite_plugin_react_preamble_installed__=true;</script><script type="module" src="/@id/virtual:smoke"></script>')} else next() }) } }] })
await server.listen()
let browser
try {
  browser = await chromium.launch({headless:true})
  const page = await browser.newPage({viewport:{width:390,height:844}})
  const errors=[]; page.on('pageerror', e=>errors.push(e.message))
  const calls=[]
  await page.route('**/api/**', async route => {
    const request=route.request(), path=new URL(request.url()).pathname
    if(path==='/api/session') return route.fulfill({json:{csrf_token:'a'.repeat(64)}})
    calls.push({path,method:request.method(),body:request.postData(),csrf:request.headers()['x-feeder-csrf']})
    if(path==='/api/feeds/1') await new Promise(resolve=>setTimeout(resolve,200))
    return route.fulfill({status:path==='/api/feeds/2'?409:200,body:'{}'})
  })
  await page.goto(server.resolvedUrls.local[0]+'smoke.html')
  const search=page.getByRole('searchbox')
  await search.fill('synthetic error')
  assert.equal(await page.locator('li').count(),0)
  await search.fill('Research')
  assert.equal(await page.locator('li').count(),101)
  await search.fill('Feed 1')
  assert.match(await page.locator('li').first().innerText(),/2026-01-01 12:00:00/)
  assert.match(await page.locator('li').first().innerText(),/synthetic error/)
  await page.locator('li').first().getByRole('button',{name:'編集'}).click()
  assert.equal(await page.evaluate(()=>window.edited),1)
  await search.fill('')
  assert.equal(await page.locator('li').nth(1).getByRole('button',{name:'今すぐ更新'}).isDisabled(),true)
  for(let i=0;i<100;i++) await page.getByRole('checkbox',{name:`Feed ${i+1}を選択`,exact:true}).check()
  assert.equal(await page.getByRole('checkbox',{name:'Feed 101を選択'}).isDisabled(),true)
  await page.getByRole('button',{name:'選択解除'}).click()
  await page.getByRole('checkbox',{name:'Feed 1を選択',exact:true}).check()
  await page.getByRole('checkbox',{name:'Feed 2を選択',exact:true}).check()
  await page.getByRole('button',{name:'選択を一時停止'}).click()
  assert.equal(await page.getByRole('button',{name:'選択を再開'}).isDisabled(),true)
  await page.getByRole('status').getByText('1件成功、1件失敗', {exact:false}).waitFor()
  assert.match(await page.getByRole('status').innerText(),/Feed 2（HTTP 409）/)
  assert.equal(await page.evaluate(()=>window.changed),1)
  assert.deepEqual(calls.map(c=>c.path),['/api/feeds/1','/api/feeds/2'])
  assert.ok(calls.every(c=>c.method==='PATCH' && JSON.parse(c.body).paused && c.csrf==='a'.repeat(64)))
  await search.fill('Feed 1')
  await page.locator('li').first().getByRole('button',{name:'今すぐ更新'}).click()
  await page.waitForFunction(()=>window.changed===2)
  assert.equal(calls.at(-1).path,'/api/feeds/1/refresh')
  assert.equal(calls.at(-1).method,'POST')
  assert.equal(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth),true)
  assert.deepEqual(errors,[])
  console.log('Subscription manager synthetic browser smoke passed.')
} finally { if(browser) await browser.close(); await server.close() }
