import { build } from 'esbuild';
import { cpSync, rmSync, mkdirSync } from 'node:fs';
import { join } from 'node:path';

const here = import.meta.dirname;
const dist = join(here, 'dist');

rmSync(dist, { recursive: true, force: true });
mkdirSync(dist, { recursive: true });
await build({
  entryPoints: [join(here, 'src/resolve.cjs')],
  bundle: true,
  platform: 'node',
  format: 'cjs',
  target: 'node18',
  outfile: join(dist, 'resolve.cjs'),
  logLevel: 'warning',
});
cpSync(join(here, 'src/shims'), join(dist, 'shims'), { recursive: true });
