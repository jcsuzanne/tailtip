# TailTip

[English](README.md) · **Français**

Utilitaire macOS dans la barre de menu : accès rapide aux tokens Tailwind (couleurs, polices, spacing, breakpoints…) de tous tes projets.

- **Tailwind v3/v2** : `tailwind.config.{js,cjs,mjs,ts}` résolu via node (Tailwind du projet, sinon Tailwind 3.4 embarqué).
- **Tailwind v4** : lecture des blocs `@theme` du CSS d'entrée (`@import "tailwindcss"`) et de ses `@import` locaux.

## Installation

[⬇︎ Télécharger la dernière version](https://github.com/jcsuzanne/tailtip/releases/latest) (Mac Apple Silicon, Node.js requis pour les configs v3).

Dézipper, déplacer `TailTip.app` dans `/Applications`, puis retirer l'attribut de quarantaine (app non notarisée, sinon macOS la dit « endommagée ») :

```sh
xattr -cr /Applications/TailTip.app
```

Ajouter ses dossiers de projets dans les réglages (⚙).

## Utilisation

- Clic sur l'icône (ou `⌥⌘T`) → liste des projets, recherche, `↑↓` + `Entrée`.
- La recherche couvre aussi les tokens perso de **tous** les projets (nom ou valeur, ex. `ivory`, `#f9af26`).
- Les configs sont surveillées : toute modification (y compris des `@import` v4) recharge le thème.
- Réglages → « Lancer TailTip au démarrage du Mac ».
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
pnpm tauri build --bundles app   # → src-tauri/target/release/bundle/macos/TailTip.app
```

Installer : copier `TailTip.app` dans `/Applications`. Non notarisée : pour un collègue, `xattr -cr /Applications/TailTip.app` après téléchargement (ou signature Developer ID + notarisation Apple).

Réglages stockés dans `~/Library/Application Support/dev.tailtip.app/settings.json`.
