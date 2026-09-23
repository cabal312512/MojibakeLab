import { readFile, readdir, mkdir, cp, copyFile } from 'node:fs/promises';
import { execFileSync } from 'node:child_process';
import path from 'node:path';

const root = path.resolve(import.meta.dirname, '..');
const { version } = JSON.parse(await readFile(path.join(root, 'package.json'), 'utf8'));
const platform = process.argv[2];
const platforms = {
  windows: { id: 'windows-x64', output: 'target/release', binary: 'mojibake-lab.exe' },
  macos: {
    id: 'macos-universal',
    output: 'target/universal-apple-darwin/release',
    binary: 'mojibake-lab',
  },
  linux: { id: 'linux-x64', output: 'target/release', binary: 'mojibake-lab' },
};
const spec = platforms[platform];
if (!spec) throw new Error('Specify windows, macos, or linux');
const output = path.join(root, spec.output);
const artifacts = path.join(root, 'artifacts');
const stage = path.join(root, '.tmp/release-stage', platform, 'MojibakeLab');
await mkdir(artifacts, { recursive: true });
await mkdir(stage, { recursive: true });
const prefix = `MojibakeLab-${version}-${spec.id}`;
for (const name of [
  'README.md',
  'README.en.md',
  'LICENSE',
  'THIRD_PARTY_NOTICES.md',
  'THIRD_PARTY_LICENSES.txt',
])
  await copyFile(path.join(root, name), path.join(stage, name));
await cp(path.join(root, 'docs'), path.join(stage, 'docs'), { recursive: true });

async function bundle(folder, suffix, destination) {
  const dir = path.join(output, 'bundle', folder);
  const found = (await readdir(dir)).filter((name) => name.endsWith(suffix));
  if (found.length !== 1) throw new Error(`Expected one ${suffix} in ${dir}, got ${found.length}`);
  await copyFile(path.join(dir, found[0]), path.join(artifacts, destination));
}
if (platform === 'windows') {
  const exe = path.join(output, spec.binary);
  await copyFile(exe, path.join(stage, 'MojibakeLab.exe'));
  await copyFile(exe, path.join(artifacts, `${prefix}-portable.exe`));
  await copyFile(path.join(root, 'docs/PORTABLE.txt'), path.join(stage, 'ReadMe.txt'));
  // Pass paths via environment, never splice them into PowerShell code.
  execFileSync(
    'pwsh',
    [
      '-NoProfile',
      '-Command',
      'Compress-Archive -LiteralPath $env:MOJIBAKE_STAGE -DestinationPath $env:MOJIBAKE_ARCHIVE -Force',
    ],
    {
      stdio: 'inherit',
      env: {
        ...process.env,
        MOJIBAKE_STAGE: stage,
        MOJIBAKE_ARCHIVE: path.join(artifacts, `${prefix}-portable.zip`),
      },
    },
  );
  await bundle('nsis', '.exe', `${prefix}-setup.exe`);
} else if (platform === 'macos') {
  const app = path.join(output, 'bundle/macos/Mojibake Lab.app');
  await cp(app, path.join(stage, 'Mojibake Lab.app'), { recursive: true, verbatimSymlinks: true });
  execFileSync('codesign', ['--verify', '--deep', '--strict', app], { stdio: 'inherit' });
  execFileSync(
    'lipo',
    [path.join(app, 'Contents/MacOS/mojibake-lab'), '-verify_arch', 'x86_64', 'arm64'],
    { stdio: 'inherit' },
  );
  execFileSync(
    'ditto',
    [
      '-c',
      '-k',
      '--sequesterRsrc',
      '--keepParent',
      stage,
      path.join(artifacts, `${prefix}.app.zip`),
    ],
    { stdio: 'inherit' },
  );
  await bundle('dmg', '.dmg', `${prefix}.dmg`);
} else {
  await copyFile(path.join(output, spec.binary), path.join(stage, spec.binary));
  execFileSync(
    'tar',
    ['-czf', path.join(artifacts, `${prefix}.tar.gz`), '-C', path.dirname(stage), 'MojibakeLab'],
    { stdio: 'inherit' },
  );
  await bundle('appimage', '.AppImage', `${prefix}.AppImage`);
  await bundle('deb', '.deb', `${prefix}.deb`);
}
console.log(`Packaged ${prefix}`);
