# Lynk Dev

**Tes dépôts et tes services locaux, dans une seule fenêtre.**

Suite d'outils de développement desktop — **Tauri 2 + React + Rust**.
Local-first, sans compte, sans télémétrie.

[Site & documentation](https://lynk-dev.pages.dev) ·
[Télécharger](https://github.com/moha528/lynk-dev/releases/latest) ·
[Signaler un bug](https://github.com/moha528/lynk-dev/issues)

---

## Ce que ça fait

### Dev Manager — superviser tes services

Pointe un dossier : Lynk Dev reconnaît ce qu'il contient et propose la commande,
le build et le port de chaque service.

- **22 familles reconnues** — Spring Boot (Maven, Gradle), Next, Nuxt, Angular,
  Nest, SvelteKit, Astro, Remix, Vite, Node, Django, FastAPI, Flask, Python, Go,
  Rust, .NET, Laravel, Rails, Docker Compose.
- **Démarrage groupé** ordonné par dépendances — la base avant l'API, l'API
  avant le front.
- **L'état `running` signifie que le port répond**, pas que la commande est
  lancée.
- **Arrêt de l'arbre de process entier.** `mvnw` lance `mvn` qui lance `java` :
  tuer le premier laisserait la JVM vivante et le port occupé.
- Sondes de port et de santé HTTP, redémarrage automatique borné (2 s → 30 s,
  cinq essais), libération d'un port tenu par un orphelin.
- Logs en direct : couleurs ANSI, niveaux mis en évidence, recherche surlignée,
  filtre par flux.

### Git Manager — piloter plusieurs dépôts

- Le statut de tous tes dépôts d'un coup d'œil : branche, avance/retard,
  fichiers modifiés, indexés, en conflit.
- Indexation, validation, `push` — au clavier, avec sélection par plage.
- Diffs colorés, branches, remises, historique, distants, conflits de fusion.
- Fetch / Pull / Push **groupés** sur plusieurs dépôts.

Le binaire `git` de ta machine est appelé directement : tes `credential.helper`,
tes hooks et ton `.gitconfig` se comportent exactement comme dans ton terminal.

### Serveur MCP — donner les yeux à ton assistant

Un serveur MCP sur la boucle locale : ton IA voit ce qui tourne, depuis quand,
lit les logs et peut redémarrer un service.

Quatre outils de lecture, quatre d'écriture, et **aucun outil d'exécution de
commande arbitraire** — ce serait un shell distant déguisé. Jeton obligatoire,
contrôle de l'origine, écriture limitée au profil actif.

### Assistance par modèle — sur un geste, jamais en fond

Message de commit rédigé depuis ton index, explication d'un diff, analyse d'une
sortie de service. Via [OpenRouter](https://openrouter.ai), avec le catalogue
chargé en direct et trié du moins cher au plus cher.

### Le reste

Sept thèmes, palette de commandes (`Ctrl+K`), raccourcis réassignables,
verrouillage par PIN, mises à jour automatiques signées, icône de zone de
notification.

## Sécurité

- **Rien ne quitte ta machine** sans un geste explicite. Pas de compte, pas de
  télémétrie.
- **Les secrets vivent dans le trousseau du système** — Credential Manager,
  Keychain, Secret Service. Jamais en clair dans un fichier. Sans trousseau
  utilisable, l'écran le dit et la fonction s'arrête là : pas de repli discret.
- **Le serveur MCP** n'écoute que sur `127.0.0.1`, exige un jeton comparé à
  durée constante, et vérifie l'en-tête `Origin` (protection contre la
  réattribution DNS).
- **Le verrouillage par PIN est un verrou d'interface**, pas un chiffrement de
  la base locale. C'est dit là plutôt que sous-entendu.

## Installer

Depuis la [page des releases](https://github.com/moha528/lynk-dev/releases/latest) :

| OS | Fichier |
| --- | --- |
| Windows | `.exe` (NSIS) ou `.msi` |
| macOS | `.dmg` universel |
| Linux | `.AppImage` ou `.deb` |

Les installeurs ne sont **pas signés** au niveau du système : Windows et macOS
affichent un avertissement au premier lancement. La
[FAQ](https://lynk-dev.pages.dev/docs/faq) donne les deux clics de
contournement.

## Développer

```bash
pnpm install
pnpm tauri dev            # lance l'application (Vite + Rust)
```

Vérifications, celles que la CI rejoue :

```bash
pnpm lint                 # Biome
pnpm build                # tsc + vite

cd src-tauri
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features
```

## Architecture

Le principe qui gouverne le backend : **un seul superviseur, plusieurs façades.**

Les serveurs de développement sont des enfants du process Lynk Dev. Le
superviseur Rust (`src-tauri/src/dev/supervisor.rs`) **ne dépend pas de Tauri** :
il émet sur un canal de diffusion, et l'application comme le serveur MCP s'y
abonnent. Lui passer un `AppHandle` obligerait le MCP à réécrire toute la
logique — et produirait deux vérités concurrentes sur ce qui tourne.

```
src-tauri/src/
├── dev/          Dev Manager — superviseur, détection, scan, sondes, logs
├── git/          Git Manager — opérations git, parseurs purs, terminal
├── mcp/          Serveur MCP — protocole, outils, journal, transport HTTP
├── ai/           Client OpenRouter et consignes
├── secrets.rs    Trousseau du système
├── store/        SQLite — profils, réglages
└── commands/     Ponts IPC. Aucune logique métier ici.
```

Le même interdit vaut pour `commands/` et pour `mcp/tools.rs` : toute règle
glissée à ce niveau ne vaudrait que pour une façade, et divergerait de l'autre.

## Publier une version

1. Mettre à jour la version dans **`package.json` et `src-tauri/tauri.conf.json`**
   — les deux, sinon l'updater propose une mise à jour qui ne s'applique jamais.
2. Mettre à jour le `CHANGELOG.md`.
3. `git tag vX.Y.Z && git push origin vX.Y.Z`

Le workflow construit sur les trois systèmes et crée une release **en
brouillon** : les installeurs sont testables avant d'être visibles.

## Limites connues

- Développé et éprouvé au quotidien **sur Windows**. Les chemins POSIX du
  superviseur (groupe de process, `kill(-pid)`) sont construits et testés sur
  Linux et macOS à chaque modification, mais leur comportement à l'exécution y
  est moins éprouvé.
- Le **DB Explorer** de la version Electron n'est pas porté, et ne le sera pas.

## Licence

[FSL-1.1-MIT](LICENSE.md) — usage libre, y compris commercial, sauf pour en
faire un produit concurrent. Devient MIT au bout de deux ans.
