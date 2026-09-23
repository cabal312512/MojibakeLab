import { readFile, readdir, writeFile, mkdir, stat } from 'node:fs/promises';
import os from 'node:os';
import path from 'node:path';
const root = path.resolve(import.meta.dirname, '..');
const entries = [];
async function collect(dir, label) {
  let files;
  try {
    files = await readdir(dir, { withFileTypes: true });
  } catch {
    return;
  }
  for (const file of files) {
    if (file.isFile() && /^(license|copying|copyright|notice)([._-]|$)/i.test(file.name)) {
      const text = await readFile(path.join(dir, file.name), 'utf8');
      entries.push(`${'='.repeat(72)}\n${label} / ${file.name}\n${'='.repeat(72)}\n${text}`);
    }
  }
}
const lock = JSON.parse(await readFile(path.join(root, 'package-lock.json'), 'utf8'));
for (const [dir, pkg] of Object.entries(lock.packages)) {
  if (dir && !pkg.dev)
    await collect(
      path.join(root, dir),
      `${dir} ${pkg.version ?? ''} (${pkg.license ?? 'see text'})`,
    );
}
const projectCargoHome = path.join(root, '.cache', 'cargo');
const cargoHome = process.env.CARGO_HOME
  ? path.resolve(process.env.CARGO_HOME)
  : (await stat(projectCargoHome).catch(() => null))?.isDirectory()
    ? projectCargoHome
    : path.join(os.homedir(), '.cargo');
const registry = path.join(cargoHome, 'registry', 'src');
for (const registryDir of await readdir(registry).catch(() => [])) {
  const dir = path.join(registry, registryDir);
  for (const crate of await readdir(dir)) await collect(path.join(dir, crate), crate);
}
entries.sort();
await mkdir(path.join(root, 'docs'), { recursive: true });
await writeFile(path.join(root, 'THIRD_PARTY_LICENSES.txt'), entries.join('\n\n'), 'utf8');
console.log(`Collected ${entries.length} third-party license files.`);
