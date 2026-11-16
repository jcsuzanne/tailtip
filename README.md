# TailTip

**English** · [Français](README.fr.md)

A macOS menu bar utility for quick access to the Tailwind tokens (colors, fonts, spacing, breakpoints…) of all your projects.

- **Tailwind v3/v2**: `tailwind.config.{js,cjs,mjs,ts}` is resolved with Node (the project's own Tailwind, or a bundled Tailwind 3.4 when it isn't installed).
- **Tailwind v4**: reads the `@theme` blocks of the CSS entry file (`@import "tailwindcss"`) and its local `@import`s.

## Installation

[⬇︎ Download the latest release](https://github.com/jcsuzanne/tailtip/releases/latest) (Apple Silicon Mac, Node.js required for v3 configs).

Unzip, move `TailTip.app` to `/Applications`, then remove the quarantine flag (the app is not notarized, macOS would otherwise report it as "damaged"):

```sh
xattr -cr /Applications/TailTip.app
```

Add your project folders in Settings (⚙).

## Usage

- Click the menu bar icon (or press `⌥⌘T`) → project list, search, `↑↓` + `Enter`.
- Search also covers the custom tokens of **every** project (by name or value, e.g. `ivory`, `#f9af26`).
- Click a token → copies the class (`tw-bg-cognac`, `tw:bg-ivory`…), with the project's prefix.
- `⌥`-click → copies the value · `⇧`-click → `var(--…)` (v4) / `theme('…')` (v3).
- Pick the utility copied per group (`bg`, `text`, `border`…) from the group header.
- ↗ icon / right-click a color → opens the token's line in Cursor.
- Configs are watched: any change (including v4 `@import`s) reloads the theme.
- Settings → “Launch TailTip at login”.
- `Esc` → back / close.

## Development

Requirements: Node, pnpm, Rust (`brew install rust` or rustup).

```sh
pnpm install
pnpm build:resolver               # bundles the v3 resolver (src-tauri/resolver)
pnpm tauri dev
pnpm tauri build --bundles app    # → src-tauri/target/release/bundle/macos/TailTip.app
```

Settings are stored in `~/Library/Application Support/dev.tailtip.app/settings.json`.

## How it works

- **Scan** — walks the configured folders (skipping `node_modules`, `vendor`, build output, hidden folders…) looking for v3 config files and v4 CSS entries. CSS files that import Tailwind only to use `@apply` are ignored when the project has a real theme file.
- **v3** — a small Node script (bundled with jiti and Tailwind 3.4) loads the config, resolves it with the project's Tailwind when available, and flags which tokens come from the project vs. Tailwind's defaults. Missing plugins are replaced by no-op stubs, so the theme can still be read.
- **v4** — `@theme` declarations are parsed in Rust, `var()` references are resolved and each token's file and line is kept.
