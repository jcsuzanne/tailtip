/**
 * TailTip v3 resolver.
 *
 * Usage: node resolve.cjs <path/to/tailwind.config.js>
 * Prints a JSON document on stdout: { version, source, prefix, groups: [...] }
 *
 * - The config file is always loaded with a bundled jiti (handles CJS, ESM and TS).
 * - The project's own `tailwindcss` is used to resolve the theme when installed,
 *   otherwise the bundled Tailwind 3.4 is used.
 * - Any module that cannot be resolved from the project (plugins, presets…) is
 *   replaced by a no-op stub so the theme can still be read.
 */
const path = require('path');
const fs = require('fs');
const Module = require('module');

const bundled = {
  resolveConfig: require('tailwindcss/resolveConfig'),
  defaultTheme: require('tailwindcss/defaultTheme'),
  defaultConfig: require('tailwindcss/defaultConfig'),
  colors: require('tailwindcss/colors'),
  plugin: require('tailwindcss/plugin'),
  version: require('tailwindcss/package.json').version,
};
const createJiti = require('jiti');

const configPath = path.resolve(process.argv[2] || '');
const projectDir = path.dirname(configPath);
const shimDir = path.join(__dirname, 'shims');

// Expose bundled tailwind pieces to the shims.
globalThis.__TAILTIP_TW__ = bundled;

const SHIMS = {
  tailwindcss: 'tailwindcss.cjs',
  'tailwindcss/plugin': 'plugin.cjs',
  'tailwindcss/colors': 'colors.cjs',
  'tailwindcss/defaultTheme': 'defaultTheme.cjs',
  'tailwindcss/defaultTheme.js': 'defaultTheme.cjs',
  'tailwindcss/defaultConfig': 'defaultConfig.cjs',
  'tailwindcss/resolveConfig': 'resolveConfig.cjs',
  'tailwindcss/resolveConfig.js': 'resolveConfig.cjs',
};

const originalResolve = Module._resolveFilename;
Module._resolveFilename = function (request, parent, ...rest) {
  try {
    return originalResolve.call(this, request, parent, ...rest);
  } catch (err) {
    if (Module.isBuiltin && Module.isBuiltin(request)) throw err;
    if (request.startsWith('.') || path.isAbsolute(request)) throw err;
    if (SHIMS[request]) return path.join(shimDir, SHIMS[request]);
    return path.join(shimDir, 'stub.cjs');
  }
};

function tryProjectRequire(id) {
  try {
    const fake = { id: configPath, filename: configPath, paths: Module._nodeModulePaths(projectDir) };
    return require(originalResolve.call(Module, id, fake, false));
  } catch {
    return null;
  }
}

function isPlainObject(v) {
  return v !== null && typeof v === 'object' && !Array.isArray(v);
}

function formatValue(section, v) {
  if (v == null) return '';
  if (typeof v === 'string' || typeof v === 'number') return String(v);
  if (Array.isArray(v)) {
    if (section === 'fontSize') {
      const [size, opts] = v;
      if (typeof opts === 'string') return `${size} / ${opts}`;
      if (isPlainObject(opts) && opts.lineHeight) return `${size} / ${opts.lineHeight}`;
      return String(size);
    }
    return v.map((x) => (typeof x === 'string' ? x : JSON.stringify(x))).join(', ');
  }
  if (isPlainObject(v)) {
    if (section === 'screens') {
      if (v.raw) return v.raw;
      const parts = [];
      if (v.min) parts.push(`min ${v.min}`);
      if (v.max) parts.push(`max ${v.max}`);
      if (parts.length) return parts.join(' · ');
    }
    return JSON.stringify(v);
  }
  return String(v);
}

/** Flatten a theme section into [{ name, value }]. Nested keys are joined with "-"; DEFAULT collapses. */
function flatten(section, obj, prefix = [], out = []) {
  for (const key of Object.keys(obj)) {
    // Skip deprecated getters (lightBlue, warmGray…) — they only log warnings.
    const desc = Object.getOwnPropertyDescriptor(obj, key);
    if (desc && desc.get) continue;
    const value = obj[key];
    if (typeof value === 'function') continue;
    const parts = key === 'DEFAULT' ? prefix : [...prefix, key];
    const nestable =
      isPlainObject(value) &&
      section !== 'screens' &&
      !(section === 'fontSize') &&
      !Object.keys(value).some((k) => ['min', 'max', 'raw', 'lineHeight'].includes(k));
    if (nestable) {
      flatten(section, value, parts, out);
    } else {
      out.push({ name: parts.length ? parts.join('-') : 'DEFAULT', value: formatValue(section, value), path: parts });
    }
  }
  return out;
}

function customKeysFor(config) {
  const custom = {};
  const theme = isPlainObject(config.theme) ? config.theme : {};
  for (const [section, value] of Object.entries(theme)) {
    if (section === 'extend') continue;
    custom[section] = { all: true };
  }
  const extend = isPlainObject(theme.extend) ? theme.extend : {};
  for (const [section, value] of Object.entries(extend)) {
    if (custom[section]) continue;
    if (typeof value === 'function') {
      custom[section] = { all: true };
      continue;
    }
    if (!isPlainObject(value)) continue;
    const names = new Set(flatten(section, value).map((t) => t.name));
    custom[section] = { names: [...names] };
  }
  return custom;
}

function main() {
  if (!fs.existsSync(configPath)) throw new Error(`Config introuvable : ${configPath}`);

  const jiti = createJiti(configPath, { interopDefault: true, cache: false, requireCache: false });
  let config = jiti(configPath);
  if (config && config.default) config = config.default;
  if (!isPlainObject(config)) throw new Error('La config ne retourne pas un objet');

  const projectResolve = tryProjectRequire('tailwindcss/resolveConfig');
  const projectPkg = tryProjectRequire('tailwindcss/package.json');
  const resolveConfig = projectResolve || bundled.resolveConfig;
  const resolved = resolveConfig(config);
  const custom = customKeysFor(config);

  const groups = [];
  for (const [section, value] of Object.entries(resolved.theme || {})) {
    if (!isPlainObject(value)) continue;
    const c = custom[section];
    const tokens = flatten(section, value).map((t) => ({
      name: t.name,
      value: t.value,
      path: t.path,
      custom: !!c && (c.all || c.names.includes(t.name)),
    }));
    if (tokens.length) groups.push({ key: section, tokens });
  }

  let prefix = resolved.prefix || '';
  if (typeof prefix === 'function') prefix = '';

  return {
    version: projectPkg ? projectPkg.version : bundled.version,
    source: projectResolve ? 'project' : 'bundled',
    prefix,
    separator: resolved.separator || ':',
    groups,
  };
}

try {
  process.stdout.write(JSON.stringify(main()));
} catch (err) {
  process.stdout.write(JSON.stringify({ error: String((err && err.message) || err) }));
  process.exitCode = 1;
}
