import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
import { spawn, spawnSync } from 'node:child_process';
import { mkdtemp, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { setTimeout as delay } from 'node:timers/promises';
import { browserSession } from './browser-session.mjs';

const require = createRequire(new URL('../frontend/package.json', import.meta.url));
const { chromium } = require('@playwright/test');
const base = new URL(process.env.FEEDER_BASE_URL || 'http://127.0.0.1:18765');
assert(base.protocol === 'http:' && base.hostname === '127.0.0.1' && base.port && base.port !== '8080' && base.pathname === '/' && !base.search && !base.hash, 'FEEDER_BASE_URL must be an unused local test port other than 8080');
const origin = base.origin;
const dir = await mkdtemp(join(tmpdir(), 'feeder-mobile-layout-'));
const db = join(dir, 'fixture.sqlite3');
let server, browser;
const failures = [];
const check = async (label, fn) => {
  try { await fn(); console.log(`PASS ${label}`); }
  catch (error) { failures.push(`${label}: ${error.message}`); console.error(`FAIL ${label}: ${error.message}`); }
};

try {
  server = spawn(process.env.FEEDER_SMOKE_BINARY || './backend/target/debug/feeder', [], {
    env: { PATH: process.env.PATH, HOME: dir, FEEDER_DB: db, FEEDER_BIND: `${base.hostname}:${base.port}`, FEEDER_DEMO: '0', FEEDER_ACCESS_MODE: 'local' },
    stdio: ['ignore', 'ignore', 'pipe'],
  });
  let stderr = ''; let exited = false;
  server.stderr.on('data', chunk => { stderr += chunk });
  server.on('exit', () => { exited = true });
  for (let i = 0; i < 100 && !stderr.includes('Feeder listening'); i++) {
    if (exited) throw new Error(`Fixture backend failed: ${stderr}`);
    await delay(100);
  }
  assert(stderr.includes('Feeder listening'), `Fixture backend did not start: ${stderr}`);
  const seed = spawnSync('python3', ['-c', `
import sqlite3, sys
c = sqlite3.connect(sys.argv[1])
c.execute("INSERT INTO feeds(id,url,title,paused) VALUES(1,'https://feed.example.invalid/rss','合成フィードの長い名前',1)")
for i in range(1, 5):
 c.execute('INSERT INTO articles(id,title,content,url,published,feed_id,is_read,is_starred) VALUES(?,?,?,?,?,?,0,0)', (i, f'合成記事 {i} 長いタイトルの表示と折り返しを確認します', '合成記事の本文です。レイアウトと操作の検証に使用します。', f'https://article.example.invalid/{i}', '2026-10-24 08:19:50', 1))
c.commit()
`, db], { encoding: 'utf8' });
  assert.equal(seed.status, 0, `Fixture seed failed: ${seed.stderr}`);
  browser = await chromium.launch();
  const context = await browser.newContext({ viewport: { width: 390, height: 844 }, hasTouch: true, reducedMotion: 'reduce' });
  await context.route('**/*', route => new URL(route.request().url()).origin === origin ? route.continue() : route.abort());
  const page = await context.newPage();
  page.setDefaultTimeout(5000);
  const errors = [];
  page.on('pageerror', error => errors.push(error.message));
  await browserSession(page, origin);
  await page.goto(origin);
  await page.locator('.article-card').first().waitFor();
  await page.locator('.filters button').filter({ hasText: 'すべて' }).first().click();
  await page.waitForFunction(() => document.querySelectorAll('.article-card').length === 4);
  const layoutSelect = page.getByRole('combobox', { name: '記事の表示形式' });
  const themeSelect = page.getByRole('combobox', { name: 'テーマ', exact: true });
  for (const theme of ['light', 'dark']) {
    await themeSelect.selectOption(theme);
    for (const layout of ['standard', 'line', 'tiles']) {
      await layoutSelect.selectOption(layout);
      const label = `${theme}/${layout}`;
      const list = page.locator(`.article-list[data-layout="${layout}"]`);
      await check(`${label} geometry`, async () => {
        const geometry = await list.evaluate(el => {
          const cards = [...el.querySelectorAll('.article-card')];
          const rects = cards.map(card => { const r = card.getBoundingClientRect(); return { x: r.x, y: r.y }; });
          const parts = [...cards[0].querySelectorAll('.source, strong, time')].map(part => {
            const r = part.getBoundingClientRect(); const style = getComputedStyle(part);
            return { y: r.y, height: r.height, lineHeight: parseFloat(style.lineHeight), whiteSpace: style.whiteSpace, text: part.textContent };
          });
          return { width: innerWidth, overflow: document.documentElement.scrollWidth > innerWidth, rects, parts,
            excerpt: getComputedStyle(cards[0].querySelector('.excerpt')).display };
        });
        assert.equal(geometry.width, 390);
        assert.equal(geometry.overflow, false, 'horizontal page overflow');
        assert.equal(geometry.rects.length, 4);
        const sameRow = (a, b) => Math.abs(a.y - b.y) < 2;
        if (layout === 'tiles') {
          assert.ok(sameRow(geometry.rects[0], geometry.rects[1]), 'first two tiles must share a row');
          assert.ok(geometry.rects[1].x > geometry.rects[0].x, 'tiles must occupy two columns');
          assert.ok(!sameRow(geometry.rects[1], geometry.rects[2]), 'third tile must start the next row');
        } else {
          assert.ok(geometry.rects.slice(1).every((r, i) => r.y > geometry.rects[i].y && Math.abs(r.x - geometry.rects[i].x) < 2), 'cards must form one column');
        }
        assert.ok(geometry.parts.every(p => p.text.trim()), 'source, title and timestamp must be present');
        if (layout === 'line') {
          assert.ok(geometry.parts.every(p => p.whiteSpace === 'nowrap' && p.height <= p.lineHeight + 2), 'each field must occupy one clipped line');
          assert.ok(geometry.parts.every(p => Math.abs(p.y - geometry.parts[0].y) < 8), 'source, title and time must share one row');
          assert.equal(geometry.excerpt, 'none');
        } else if (layout === 'standard') {
          assert.notEqual(geometry.excerpt, 'none');
          assert.ok(geometry.parts[1].y > geometry.parts[0].y && geometry.parts[2].y > geometry.parts[1].y, 'standard fields must use separate rows');
        }
      });
      await page.mouse.move(0, 0);
      await check(`${label} theme and contrast`, async () => {
        assert.equal(await page.locator('html').getAttribute('data-theme'), theme);
        await page.waitForFunction(expected => {
          const rgb = getComputedStyle(document.querySelector('.article-card')).backgroundColor.match(/[\d.]+/g).slice(0, 3).map(Number);
          return expected === 'dark' ? Math.max(...rgb) < 120 : Math.min(...rgb) > 180;
        }, theme);
        const colors = await page.locator('.article-card').first().evaluate(el => {
          const parse = value => value.match(/[\d.]+/g).slice(0, 3).map(Number);
          const luminance = rgb => rgb.map(c => { c /= 255; return c <= 0.04045 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4; }).reduce((s, c, i) => s + c * [0.2126, 0.7152, 0.0722][i], 0);
          const bg = luminance(parse(getComputedStyle(el).backgroundColor));
          const fg = luminance(parse(getComputedStyle(el.querySelector('strong')).color));
          return { bg, ratio: (Math.max(bg, fg) + 0.05) / (Math.min(bg, fg) + 0.05) };
        });
        assert.ok(theme === 'dark' ? colors.bg < 0.2 : colors.bg > 0.5, 'card surface matches theme');
        assert.ok(colors.ratio >= 4.5, `title contrast ${colors.ratio.toFixed(2)} is below 4.5`);
      });
      await check(`${label} touch and keyboard actions`, async () => {
        const card = list.locator('.article-card').first();
        const select = card.locator('.article-select');
        await select.check(); assert.equal(await select.isChecked(), true); await select.uncheck();
        const star = card.locator('.card-star');
        const initialStar = await star.getAttribute('aria-pressed');
        await star.tap(); await page.waitForFunction(({ initial }) => document.querySelector('.article-card .card-star')?.getAttribute('aria-pressed') !== initial, { initial: initialStar });
        await star.focus(); await page.keyboard.press('Space');
        await page.waitForFunction(({ initial }) => document.querySelector('.article-card .card-star')?.getAttribute('aria-pressed') === initial, { initial: initialStar });
        const saved = card.locator('.read-later-button-compact');
        const initialSaved = await saved.getAttribute('aria-pressed');
        await saved.tap(); await page.waitForFunction(({ initial }) => document.querySelector('.article-card .read-later-button-compact')?.getAttribute('aria-pressed') !== initial, { initial: initialSaved });
        await saved.tap(); await page.waitForFunction(({ initial }) => document.querySelector('.article-card .read-later-button-compact')?.getAttribute('aria-pressed') === initial, { initial: initialSaved });
        await card.locator('.article-open').tap(); await page.locator('.article-detail h1').waitFor();
        await page.keyboard.press('Escape'); await page.locator('.article-detail').waitFor({ state: 'detached' });
      });
    }
  }
  await check('browser errors', async () => assert.deepEqual(errors, []));
  await context.close();
} finally {
  if (browser) await browser.close();
  if (server && server.exitCode === null) { server.kill('SIGTERM'); await new Promise(resolve => server.once('exit', resolve)); }
  await rm(dir, { recursive: true, force: true });
}
if (failures.length) {
  console.error(`\n${failures.length} check(s) failed:\n${failures.join('\n')}`);
  process.exitCode = 1;
} else console.log('Mobile layout browser smoke passed.');
