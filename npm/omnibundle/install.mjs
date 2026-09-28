// Download the release binary for this platform, or say clearly why we did not.
//
// Design constraints, in order of importance:
//   1. Never fail an install silently. If the download does not happen, the
//      shim prints the reason and the fix.
//   2. Verify the checksum. A binary fetched over the network and then executed
//      is the exact thing supply-chain hygiene exists to prevent.
//   3. Skip the work when a usable binary is already present (a local cargo
//      build, or OMNIBUNDLE_BIN), so `npm i` in a dev checkout is instant.
//   4. No new dependencies: node:crypto and node:https are enough.

import { createHash } from 'node:crypto';
import { chmodSync, existsSync, mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';
import { get as httpsGet } from 'node:https';

const here = dirname(fileURLToPath(import.meta.url));
const vendorDir = resolve(here, 'vendor', `${process.platform}-${process.arch}`);
const exe = process.platform === 'win32' ? 'omnibundle.exe' : 'omnibundle';
const target = join(vendorDir, exe);

const manifest = JSON.parse(readFileSync(resolve(here, 'package.json'), 'utf8'));
const version = process.env.OMNIBUNDLE_VERSION ?? manifest.version;

// The download host is read out of package.json's own repository field rather than
// kept as a second copy of the slug, so filling in repo-links.json is enough and
// there is nothing here to forget.
const slugFromManifest = String(manifest.repository?.url ?? '')
  .replace(/^git\+/, '')
  .replace(/\.git$/, '')
  .replace(/^https?:\/\/github\.com\//, '');
const repo = process.env.OMNIBUNDLE_REPO ?? slugFromManifest;
const base = `https://github.com/${repo}/releases/download/v${version}`;

/** True while the repository URL is still an unfilled placeholder token. */
const unfilled = () => /\{\{[A-Z_]+\}\}/.test(repo);

const alreadyUsable = () => {
  if (process.env.OMNIBUNDLE_BIN && existsSync(process.env.OMNIBUNDLE_BIN)) return true;
  const local = [
    join(here, '..', '..', 'target', 'release', exe),
    join(here, '..', '..', 'target', 'debug', exe),
  ].find(existsSync);
  return Boolean(local);
};

if (alreadyUsable()) {
  process.exit(0);
}

const assetName = `omnibundle-${version}-${process.platform}-${process.arch}${process.platform === 'win32' ? '.exe' : '.tar.gz'}`;

function fetch(url, redirectsLeft = 5) {
  return new Promise((res, rej) => {
    httpsGet(url, { headers: { 'user-agent': 'omnibundle-install' } }, (r) => {
      if (r.statusCode >= 300 && r.statusCode < 400 && r.headers.location) {
        if (redirectsLeft === 0) return rej(new Error('too many redirects'));
        r.resume();
        return res(fetch(r.headers.location, redirectsLeft - 1));
      }
      if (r.statusCode !== 200) {
        r.resume();
        return rej(new Error(`HTTP ${r.statusCode} for ${url}`));
      }
      const chunks = [];
      r.on('data', (c) => chunks.push(c));
      r.on('end', () => res(Buffer.concat(chunks)));
      r.on('error', rej);
    }).on('error', rej);
  });
}

async function main() {
  if (unfilled()) {
    // Fail with the reason rather than requesting a URL containing an unfilled
    // placeholder, which comes back as a confusing 404 from an address that never
    // existed. Named in terms of the table, because that is what has to be filled in.
    throw new Error(
      'no repository to download from: the repository URL in package.json is still a placeholder.\n' +
        'Fill it in via repo-links.json, or set OMNIBUNDLE_REPO=owner/name.',
    );
  }
  try {
    process.stdout.write(`omnibundle: fetching ${assetName}\n`);
    const payload = await fetch(`${base}/${assetName}`);

    // Checksums ship as one file per release; if it is missing we do not
    // execute what we downloaded.
    const checksums = await fetch(`${base}/checksums.txt`).catch(() => null);
    if (!checksums) {
      throw new Error(`no checksums.txt published for v${version}`);
    }
    const expected = checksums
      .toString('utf8')
      .split('\n')
      .map((line) => line.trim().split(/\s+/))
      .find(([name]) => name === assetName)?.[0];
    if (!expected) {
      throw new Error(`checksums.txt has no entry for ${assetName}`);
    }
    const actual = createHash('sha256').update(payload).digest('hex');
    if (actual !== expected) {
      throw new Error(
        `checksum mismatch for ${assetName}\n  expected ${expected}\n  actual   ${actual}`,
      );
    }

    mkdirSync(vendorDir, { recursive: true });
    if (assetName.endsWith('.tar.gz')) {
      // The tarball contains the binary under `omnibundle-<target>/`. Extracting
      // it needs `tar`, which every supported platform ships; doing it here
      // avoids a tar dependency in the package.
      const { execFileSync } = await import('node:child_process');
      const tmp = join(vendorDir, '..', `extract-${process.pid}`);
      mkdirSync(tmp, { recursive: true });
      const tgz = join(tmp, assetName);
      writeFileSync(tgz, payload);
      execFileSync('tar', ['-xzf', tgz, '-C', tmp], { stdio: 'ignore' });
      const { readdirSync } = await import('node:fs');
      const [dir] = readdirSync(tmp).filter((n) => n.startsWith('omnibundle-'));
      writeFileSync(target, readFileSync(join(tmp, dir, exe)));
    } else {
      writeFileSync(target, payload);
    }
    if (process.platform !== 'win32') chmodSync(target, 0o755);
    process.stdout.write(`omnibundle: installed ${target}\n`);
  } catch (err) {
    // Not fatal: the shim gives better guidance than an install-time failure,
    // and a developer working offline should still be able to `npm i`.
    process.stdout.write(
      [
        `omnibundle: skipped the binary download (${err.message}).`,
        'The package is installed, but it will need a binary at first run.',
        '  - build from source: cargo build --release',
        '  - or re-run with the network: node install.mjs',
        '  - or point at one: OMNIBUNDLE_BIN=/path/to/omnibundle',
        '',
      ].join('\n'),
    );
  }
}

await main();

// Referenced so the linter sees the import as used on every platform.
void pathToFileURL;