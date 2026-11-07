// End-to-end acceptance on a disposable v10 database. Never target a live DB.
import assert from 'node:assert/strict'
import { createRequire } from 'node:module'
import { spawn, spawnSync } from 'node:child_process'
import { mkdtemp, rm } from 'node:fs/promises'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import { setTimeout as delay } from 'node:timers/promises'
import { browserSession } from './browser-session.mjs'

const require = createRequire(new URL('../frontend/package.json', import.meta.url))
const { chromium } = require('@playwright/test')
const fixturePython = process.env.FEEDER_SMOKE_PYTHON || 'python3'
const base = new URL(process.env.FEEDER_CATCHUP_BASE_URL || 'http://127.0.0.1:18764')
assert(base.protocol === 'http:' && base.hostname === '127.0.0.1' && base.port && base.pathname === '/' && !base.search && !base.hash,
  'FEEDER_CATCHUP_BASE_URL must be an isolated http://127.0.0.1:PORT/')

const dir = await mkdtemp(join(tmpdir(), 'feeder-reader-catchup-'))
const db = join(dir, 'synthetic.sqlite3')
let server, browser
try {
  server = spawn(process.env.FEEDER_SMOKE_BINARY || './backend/target/debug/feeder', [], {
    env: { PATH: process.env.PATH, HOME: dir, FEEDER_DB: db, FEEDER_BIND: `${base.hostname}:${base.port}`, FEEDER_DEMO: '0', FEEDER_ACCESS_MODE: 'local' },
    stdio: ['ignore', 'ignore', 'pipe'],
  })
  let stderr = ''
  server.stderr.on('data', chunk => { stderr += chunk })
  let exited = false
  server.on('exit', () => { exited = true })
  for (let i = 0; i < 150 && !stderr.includes('Feeder listening'); i++) {
    if (exited) throw new Error(`Synthetic backend failed to start: ${stderr}`)
    await delay(100)
  }
  assert(stderr.includes('Feeder listening'), `Synthetic backend did not start: ${stderr}`)

  // v10 migrations have run. Seed only this temp DB using the current module DDL.
  // Fingerprints and story keys mirror article_revisions::fingerprint and
  // story_groups::index_article; no production source or user database is read.
  const fixtureNow = (await (await fetch(`${base.origin}/api/catchup`)).json()).window_end
  const seed = spawnSync(fixturePython, ['-c', `
import hashlib, json, sqlite3, sys, time
from urllib.parse import urlsplit, urlunsplit, parse_qsl, urlencode
db=sys.argv[1]; c=sqlite3.connect(db); c.execute('PRAGMA foreign_keys=ON')
now=sys.argv[2]
published=c.execute("SELECT strftime('%Y-%m-%dT%H:%M:%fZ','now','-1 hour')").fetchone()[0]
old_visit=c.execute("SELECT strftime('%Y-%m-%dT%H:%M:%fZ','now','-2 hours')").fetchone()[0]
c.execute("INSERT INTO folders(id,name) VALUES(9301,'Synthetic catch-up folder')")
for n in range(1,16):
 feed_id=9400+n; host='source%02d.synthetic.invalid'%n
 c.execute('INSERT INTO feeds(id,url,title,folder_id,paused,last_checked) VALUES(?,?,?,?,0,?)',(feed_id,'https://'+host+'/feed','Synthetic source %02d'%n,9301,now))
 c.execute('INSERT INTO collection_health(feed_id,last_success,consecutive_failures) VALUES(?,?,0)',(feed_id,now))

def fingerprint(title,content,url):
 h=hashlib.sha256()
 for value in (title,content,url):
  b=value.encode(); h.update(len(b).to_bytes(8,'big')); h.update(b)
 return h.hexdigest()
def norm(value): return ''.join(ch.lower() for ch in value if ch.isalnum())
def canonical(raw):
 u=urlsplit(raw); q=[(k,v) for k,v in parse_qsl(u.query,keep_blank_values=True) if not k.lower().startswith('utm_') and k.lower() not in ('gclid','fbclid')]
 return urlunsplit((u.scheme.lower(),u.netloc.lower(),u.path or '/',urlencode(q),''))
def add(article_id,feed_no,title,content,url,pub=published,kind='normal',read=0,versions=None,visited=None):
 feed_id=9400+feed_no
 values=versions or [(title,content,url,pub)]
 title,content,url,pub=values[-1]
 c.execute('INSERT INTO articles(id,title,content,url,published,is_read,is_starred,feed_id,external_id) VALUES(?,?,?,?,?,?,0,?,?)',(article_id,title,content,url,pub,read,feed_id,'synthetic-%d'%article_id))
 current_fp=fingerprint(*values[-1][0:3])
 changed=now if len(values)>1 else None
 c.execute('INSERT INTO article_revision_state(article_id,current_revision,fingerprint,first_seen_at,ingestion_kind,changed_at,source_updated_at,history_complete) VALUES(?,?,?,?,?,?,NULL,0)',(article_id,len(values),current_fp,now,kind,changed))
 for revision,(t,b,u,p) in enumerate(values,1):
  fp=fingerprint(t,b,u)
  c.execute('INSERT INTO article_revisions(article_id,revision,fingerprint,title,content,url,published,source_updated_at,observed_at,baseline) VALUES(?,?,?,?,?,?,?,NULL,?,?)',(article_id,revision,fp,t,b,u,p,old_visit if revision < len(values) else now,1 if revision==1 else 0))
 c.execute('INSERT INTO article_story_index(article_id,canonical_url,body_fingerprint,title_key,published,source_key) VALUES(?,?,?,?,?,?)',(article_id,canonical(url),hashlib.sha256(norm(content).encode()).hexdigest() if len(norm(content))>=200 and len(norm(title))>=12 else None,norm(title),pub,urlsplit(canonical(url)).hostname))
 if visited: c.execute('INSERT INTO article_visits(article_id,visited_at) VALUES(?,?)',(article_id,visited))

# 30 independently eligible events, three on one host to exercise the 2/source cap.
for i in range(30):
 article_id=9500+i
 feed_no=1 if i<3 else ((i-3)//2)+2
 title='Synthetic candidate %02d'%i
 body='Synthetic body %02d '%i + ('distinct fixture content ' * 18)
 url='https://article%02d.synthetic.invalid/story/%d'%(feed_no,article_id)
 add(article_id,feed_no,title,body,url)
# Specific ranking fixtures: literal Japanese/English match, priority source, update event.
add(9540,4,'Synthetic robotics interest','Robotics literal match fixture body','https://article04.synthetic.invalid/interest')
add(9541,5,'Synthetic priority feed','Priority source fixture body','https://article05.synthetic.invalid/priority')
add(9542,6,'Synthetic updated event','Updated content old body','https://article06.synthetic.invalid/updated',versions=[('Synthetic updated event','Updated content old body','https://article06.synthetic.invalid/updated',published),('Synthetic updated event','Updated content NEW body','https://article06.synthetic.invalid/updated',published)],visited=old_visit)
# A canonical URL pair is represented once and exposes one related member.
add(9543,7,'Synthetic canonical representative','Canonical group first body','https://group.synthetic.invalid/story?utm_source=one')
add(9544,8,'Synthetic canonical related','Canonical group second body','https://group.synthetic.invalid/story?utm_source=two')
add(9545,9,'Synthetic muted catch-up candidate','Mute marker body','https://article09.synthetic.invalid/muted')
# These must not become new events: migration/backfill, invalid/future time, or read.
add(9546,10,'Synthetic initial backfill','Initial context','https://article10.synthetic.invalid/initial',kind='initial')
add(9547,11,'Synthetic recovery backfill','Recovery context','https://article11.synthetic.invalid/recovery',kind='recovery')
add(9548,12,'Synthetic invalid timestamp','Invalid date context','https://article12.synthetic.invalid/invalid',pub='not-a-date')
future=c.execute("SELECT strftime('%Y-%m-%dT%H:%M:%fZ','now','+1 day')").fetchone()[0]
add(9549,13,'Synthetic future timestamp','Future date context','https://article13.synthetic.invalid/future',pub=future)
add(9550,13,'Synthetic already read','Read context','https://article13.synthetic.invalid/read',read=1)
c.commit(); c.close()
`, db, fixtureNow], { encoding: 'utf8' })
  assert.equal(seed.status, 0, `Synthetic fixture seed failed: ${seed.stderr}`)
  const seedCheck = spawnSync(fixturePython, ['-c', `
import sqlite3,sys
c=sqlite3.connect(sys.argv[1]); c.execute('PRAGMA foreign_keys=ON')
assert not c.execute('PRAGMA foreign_key_check').fetchall()
assert c.execute("SELECT count(*) FROM articles a JOIN article_revision_state s ON s.article_id=a.id JOIN article_revisions r ON r.article_id=s.article_id AND r.revision=s.current_revision WHERE a.title<>r.title OR a.content<>r.content OR a.url<>r.url OR s.fingerprint<>r.fingerprint").fetchone()[0] == 0
assert c.execute('SELECT count(*) FROM articles a LEFT JOIN article_story_index i ON i.article_id=a.id WHERE i.article_id IS NULL').fetchone()[0] == 0
`, db], { encoding: 'utf8' })
  assert.equal(seedCheck.status, 0, `Synthetic fixture integrity failed: ${seedCheck.stderr}`)

  browser = await chromium.launch({ headless: true })
  const origin = base.origin
  const errors = []
  const failedRequests = []
  const context = await browser.newContext({ viewport: { width: 1440, height: 900 } })
  const page = await context.newPage()
  page.on('pageerror', error => errors.push(error.message))
  const visitRequests = []
  page.on('request', request => { if (/\/api\/articles\/\d+\/visit$/.test(new URL(request.url()).pathname)) visitRequests.push(request.url()) })
  page.on('requestfailed', request => failedRequests.push(`${request.method()} ${request.url()}`))
  await context.route('**/*', route => {
    try { return new URL(route.request().url()).origin === origin ? route.continue() : route.abort() }
    catch { return route.abort() }
  })
  await browserSession(page, origin)
  const getResponse = path => page.request.get(`${origin}${path}`)
  const get = async path => {
    const response = await getResponse(path)
    assert.equal(response.status(), 200, `GET ${path}: ${await response.text()}`)
    return response.json()
  }
  const send = async (path, method, data, status = 200) => {
    const response = await page.request.fetch(`${origin}${path}`, { method, data })
    assert.equal(response.status(), status, `${method} ${path}: ${await response.text()}`)
    return status === 204 ? null : response.json()
  }

  assert.equal((await getResponse('/api/session')).status(), 200)
  const mute = await send('/api/mute-rules', 'POST', { name: 'Synthetic catch-up mute', term: 'Mute marker', field: 'body', feed_id: null, enabled: true }, 201)
  assert(mute.id)
  const beforePrefs = await get('/api/catchup/interests')
  const prefs = await send('/api/catchup/interests', 'PUT', {
    revision: beforePrefs.revision,
    terms: ['robotics', '合成関心'],
    priority_feed_ids: [9405],
  })
  assert.equal(prefs.revision, beforePrefs.revision + 1)
  assert.deepEqual(prefs.terms, ['robotics', '合成関心'])
  assert.deepEqual(prefs.priority_feed_ids, [9405])
  const conflictedPrefs = await page.request.fetch(`${origin}/api/catchup/interests`, {
    method: 'PUT', data: { ...beforePrefs, terms: ['stale'] },
  })
  assert.equal(conflictedPrefs.status(), 409, 'stale interest revision is rejected')
  const invalidPrefs = await page.request.fetch(`${origin}/api/catchup/interests`, {
    method: 'PUT', data: { ...prefs, terms: ['x'.repeat(81)] },
  })
  assert.equal(invalidPrefs.status(), 400, 'oversized interest term is rejected')
  assert.deepEqual(await get('/api/catchup/interests'), prefs, 'invalid settings leave shared interests unchanged')

  const articleIds = [9500, 9501, 9502, 9540, 9541, 9542, 9543, 9544, 9545, 9546, 9547, 9548, 9549, 9550]
  const stateBefore = new Map()
  for (const id of articleIds) {
    const article = await get(`/api/articles/${id}`)
    stateBefore.set(id, [article.is_read, article.is_starred, article.is_saved_for_later, article.last_visited])
  }
  const visitsBefore = spawnSync(fixturePython, ['-c', "import sqlite3,sys; c=sqlite3.connect(sys.argv[1]); print(c.execute('SELECT count(*) FROM article_visits').fetchone()[0])", db], { encoding: 'utf8' })
  assert.equal(visitsBefore.status, 0)

  const catchup = await get('/api/catchup?hours=24&limit=10')
  const requiredResponseFields = ['generated_at','window_start','window_end','limit','policy_version','scanned','eligible_count','truncated','collection_incomplete','items']
  for (const key of requiredResponseFields) assert(Object.hasOwn(catchup, key), `catch-up response contains ${key}`)
  assert.equal(catchup.limit, 10)
  assert(catchup.items.length > 0, 'synthetic normal-ingest fixtures produce catch-up candidates')
  assert(catchup.items.length <= 10)
  assert(catchup.scanned <= 5000, 'evaluated candidate count is capped at 5,000')
  assert(catchup.items.every(item => item.article && Number.isInteger(item.revision) && item.revision > 0 && item.fingerprint && item.event_key))
  assert(!catchup.items.some(item => [9545,9546,9547,9548,9549,9550].includes(item.article.id)), 'muted, backfilled, invalid/future-date and read fixtures are excluded')
  assert(catchup.items.some(item => item.article.id === 9540 && item.reasons.some(reason => reason.code === 'interest_term' && reason.value === 'robotics')))
  assert(catchup.items.some(item => item.article.id === 9541 && item.reasons.some(reason => reason.code === 'priority_feed' && reason.value === 9405)))
  const updateInitial = catchup.items.find(item => item.article.id === 9542)
  assert(updateInitial && updateInitial.kind === 'updated' && updateInitial.reasons.some(reason => reason.code === 'updated_since_visit'))
  const groupCandidate = catchup.items.find(item => item.article.id === 9543 || item.article.id === 9544)
  assert(groupCandidate && groupCandidate.related_count === 1, 'canonical duplicate is represented once with related count')
  const sourceFeeds = (await get('/api/library')).feeds
  const sourceHost = item => new URL(sourceFeeds.find(feed => feed.id === item.article.feed_id).url).hostname.toLowerCase()
  const sourceCounts = new Map()
  for (const item of catchup.items) {
    const host = sourceHost(item)
    sourceCounts.set(host, (sourceCounts.get(host) || 0) + 1)
  }
  assert([...sourceCounts.values()].every(count => count <= 2), 'source host cap is two representatives')
  const cappedFeed = (await get('/api/catchup?hours=168&limit=20&feed_id=9401')).items.map(item => item.article.id)
  assert.deepEqual(cappedFeed, [9502, 9501], 'source cap keeps the two highest ranked deterministic IDs for the three-item host')
  const rankTuple = item => [
    Number(item.reasons.some(reason => reason.code === 'interest_term')),
    Number(item.reasons.some(reason => reason.code === 'priority_feed')),
    Number(item.kind === 'updated'),
    Date.parse(item.kind === 'updated' ? item.article.changed_at : item.article.published),
    item.article.id,
  ]
  for (let i = 1; i < catchup.items.length; i++) {
    const before = rankTuple(catchup.items[i - 1]); const after = rankTuple(catchup.items[i])
    const cmp = before[0] - after[0] || before[1] - after[1] || before[2] - after[2] || before[3] - after[3] || before[4] - after[4]
    assert(cmp >= 0, 'selection order follows interest, priority feed, update, recent event, descending article ID')
  }
  const repeated = await get('/api/catchup?hours=24&limit=10')
  assert.deepEqual(repeated.items.map(item => item.event_key), catchup.items.map(item => item.event_key), 'selection is deterministic')
  const week = await get('/api/catchup?hours=168&limit=20')
  assert.equal(week.limit, 20)
  assert(week.items.length > 10 && week.items.length <= 20, '20-item queue is populated from synthetic fixtures')
  assert(week.scanned <= 5000)
  for (const host of new Set(week.items.map(item => sourceHost(item)))) {
    assert(week.items.filter(item => sourceHost(item) === host).length <= 2)
  }
  for (const path of ['/api/catchup?hours=25&limit=10','/api/catchup?hours=24&limit=11','/api/catchup?hours=24&limit=10&feed_id=9401&folder_id=9301']) {
    assert.equal((await getResponse(path)).status(), 400, `invalid query is rejected: ${path}`)
  }
  assert((await get('/api/catchup?hours=24&limit=10&feed_id=9401')).items.every(item => item.article.feed_id === 9401), 'feed scope is applied')
  assert((await get('/api/catchup?hours=24&limit=10&folder_id=9301')).items.length > 0, 'folder scope returns its candidates')
  for (const id of articleIds) {
    const article = await get(`/api/articles/${id}`)
    assert.deepEqual([article.is_read,article.is_starred,article.is_saved_for_later,article.last_visited], stateBefore.get(id), 'GET recommendation does not mutate reader state')
  }
  const visitsAfterGet = spawnSync(fixturePython, ['-c', "import sqlite3,sys; c=sqlite3.connect(sys.argv[1]); print(c.execute('SELECT count(*) FROM article_visits').fetchone()[0])", db], { encoding: 'utf8' })
  assert.equal(Number(visitsAfterGet.stdout.trim()), Number(visitsBefore.stdout.trim()), 'GET does not create visits')

  // Ack, idempotent undo, and update reappearance are tested against an exact version.
  const target = catchup.items.find(item => item.article.id === 9542) || catchup.items[0]
  const ackBody = { article_id: target.article.id, revision: target.revision, fingerprint: target.fingerprint }
  const staleAck = await page.request.fetch(`${origin}/api/catchup/acknowledgements`, { method: 'PUT', data: { ...ackBody, revision: target.revision + 1, action: 'confirmed' } })
  assert.equal(staleAck.status(), 409)
  assert((await get('/api/catchup?hours=24&limit=20')).items.some(item => item.event_key === target.event_key), 'stale acknowledgement changes nothing')
  const beforeAckArticle = await get(`/api/articles/${target.article.id}`)
  const beforeAckLater = await get('/api/read-later')
  const ack = await send('/api/catchup/acknowledgements', 'PUT', { ...ackBody, action: 'confirmed' })
  assert.deepEqual({article_id:ack.article_id,event_key:ack.event_key,revision:ack.revision,fingerprint:ack.fingerprint,action:ack.action}, {article_id:target.article.id,event_key:target.event_key,revision:target.revision,fingerprint:target.fingerprint,action:'confirmed'})
  assert(!(await get('/api/catchup?hours=24&limit=20')).items.some(item => item.event_key === target.event_key), 'acknowledged revision is excluded')
  await send('/api/catchup/acknowledgements', 'DELETE', ackBody, 204)
  await send('/api/catchup/acknowledgements', 'DELETE', ackBody, 204)
  assert((await get('/api/catchup?hours=24&limit=20')).items.some(item => item.event_key === target.event_key), 'idempotent undo restores the event')
  const afterAckArticle = await get(`/api/articles/${target.article.id}`)
  assert.deepEqual([afterAckArticle.is_read,afterAckArticle.is_starred,afterAckArticle.is_saved_for_later,afterAckArticle.last_visited], [beforeAckArticle.is_read,beforeAckArticle.is_starred,beforeAckArticle.is_saved_for_later,beforeAckArticle.last_visited])
  assert.deepEqual(await get('/api/read-later'), beforeAckLater)

  // Acknowledge the exact old event, then use a schema-aware synthetic fixture
  // update to represent a later normal collection commit for the same article.
  await send('/api/catchup/acknowledgements', 'PUT', { ...ackBody, action: 'skipped' })
  const observedAt = (await get('/api/catchup')).window_end
  const advance = spawnSync(fixturePython, ['-c', `
import hashlib,sqlite3,sys
c=sqlite3.connect(sys.argv[1]); article_id=int(sys.argv[2])
title='Synthetic updated event'; content='Updated content second NEW revision'; url='https://article06.synthetic.invalid/updated'
h=hashlib.sha256()
for value in (title,content,url):
 b=value.encode(); h.update(len(b).to_bytes(8,'big')); h.update(b)
fp=h.hexdigest(); now=sys.argv[3]
revision=c.execute('SELECT current_revision FROM article_revision_state WHERE article_id=?',(article_id,)).fetchone()[0]+1
published=c.execute('SELECT published FROM articles WHERE id=?',(article_id,)).fetchone()[0]
c.execute('UPDATE articles SET title=?,content=?,url=? WHERE id=?',(title,content,url,article_id))
c.execute('INSERT INTO article_revisions(article_id,revision,fingerprint,title,content,url,published,source_updated_at,observed_at,baseline) VALUES(?,?,?,?,?,?,?,NULL,?,0)',(article_id,revision,fp,title,content,url,published,now))
c.execute('UPDATE article_revision_state SET current_revision=?,fingerprint=?,changed_at=? WHERE article_id=?',(revision,fp,now,article_id))
c.execute('UPDATE article_story_index SET canonical_url=?,body_fingerprint=?,title_key=?,published=? WHERE article_id=?',(url,hashlib.sha256(''.join(x.lower() for x in content if x.isalnum()).encode()).hexdigest(),''.join(x.lower() for x in title if x.isalnum()),published,article_id))
c.commit()
`, db, String(target.article.id), observedAt], { encoding: 'utf8' })
  assert.equal(advance.status, 0, `synthetic revision advance failed: ${advance.stderr}`)
  const newerQueue = await get('/api/catchup?hours=24&limit=20')
  const newerEvent = newerQueue.items.find(item => item.article.id === target.article.id)
  assert(newerEvent, 'new substantive revision reappears after the prior revision was acknowledged')
  assert.equal(newerEvent.revision, target.revision + 1)
  assert.equal(newerEvent.kind, 'updated')
  const staleAfterChange = await page.request.fetch(`${origin}/api/catchup/acknowledgements`, { method: 'PUT', data: { ...ackBody, action: 'confirmed' } })
  assert.equal(staleAfterChange.status(), 409, 'acknowledging the prior revision cannot acknowledge the new revision')
  assert((await get('/api/catchup?hours=24&limit=20')).items.some(item => item.event_key === newerEvent.event_key))

  const revisionData = await get(`/api/articles/${target.article.id}/revisions?limit=10`)
  assert.deepEqual(revisionData.items.map(item => item.revision), [newerEvent.revision, target.revision, 1])
  assert.equal(revisionData.items[0].baseline, false)
  assert.equal(revisionData.items[1].baseline, false)
  assert.equal(revisionData.items[2].baseline, true)
  assert.equal(revisionData.history_complete, false)
  assert.equal(revisionData.history_start, 1)
  assert.equal(revisionData.items[0].title, 'Synthetic updated event')
  assert.equal((await getResponse('/api/articles/9542/revisions?limit=21')).status(), 400)
  const related = await get('/api/articles/9543/related?limit=20')
  assert.equal(related.article_id, 9543)
  assert.equal(related.total, 1)
  assert.equal(related.articles[0].article.id, 9544)
  assert.equal(related.articles[0].reason, 'canonical_url')
  const excluded = await send('/api/articles/9544/related-exclusion', 'PUT', { excluded: true })
  assert.equal(excluded.excluded, true)
  assert.equal((await get('/api/articles/9544/related')).excluded, true)
  assert((await get('/api/catchup?hours=24&limit=20')).items.some(item => item.article.id === 9543), 'excluded related article returns as an independent candidate')
  const restored = await send('/api/articles/9544/related-exclusion', 'PUT', { excluded: false })
  assert.equal(restored.excluded, false)
  assert((await get('/api/catchup?hours=24&limit=20')).items.some(item => item.article.id === 9544 && item.related_count === 1), 'restoring grouping returns the representative')

  // UI acceptance on the integrated Reader. Direct API assertions above remain
  // authoritative for state preservation; UI actions are checked independently.
  await page.goto(`${origin}/?view=all&mode=catchup`)
  const modeButton = page.getByRole('button', { name: 'キャッチアップ', exact: true })
  await modeButton.waitFor()
  await page.getByRole('heading', { name: 'キャッチアップ', exact: true }).waitFor()
  const queue = () => page.locator('.catchup-open')
  await queue().first().waitFor()
  assert(await page.locator('.catchup-panel').isVisible())
  await page.screenshot({ path: '/tmp/feeder-catchup-desktop.png', fullPage: true })
  const limitSelect = page.locator('.catchup-controls').getByLabel('件数')
  await limitSelect.selectOption('20')
  await page.waitForFunction(() => document.querySelectorAll('.catchup-open').length > 10)
  assert((await queue().count()) <= 20)
  const periodSelect = page.locator('.catchup-controls').getByLabel('期間')
  await periodSelect.selectOption('168')
  await page.waitForFunction(() => document.querySelector('.catchup-coverage')?.textContent?.includes('最大20件'))
  const uiTarget = page.locator('.catchup-open').first()
  const uiTargetId = Number(await uiTarget.getAttribute('data-article-id'))
  const visitsBeforeOpen = visitRequests.length
  await uiTarget.focus()
  await page.keyboard.press('Enter')
  await page.locator('.article-detail').waitFor()
  await page.waitForURL(url => url.searchParams.get('article') === String(uiTargetId))
  assert.equal(await page.locator('.article-detail > .detail-history > summary').textContent(), '記事の更新履歴')
  await page.keyboard.press('Escape')
  await page.waitForURL(url => !url.searchParams.has('article'))
  await page.waitForFunction(() => document.activeElement?.matches('.catchup-open'))
  assert.equal(visitRequests.length, visitsBeforeOpen + 1, 'opening from the catch-up queue records one normal visit')
  await page.goBack()
  await page.waitForURL(url => url.searchParams.has('article'))
  await page.goForward()
  await page.waitForURL(url => !url.searchParams.has('article'))
  assert.equal(visitRequests.length, visitsBeforeOpen + 1, 'back/forward restores reader state without another visit')

  const revisionItem = page.locator('.catchup-open[data-article-id="9542"]')
  await revisionItem.waitFor()
  await revisionItem.click()
  await page.locator('.article-detail').waitFor()
  const detailSummaries = page.locator('.article-detail details > summary')
  await detailSummaries.filter({ hasText: /改訂|版履歴|履歴/ }).click()
  await page.locator('.revision-entry > summary').filter({ hasText: '版1 · 最初の記録' }).click()
  await page.locator('.revision-content').getByText('Updated content old body', { exact: true }).waitFor()
  await page.locator('.revision-entry .revision-content').getByText('Updated content second NEW revision', { exact: true }).waitFor()
  await page.getByRole('button', { name: /一覧に戻る/ }).click()
  await page.locator('.article-detail').waitFor({ state: 'detached' })
  const relatedItem = page.locator('.catchup-open[data-article-id="9544"]')
  await relatedItem.waitFor()
  await relatedItem.click()
  await page.locator('.article-detail').waitFor()
  await page.locator('.article-detail details > summary').filter({ hasText: '関連記事' }).click()
  await page.locator('.article-detail').getByText('Synthetic canonical representative').waitFor()
  await page.getByRole('button', { name: /一覧に戻る/ }).click()
  await page.locator('.article-detail').waitFor({ state: 'detached' })

  // A stale write is visible and leaves the finite list intact.
  const countBeforeStale = await queue().count()
  await page.route('**/api/catchup/acknowledgements', route => route.fulfill({ status: 409 }))
  await page.locator('.catchup-actions').first().getByRole('button', { name: '現在版を確認' }).click()
  await page.getByRole('alert').filter({ hasText: '記事の版が変わりました' }).waitFor()
  assert.equal(await queue().count(), countBeforeStale)
  await page.unroute('**/api/catchup/acknowledgements')
  const ackUi = page.locator('.catchup-actions').first().getByRole('button', { name: '現在版を確認' })
  await ackUi.click()
  await page.getByRole('region', { name: '今回の確認と見送り' }).waitFor()
  const undoUi = page.getByRole('region', { name: '今回の確認と見送り' }).getByRole('button', { name: /取り消す/ }).first()
  await undoUi.focus()
  await undoUi.press('Enter')
  await page.getByRole('button', { name: '再読み込み' }).waitFor()
  await page.waitForFunction(() => document.activeElement?.textContent?.includes('再読み込み'))

  // Consecutive preference saves must use the revision returned by the server.
  await page.locator('.catchup-settings > summary').click()
  const termsInput = page.getByLabel('関心のある語句')
  for (const terms of ['robotics', 'robotics\n合成関心']) {
    await termsInput.fill(terms)
    await page.getByRole('button', { name: '共有設定を保存', exact: true }).click()
    await page.getByText('共有の関心設定を保存しました。', { exact: true }).waitFor()
    assert.deepEqual((await get('/api/catchup/interests')).terms, terms.split('\n'))
  }
  await page.locator('.catchup-settings > summary').click()
  await page.route('**/api/catchup?**', route => route.fulfill({ status: 503 }))
  await page.getByRole('button', { name: '再読み込み', exact: true }).click()
  await page.getByRole('alert').filter({ hasText: '通信に失敗しました' }).waitFor()
  assert(await queue().first().isDisabled(), 'stale results cannot be acted on after a failed refresh')
  await page.unroute('**/api/catchup?**')
  await page.getByRole('button', { name: '再読み込み', exact: true }).click()
  await page.waitForFunction(() => document.querySelector('.catchup-open')?.disabled === false)

  // Keyboard-visible queue and detail interactions at desktop; then 390px touch
  // hit target, light/dark overflow and focus/reflow checks.
  for (const theme of ['light','dark']) {
    await page.getByLabel('テーマ').selectOption(theme)
    await page.waitForFunction(value => document.documentElement.dataset.theme === value, theme)
    for (const width of [1440,390]) {
      await page.setViewportSize({ width, height: 844 })
      assert(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth), `${width}px ${theme} has no horizontal overflow`)
    }
  }
  await page.setViewportSize({ width: 390, height: 844 })
  await context.close()
  const touchContext = await browser.newContext({ viewport: { width: 390, height: 844 }, isMobile: true, hasTouch: true })
  const touchPage = await touchContext.newPage()
  touchPage.on('pageerror', error => errors.push(error.message))
  await touchContext.route('**/*', route => {
    try { return new URL(route.request().url()).origin === origin ? route.continue() : route.abort() }
    catch { return route.abort() }
  })
  await browserSession(touchPage, origin)
  await touchPage.goto(`${origin}/?view=all&mode=catchup`)
  const touchItem = touchPage.locator('.catchup-open').first()
  await touchItem.waitFor()
  await touchItem.tap()
  await touchPage.locator('.article-detail').waitFor()
  await touchPage.getByRole('button', { name: /一覧に戻る/ }).tap()
  await touchPage.waitForFunction(() => !document.querySelector('.article-detail'))
  assert(await touchPage.evaluate(() => document.documentElement.scrollWidth <= innerWidth), '390px touch layout has no horizontal overflow')
  await touchPage.evaluate(() => window.scrollTo(0, 0))
  await touchPage.screenshot({ path: '/tmp/feeder-catchup-mobile.png', fullPage: false })
  await touchContext.close()

  assert.deepEqual(errors, [], 'no uncaught browser exceptions')
  assert.deepEqual(failedRequests, [], 'no failed same-origin browser requests')
  console.log('Reader catch-up acceptance passed: API, state, revisions, grouping, desktop/mobile UI.')
} finally {
  await browser?.close()
  if (server && server.exitCode === null) {
    server.kill('SIGTERM')
    await Promise.race([new Promise(resolve => server.once('exit', resolve)), delay(3000)])
  }
  await rm(dir, { recursive: true, force: true })
}
