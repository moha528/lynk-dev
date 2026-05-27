# Lynk Dev

Suite d'outils de développement desktop (Git Manager, Dev Manager, DB Explorer) —
**Tauri 2 + React + Rust**.

Ce dépôt est pour l'instant un **template de base** dérivé de Lynk Client : il fournit
le squelette commun et laisse la place aux modules métier.

## Inclus dans le template

- Fenêtre + **icône de zone de notification (tray)** + comportement de fermeture
  configurable (quitter / tray / réduire / demander)
- **Thèmes multiples** (Catppuccin, Tokyo Night, Dracula, Solarized, Gruvbox) via
  variables CSS — `src/lib/themes.ts`
- **Réglages** persistés (SQLite) — sections Apparence, Raccourcis, Sécurité, À propos
- **Raccourcis clavier** personnalisables + palette de commandes (Ctrl+K)
- **Verrouillage par PIN** (Argon2id) + auto-lock après inactivité
- **Auto-update** (Tauri updater, à configurer avec une clé de signature)
- Chrome de fenêtre natif Windows (Mica / couleur de titre)

## Développement

```bash
pnpm install
pnpm tauri dev      # lance l'app (Vite + Rust)
```

## Build

```bash
pnpm tauri build    # installeurs par plateforme
```

> Avant la première release : régénérer la clé de signature de l'updater
> (`pnpm tauri signer generate`) et renseigner `pubkey` dans
> `src-tauri/tauri.conf.json`, puis remettre `createUpdaterArtifacts: true`.

## Licence

FSL-1.1-MIT (source-available).
