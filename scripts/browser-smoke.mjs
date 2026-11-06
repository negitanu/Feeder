import {browserSession} from './browser-session.mjs';
import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
const require = createRequire(new URL('../frontend/package.json', import.meta.url));
const { chromium } = require('@playwright/test');
const browser = await chromium.launch({ headless: true });
try {
  const page = await browser.newPage({ viewport: { width: 1280, height: 800 } });
  const errors = [];
  page.on('pageerror', e => errors.push(e.message));
  const origin=process.env.FEEDER_URL || 'http://127.0.0.1:8080';
  await browserSession(page,origin);
  await page.goto(origin);
  await page.locator('.article-card').first().waitFor();
  if (process.env.FEEDER_SCREENSHOT) await page.screenshot({ path: process.env.FEEDER_SCREENSHOT, fullPage: false });
  const firstTitle = await page.locator('.article-card strong').first().innerText();
  await page.locator('.article-open').first().click();
  await page.getByRole('heading', { name: firstTitle, exact: true }).waitFor();
  if (await page.getByRole('button', { name: 'スターを解除', exact: true }).first().isVisible()) {
    await page.getByRole('button', { name: 'スターを解除', exact: true }).first().click();
  }
  await page.getByRole('button', { name: 'スターを付ける', exact: true }).first().click();
  await page.getByRole('button', { name: 'スターを解除', exact: true }).first().waitFor();
  await page.getByRole('button', { name: '未読にする', exact: true }).click();
  await page.getByRole('button', { name: '既読にする', exact: true }).click();
  await page.getByRole('button', { name: /一覧に戻る/ }).click();
  await page.locator('.article-card.read').first().waitFor();
  await page.locator('.load-sentinel').scrollIntoViewIfNeeded();
  await page.waitForFunction(() => document.querySelectorAll('.article-card').length === 36);
  await page.getByRole('combobox', { name: 'テーマ' }).selectOption('dark');
  assert.equal(await page.locator('html').getAttribute('data-theme'), 'dark');
  await page.reload();
  assert.equal(await page.locator('html').getAttribute('data-theme'), 'dark');
  await page.setViewportSize({ width: 390, height: 844 });
  await page.getByRole('textbox', { name: '記事を検索' }).fill('見つからない検索語');
  await page.getByRole('textbox', { name: '記事を検索' }).press('Enter');
  await page.getByText('記事がありません', { exact: true }).waitFor();
  assert.equal(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth), true);
  assert.deepEqual(errors, []);
  console.log('Browser smoke passed: detail/read/star, progressive loading, theme persistence, narrow search, no overflow/errors.');
} finally { await browser.close(); }
