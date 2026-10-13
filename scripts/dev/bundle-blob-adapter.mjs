// The upstream build must have verified peers and applied its maintained SDK patch.
import { createRequire } from 'node:module';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { resolve, join } from 'node:path';
import { createHash } from 'node:crypto';

const [sourceArgument, outputArgument, commit, ...extra] = process.argv.slice(2);
if (!sourceArgument || !outputArgument || !/^[a-f0-9]{40}$/.test(commit ?? '') || extra.length)
  throw new Error('Usage: bundle-blob-adapter.mjs RELEASED_SOURCE NEW_OUTPUT SOURCE_COMMIT');
const source = resolve(sourceArgument), output = resolve(outputArgument);
const require = createRequire(join(source, 'tests/browser/package.json'));
const { build } = require('esbuild');
const peer = JSON.parse(await readFile(join(source, 'clients/browser/package.json')));
const nodePaths = [join(source, 'tests/browser/node_modules')];
await mkdir(output, { mode: 0o700 });
const common = { bundle: true, format: 'esm', platform: 'node', nodePaths,
  external: ['/host.js'], // Upstream launcher evaluates this import inside Chromium.
  alias: { '@caffeineai/object-storage': join(source, '.tmp/browser/caffeine/dist/index.js') },
  banner: { js: "import { createRequire } from 'node:module'; const require = createRequire(import.meta.url);" },
};
await build({ ...common, stdin: { contents: `export { StorageClient } from '@caffeineai/object-storage';
export { Ed25519KeyIdentity } from '@icp-sdk/core/identity';
export { publicationConfiguration } from './clients/browser/worker-configuration.js';`,
  resolveDir: source, sourcefile: 'ic-crates-sdk.mjs' }, outfile: join(output, 'sdk.mjs') });
await build({ ...common, entryPoints: [join(source, 'clients/browser/launcher.mjs')],
  outfile: join(output, 'launcher.mjs') });
const files = ['sdk.mjs', 'launcher.mjs'];
const hashes = Object.fromEntries(await Promise.all(files.map(async file =>
  [file, createHash('sha256').update(await readFile(join(output, file))).digest('hex')])));
await writeFile(join(output, 'selection.json'), JSON.stringify({ source_commit: commit,
  node: process.version, peers: peer.peerDependencies, hashes }, null, 2), { flag: 'wx', mode: 0o600 });
