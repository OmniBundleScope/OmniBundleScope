// The licence copies inside each crate must be byte-identical to the canonical
// ones at the repository root.
//
// Why copies at all: crates.io renders the licence text from files *inside* the
// published package, and a dual `MIT OR Apache-2.0` crate has no single
// `license-file` to point at. Every dual-licensed Rust project keeps copies next
// to each manifest. The failure mode that comes with copies is drift - someone
// edits the root licence and the published one is a year old - so the copies get
// a gate, and this is that gate.
//
//   node bench/harness/check-licenses.mjs

import { readFileSync } from 'node:fs';
import { join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const repoRoot = resolve(fileURLToPath(import.meta.url), '..', '..', '..');
const LICENCES = ['LICENSE-MIT', 'LICENSE-APACHE'];
const CRATES = ['omnibundlescope-core', 'omnibundlescope-cli', 'omnibundlescope-wasm'];

let failures = 0;

for (const licence of LICENCES) {
  const canonicalPath = join(repoRoot, licence);
  let canonical;
  try {
    canonical = readFileSync(canonicalPath, 'utf8');
  } catch {
    console.error(`FAIL ${licence}: missing at the repository root`);
    failures++;
    continue;
  }

  for (const crate of CRATES) {
    const copyPath = join(repoRoot, 'crates', crate, licence);
    let copy;
    try {
      copy = readFileSync(copyPath, 'utf8');
    } catch {
      console.error(`FAIL crates/${crate}/${licence}: missing`);
      console.error('     crates.io renders the licence from inside the package, so a');
      console.error('     dual-licensed crate needs a copy next to its manifest.');
      failures++;
      continue;
    }
    if (copy !== canonical) {
      console.error(`FAIL crates/${crate}/${licence}: differs from the root copy`);
      const a = canonical.split('\n');
      const b = copy.split('\n');
      for (let i = 0; i < Math.max(a.length, b.length); i++) {
        if (a[i] !== b[i]) {
          console.error(`     first difference at line ${i + 1}:`);
          console.error(`       root:   ${JSON.stringify(a[i])}`);
          console.error(`       crate:  ${JSON.stringify(b[i])}`);
          break;
        }
      }
      failures++;
    }
  }
}

// The SPDX expression in the manifest and the files on disk have to agree, or
// crates.io shows one licence and the package contains another.
const workspaceManifest = readFileSync(join(repoRoot, 'Cargo.toml'), 'utf8');
if (!/^license = "MIT OR Apache-2\.0"$/m.test(workspaceManifest)) {
  console.error('FAIL Cargo.toml: the license field no longer says "MIT OR Apache-2.0"');
  failures++;
} else {
  for (const crate of CRATES) {
    const manifest = readFileSync(join(repoRoot, 'crates', crate, 'Cargo.toml'), 'utf8');
    if (!/^license\.workspace = true$/m.test(manifest)) {
      console.error(`FAIL crates/${crate}/Cargo.toml: license.workspace is not inherited`);
      failures++;
    }
  }
}

if (failures > 0) {
  console.error(`${failures} licence problem(s). Fix: copy the root files over the crate copies.`);
  process.exit(1);
}
console.log(`ok   ${LICENCES.length} licences x ${CRATES.length} crates, byte-identical to the root copies`);
console.log('ok   the SPDX expression matches the files on disk');