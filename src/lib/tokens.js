// Group metadata and class building for Tailwind v3 / v4 tokens.

export const GROUP_LABELS = {
  colors: 'Couleurs',
  fontFamily: 'Polices',
  fontSize: 'Tailles de texte',
  fontWeight: 'Graisses',
  lineHeight: 'Interlignage',
  letterSpacing: 'Interlettrage',
  spacing: 'Spacing',
  screens: 'Breakpoints',
  containers: 'Containers',
  borderRadius: 'Arrondis',
  boxShadow: 'Ombres',
  insetShadow: 'Ombres internes',
  dropShadow: 'Drop shadows',
  textShadow: 'Ombres de texte',
  zIndex: 'Z-index',
  maxWidth: 'Max width',
  width: 'Width',
  height: 'Height',
  minHeight: 'Min height',
  maxHeight: 'Max height',
  padding: 'Padding',
  margin: 'Margin',
  gap: 'Gap',
  transitionTimingFunction: 'Easings',
  transitionDuration: 'Durées',
  animation: 'Animations',
  aspectRatio: 'Ratios',
  gridTemplateColumns: 'Grid colonnes',
  gridColumn: 'Grid span',
  blur: 'Flous',
  other: 'Variables',
};

const ORDER = [
  'colors',
  'fontFamily',
  'fontSize',
  'fontWeight',
  'lineHeight',
  'letterSpacing',
  'spacing',
  'screens',
  'containers',
  'borderRadius',
  'boxShadow',
  'zIndex',
  'maxWidth',
  'transitionTimingFunction',
  'animation',
];

export function groupLabel(key) {
  return GROUP_LABELS[key] ?? key.replace(/([A-Z])/g, ' $1').replace(/^./, (c) => c.toUpperCase());
}

export function sortGroups(groups) {
  const rank = (k) => (k === 'other' ? 1000 : ORDER.includes(k) ? ORDER.indexOf(k) : 100);
  return [...groups].sort((a, b) => rank(a.key) - rank(b.key) || a.key.localeCompare(b.key));
}

/** Utilities a token of the group can be used with. First one is the default. */
export const UTILITIES = {
  colors: ['bg', 'text', 'border', 'fill', 'stroke', 'ring', 'outline', 'decoration', 'from', 'via', 'to', 'divide', 'accent', 'caret', 'placeholder'],
  spacing: ['p', 'px', 'py', 'pt', 'pb', 'm', 'mx', 'my', 'mt', 'mb', 'gap', 'w', 'h', 'size', 'top', 'left', 'inset', 'space-y', 'space-x', 'translate-y'],
  padding: ['p', 'px', 'py', 'pt', 'pr', 'pb', 'pl'],
  margin: ['m', 'mx', 'my', 'mt', 'mr', 'mb', 'ml'],
  gap: ['gap', 'gap-x', 'gap-y'],
  inset: ['inset', 'top', 'right', 'bottom', 'left'],
  fontSize: ['text'],
  fontFamily: ['font'],
  fontWeight: ['font'],
  lineHeight: ['leading'],
  letterSpacing: ['tracking'],
  borderRadius: ['rounded', 'rounded-t', 'rounded-b', 'rounded-l', 'rounded-r'],
  borderWidth: ['border', 'border-t', 'border-b', 'border-l', 'border-r'],
  boxShadow: ['shadow'],
  insetShadow: ['inset-shadow'],
  dropShadow: ['drop-shadow'],
  textShadow: ['text-shadow'],
  blur: ['blur', 'backdrop-blur'],
  zIndex: ['z'],
  width: ['w'],
  height: ['h'],
  minWidth: ['min-w'],
  minHeight: ['min-h'],
  maxWidth: ['max-w'],
  maxHeight: ['max-h'],
  opacity: ['opacity'],
  transitionTimingFunction: ['ease'],
  transitionDuration: ['duration'],
  transitionDelay: ['delay'],
  animation: ['animate'],
  aspectRatio: ['aspect'],
  gridTemplateColumns: ['grid-cols'],
  gridTemplateRows: ['grid-rows'],
  gridColumn: ['col'],
  gridColumnStart: ['col-start'],
  gridColumnEnd: ['col-end'],
  gridRow: ['row'],
  backgroundImage: ['bg'],
  perspective: ['perspective'],
  scale: ['scale'],
  rotate: ['rotate'],
  translate: ['translate-x', 'translate-y'],
  containers: ['@', 'max-w'],
  screens: ['variant'],
};

/** Display value: resolved var() for v4, alpha placeholders filled for v3. */
export function displayValue(token) {
  return (token.resolved ?? token.value).replaceAll('<alpha-value>', '1');
}

export function className(theme, groupKey, util, token) {
  if (!util) return null;
  const { name } = token;
  if (groupKey === 'spacing' && name === 'base' && theme.kind === 'v4') return null;
  if (util === 'variant') return `${name}:`;
  if (util === '@') return `@${name}:`;

  const base = name === 'DEFAULT' ? util : `${util}-${name}`;
  if (!theme.prefix) return base;
  return theme.kind === 'v4' ? `${theme.prefix}:${base}` : `${theme.prefix}${base}`;
}

/** Reference usable in CSS: var(--x) for v4, theme() for v3. */
export function cssReference(theme, groupKey, token) {
  if (token.varName) return `var(${token.varName})`;
  return `theme('${groupKey}.${token.name.replaceAll('-', '.')}')`;
}

export function isColorLike(value) {
  return /^(#|rgb|hsl|oklch|oklab|lab|lch|color\(|color-mix)/i.test(value.trim());
}

const PX_PER_REM = 16;
export function toPx(value) {
  const m = /^(-?[\d.]+)(px|rem|em)?$/.exec(value.trim());
  if (!m) return null;
  const n = parseFloat(m[1]);
  return m[2] === 'rem' || m[2] === 'em' ? n * PX_PER_REM : n;
}

// Utility picked per group (bg / text / border…), remembered across sessions.
const UTIL_KEY = 'tailtip:utils';

export function readUtilPrefs() {
  try {
    return JSON.parse(localStorage.getItem(UTIL_KEY) || '{}');
  } catch {
    return {};
  }
}

export function writeUtilPrefs(prefs) {
  try {
    localStorage.setItem(UTIL_KEY, JSON.stringify(prefs));
  } catch {}
}

export function utilFor(prefs, groupKey) {
  const options = UTILITIES[groupKey];
  if (!options) return null;
  return options.includes(prefs[groupKey]) ? prefs[groupKey] : options[0];
}
