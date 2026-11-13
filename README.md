# TailTip

Utilitaire macOS dans la barre de menu : accès rapide aux tokens Tailwind (couleurs, polices, spacing, breakpoints…) de tous tes projets.

- **Tailwind v3/v2** : `tailwind.config.{js,cjs,mjs,ts}` résolu via node (Tailwind du projet, sinon Tailwind 3.4 embarqué).
- **Tailwind v4** : lecture des blocs `@theme` du CSS d'entrée (`@import "tailwindcss"`) et de ses `@import` locaux.

## Utilisation

- Clic sur l'icône (ou `⌥⌘T`) → liste des projets, recherche, `↑↓` + `Entrée`.
- Clic sur un token → copie la classe (`tw-bg-cognac`, `tw:bg-ivory`…), préfixe respecté.
- `⌥` clic → valeur · `⇧` clic → `var(--…)` (v4) / `theme('…')` (v3).
- Icône ↗ / clic droit sur une couleur → ouvre la ligne dans Cursor.
- `Échap` → retour / fermer.

## Dev

Prérequis : Node, pnpm, Rust (`brew install rust` ou rustup).

```sh
pnpm install
pnpm build:resolver      # bundle du résolveur v3 (src-tauri/resolver)
pnpm tauri dev
pnpm tauri build         # → src-tauri/target/release/bundle/{macos,dmg}
```

Réglages stockés dans `~/Library/Application Support/dev.tailtip.app/settings.json`.
