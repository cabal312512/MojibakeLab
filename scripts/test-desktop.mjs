// Real release-app checks, launched by test-desktop.ps1.
import { chromium, expect as baseExpect } from '@playwright/test';
import { readFile, writeFile, mkdir, rm } from 'node:fs/promises';
import path from 'node:path';
const root = path.resolve(import.meta.dirname, '..');
const expect = baseExpect.configure({ timeout: 30000 });
const out = path.join(root, 'test-results'),
  shots = path.join(root, 'docs/screenshots');
await mkdir(out, { recursive: true });
await mkdir(shots, { recursive: true });
const browser = await chromium.connectOverCDP('http://127.0.0.1:9223');
const context = browser.contexts()[0];
const page = context.pages().find((p) => p.url().includes('tauri.localhost')) ?? context.pages()[0];
if (!page) throw new Error('No desktop webview found');
page.setDefaultTimeout(30000);
await page.waitForSelector('.app');
expect(await page.evaluate(() => localStorage.getItem('mojibake.locale'))).toBe('en');
await expect(page.locator('html')).toHaveAttribute('lang', 'en');
await page.getByRole('button', { name: 'MIT', exact: true }).click();
await expect(page.locator('.license-dialog')).toBeVisible();
await expect(page.locator('.license-dialog pre')).toContainText('Copyright (c) 2026 cabal312512');
await expect(page.locator('.license-dialog pre')).toContainText('THIRD-PARTY LICENSES');
await page.keyboard.press('Escape');
await expect(page.locator('.license-dialog')).not.toBeVisible();
const issues = [],
  external = [];
page.on('pageerror', (error) => issues.push(error.message));
page.on('request', (request) => {
  if (/^https?:/.test(request.url()) && !new URL(request.url()).hostname.endsWith('localhost'))
    external.push(request.url());
});
await page.evaluate(() => localStorage.setItem('mojibake.theme', 'dark'));
await page.reload();
await page.waitForSelector('.app');
expect(await page.evaluate(() => getComputedStyle(document.documentElement).colorScheme)).toBe(
  'light',
);
expect(await page.evaluate(() => localStorage.getItem('mojibake.theme'))).toBeNull();
await page.evaluate(() => {
  const original = window.fetch.bind(window);
  window.__DESKTOP_TEST__ = {};
  window.fetch = async (input, init) => {
    const url = new URL(typeof input === 'string' ? input : input.url);
    const command =
      url.hostname === 'ipc.localhost' ? decodeURIComponent(url.pathname.slice(1)) : '';
    // Bypass only the OS picker UI; Rust still performs all reads and safe writes.
    if (command === 'plugin:dialog|open' || command === 'plugin:dialog|save') {
      const value = command.endsWith('|open')
        ? window.__DESKTOP_TEST__.openPath
        : window.__DESKTOP_TEST__.savePath;
      return new Response(JSON.stringify(value), {
        headers: { 'Content-Type': 'application/json', 'Tauri-Response': 'ok' },
      });
    }
    const result = await original(input, init);
    if (command.startsWith('analyze_') && result.headers.get('Tauri-Response') === 'ok')
      window.__DESKTOP_TEST__.analysis = await result.clone().json();
    return result;
  };
});
const invoke = (command, args) =>
  page.evaluate(
    ([command, args]) => window.__TAURI_INTERNALS__.invoke(command, args),
    [command, args],
  );
const setPaths = (values) =>
  page.evaluate((values) => Object.assign(window.__DESKTOP_TEST__, values), values);
