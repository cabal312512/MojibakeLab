// Browser-only layout QA using actual Rust-generated evidence, never a JS engine.
import { chromium, expect } from '@playwright/test';
import { mkdir, writeFile } from 'node:fs/promises';
import { execFileSync } from 'node:child_process';
import path from 'node:path';
const root = path.resolve(import.meta.dirname, '..');
const output = path.join(root, 'test-results', 'compact-visual');
await mkdir(output, { recursive: true });
await mkdir(path.join(root, '__preview__'), { recursive: true });
await writeFile(
  path.join(root, '__preview__/case.json'),
  execFileSync(path.join(root, 'target/debug/examples/sample_json.exe'), ['cp1252']),
);
const context = await chromium.launchPersistentContext(path.join(output, 'edge-profile'), {
  executablePath: 'C:\\Program Files (x86)\\Microsoft\\Edge\\Application\\msedge.exe',
  headless: true,
  viewport: { width: 820, height: 640 },
  args: ['--disable-background-networking', '--disable-component-update', '--disable-sync'],
});
const page = context.pages()[0];
await page.emulateMedia({ reducedMotion: 'reduce' });
const errors = [];
page.on('pageerror', (error) => errors.push(error.message));
await page.goto('http://127.0.0.1:1420');
await expect(page.locator('textarea')).toBeFocused();
await page.locator('.language-select select').selectOption('zh-CN');
await page.screenshot({ path: path.join(output, 'input.png') });
await page.goto('http://127.0.0.1:1420/?preview=1');
await expect(page.locator('.result-workspace')).toBeVisible();
await page.screenshot({ path: path.join(output, 'result.png') });
for (const tab of ['compare', 'graph', 'evidence', 'why']) {
  await page.locator('#tab-' + tab).click();
  await page.screenshot({ path: path.join(output, tab + '.png') });
}
await page.locator('#tab-preview').click();
await page.setViewportSize({ width: 680, height: 520 });
const dimensions = [];
for (const locale of ['zh-CN', 'en', 'ja']) {
  await page.locator('.language-select select').selectOption(locale);
  await page.screenshot({ path: path.join(output, 'minimum-' + locale + '.png') });
  const size = await page.evaluate(() => ({
    viewport: [innerWidth, innerHeight],
    document: [document.documentElement.scrollWidth, document.documentElement.scrollHeight],
  }));
  expect(size.document).toEqual(size.viewport);
  dimensions.push({ locale, ...size });
}
if (errors.length) throw new Error(errors.join('\n'));
await writeFile(
  path.join(output, 'result.json'),
  JSON.stringify({ passed: true, dimensions, errors }, null, 2),
);
console.log(JSON.stringify({ output, dimensions, errors }));
await context.close();
