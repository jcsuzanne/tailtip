import { build } from 'esbuild';
import { cpSync, rmSync, mkdirSync } from 'node:fs';

rmSync('dist', { recursive: true, force: true });
mkdirSync('dist', { recursive: true });
await build({
  entryPoints: ['src/resolve.cjs'],
  bundle: true,
  platform: 'node',
  format: 'cjs',
  target: 'node18',
  outfile: 'dist/resolve.cjs',
  logLevel: 'warning',
});
cpSync('src/shims', 'dist/shims', { recursive: true });