const newCase = async () => {
  await page.locator('.brand').click();
  await expect(page.locator('.input-workspace')).toBeVisible();
};
const sample = async (id) => {
  await page.locator('.sample-select select').selectOption(id);
  await expect(page.locator('.result-workspace')).toBeVisible();
};
const locale = page.locator('.language-select select');
await locale.selectOption('en');
await newCase();
await page.locator('textarea').fill('ä¸­æ–‡');
await page.screenshot({ path: path.join(shots, 'input.png') });
await page.locator('textarea').press('Control+Enter');
await expect(page.locator('.recovered-text')).toHaveText('中文');
await expect(page.locator('.transformation-strip')).toContainText('Windows-1252');
await page.locator('#tab-compare').click();
await expect(page.locator('.compare-grid')).toBeVisible();
await page.locator('#tab-graph').click();
await expect(page.locator('.graph-active-path')).toContainText('Windows-1252');
await page.locator('#tab-evidence').click();
await expect(page.getByRole('tabpanel')).toContainText('Original bytes unavailable');
await page.locator('#tab-why').click();
await expect(page.locator('.score-breakdown')).toBeVisible();
await page.locator('#tab-preview').click();
const first = await page.locator('.recovered-text').textContent();
const candidates = page.locator('.candidate-card');
if ((await candidates.count()) > 1) {
  await candidates.nth(1).click();
  await expect(page.locator('.recovered-text')).not.toHaveText(first);
  await candidates.nth(0).click();
}
await page.getByRole('button', { name: 'Copy text', exact: true }).click();
await expect(page.locator('.toast')).toContainText('Text copied');
await sample('double');
await expect(page.locator('.recovered-text')).toContainText('中文测试文件');
await sample('loss');
await expect(page.locator('.case-warnings')).toContainText(/loss/i);
await sample('clean');
await expect(page.locator('.app-footer')).toContainText('No obvious encoding corruption');
const source = path.join(root, 'tests/fixtures/utf8_as_windows1252.txt');
const original = await readFile(source),
  destination = path.join(out, 'desktop.recovered.txt');
await rm(destination, { force: true });
await setPaths({ openPath: source, savePath: destination });
await page.getByRole('button', { name: 'Open file', exact: true }).click();
await expect(page.locator('.document-heading h1')).toHaveText('utf8_as_windows1252.txt');
await expect(page.locator('.recovered-text')).toContainText('中文');
await page.getByRole('button', { name: 'Save as…', exact: true }).click();
await expect(page.locator('.toast')).toContainText('Recovered file saved');
const analysis = await page.evaluate(() => window.__DESKTOP_TEST__.analysis);
expect(await readFile(destination, 'utf8')).toBe(analysis.candidates[0].fullText);
for (const existing of [source, destination]) {
  try {
    await invoke('save_candidate', {
      caseId: analysis.caseId,
      candidateId: analysis.candidates[0].id,
      path: existing,
    });
    throw new Error('Existing file was overwritten');
  } catch (error) {
    if (!String(error).includes('output_exists')) throw error;
  }
}
expect((await readFile(source)).equals(original)).toBe(true);
await page.locator('#tab-hex').click();
await expect(page.locator('.hex-table tbody tr').first()).toBeVisible();
await page.locator('#tab-preview').click();
await locale.selectOption('zh-CN');
await expect(page.locator('html')).toHaveAttribute('lang', 'zh-CN');
await expect(page.locator('.toast')).not.toBeVisible();
await page.screenshot({ path: path.join(shots, 'workbench.png') });
await page.locator('#tab-graph').click();
await page.screenshot({ path: path.join(shots, 'path.png') });
await locale.selectOption('ja');
await expect(page.locator('html')).toHaveAttribute('lang', 'ja');
await page.screenshot({ path: path.join(shots, 'japanese.png') });
await locale.selectOption('zh-CN');
await page.locator('#tab-preview').click();
const geometry = await page.evaluate(() => ({
  width: innerWidth,
  height: innerHeight,
  scrollWidth: document.documentElement.scrollWidth,
  scrollHeight: document.documentElement.scrollHeight,
}));
expect(geometry.width).toBeLessThanOrEqual(840);
expect(geometry.height).toBeLessThanOrEqual(660);
expect(geometry.scrollWidth).toBeLessThanOrEqual(geometry.width);
expect(geometry.scrollHeight).toBeLessThanOrEqual(geometry.height);
if (issues.length) throw new Error('Frontend errors: ' + issues.join('; '));
if (external.length) throw new Error('Unexpected network requests: ' + external.join('; '));
await writeFile(
  path.join(out, 'desktop-smoke.json'),
  JSON.stringify(
    {
      passed: true,
      checks: [
        'English on a fresh profile',
        'embedded offline license viewer',
        'light-only profile migration',
        'native paste IPC',
        'candidate selection',
        'typed graph',
        'compare',
        'evidence',
        'score',
        'clipboard',
        'double recovery',
        'loss warning',
        'clean preservation',
        'file picker bridge',
        'hex',
        'safe full save',
        'overwrite refusal',
        'three locales',
        'compact dimensions',
        'no page errors',
        'no external page requests',
      ],
      geometry,
      issues,
      external,
    },
    null,
    2,
  ),
);
console.log('Desktop smoke checks passed.');
await browser.close();
