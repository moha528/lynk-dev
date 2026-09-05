# Lynk Dev — plan de travail

> **Fichier vivant.** On coche au fur et à mesure ; on met à jour l'état constaté quand une
> hypothèse tombe. Les affirmations factuelles portent leur `fichier:ligne` — un agent qui
> reprend à froid ne doit rien avoir à re-débattre.
>
> Dernière mise à jour : **2026-09-05**

---

## État constaté (2026-09-05) — ce qui EST, pas ce que disent les README

| Dépôt local | GitHub | Nature | Vérifié |
|---|---|---|---|
| `perso/lynk-dev-tauri` (**ce repo**) | `moha528/lynk-dev` | Tauri 2 + React 19 + Rust. **Template**, 2 commits. | `git log` : `3824ef0`, `674595c` |
| `perso/lynk-dev-electron` | `moha528/lynk-dev-electron` | Electron 33. **Référence de comportement** pour Git + Dev. | `electron/*.ts` = 4 611 l. |
| `perso/lynk-client` | `moha528/lynk` | Lynk Client (SSH/SFTP), v1.0.5. **Le modèle** de release. | `src-tauri/tauri.conf.json:4` |

> **Renommage fait des deux côtés le 2026-09-05.** Dossiers locaux : `termiusv2` → `lynk-client`,
> `zeitune-tools` → `lynk-dev-electron`, `lynk-dev` → `lynk-dev-tauri`. Dépôts GitHub :
> `moha528/lynk-dev` (Electron) → `lynk-dev-electron`, puis `lynk-dev-tauri` → **`lynk-dev`**.
> Le dossier local garde le suffixe `-tauri` **exprès** : sur le disque, les deux clones doivent
> rester distinguables au premier coup d'œil.

### Ce que le template a DÉJÀ (ne pas le réécrire)

- Fenêtre + tray + comportement de fermeture — `src-tauri/src/lib.rs:145` (`build_tray`)
- 7 thèmes (Catppuccin Mocha/Latte, Tokyo Night, Dracula, Solarized Dark/Light, Gruvbox Dark) — `src/lib/themes.ts:71-345`
- Réglages persistés SQLite + migrations auto-réparantes (checksum CRLF/LF) — `src-tauri/src/store/db.rs:56`
- Verrouillage PIN Argon2id, palette de commandes, raccourcis configurables
- **Auto-update : le CODE est déjà là et il est identique à celui de Lynk Client.**
  `src/lib/updater.ts` est **byte-identique** au fichier de `lynk-client` (vérifié par `diff`) ;
  le check silencieux au démarrage tourne (`src/components/MainLayout.tsx:39`), le bouton manuel
  aussi (`src/components/AboutSection.tsx:37`), les plugins sont enregistrés
  (`src-tauri/src/lib.rs:60-61`) et les permissions sont accordées
  (`src-tauri/capabilities/default.json`).

> ⚠️ **Correction d'une prémisse de départ.** « Ajouter les mises à jour automatiques » n'est pas
> une feature à écrire : c'est **3 lignes de config + 2 workflows + une clé de signature**. Tout le
> reste existe. Le détail en §2.1.

### Ce qui manque vraiment

- **Les modules** : la sidebar affiche Git/Dev/DB en `disabled` + badge « soon » — `src/components/Sidebar.tsx:30,37`
  (l'entrée DB est à **supprimer**, cf. D8). Le corps de l'app est un placeholder — `src/components/MainLayout.tsx:130-143`.
- Côté Rust : **7 commandes seulement** (settings + vault) — `src-tauri/src/lib.rs:80-88`.
- Aucun `.github/workflows/`, aucun `CHANGELOG.md`.

---

## §0 Décisions figées

| # | Question | Décision | Motif |
|---|---|---|---|
| D1 | Quelle base on étend ? | **Le Tauri** (`perso/lynk-dev-tauri`) | 2026-09-05. Même stack que Lynk Client → updater, CI et release copiables tels quels. L'Electron devient la **référence de comportement**, pas la base de code. |
| D2 | Ordre des travaux | **Port d'abord, vérification, puis ajouts** | 2026-09-05, choix utilisateur. On ne greffe pas des features sur un socle vide. |
| D3 | Périmètre du MCP | **Process locaux du Dev Manager uniquement** | 2026-09-05. Pas de SSH, pas de preprod : aucun secret réseau, aucun risque de redémarrer de la prod par erreur. |
| D4 | Git en Rust : `git2`/libgit2 ou binaire `git` ? | **Binaire `git`** via `tokio::process::Command` | L'Electron fait déjà exactement ça (`electron/git-handlers.ts:56`, `execFile('git', args)`) : le port devient une **traduction 1:1**, mêmes sorties, mêmes parseurs. libgit2 changerait les comportements (credential helpers, hooks, `.gitconfig`) pour zéro gain. |
| D5 | Coloration syntaxique | **Shiki** | Les 7 thèmes de l'app (`src/lib/themes.ts`) existent tels quels dans les thèmes livrés par Shiki → mapping 1:1, zéro palette à réinventer. |
| D6 | Transport MCP | **HTTP sur loopback**, hébergé par l'app | Un binaire MCP lancé par le client IA est un **autre process** : il ne peut pas piloter les enfants de Lynk Dev. Le superviseur doit rester unique → c'est l'app qui expose. Détail et alternative en §3.2. |
| D7 | Modèle OpenRouter | **Configurable, liste chargée en direct** | Figer un identifiant de modèle dans le code le périme en trois mois. Défaut proposé + liste live via `GET /api/v1/models`. |
| D8 | DB Explorer | **Hors périmètre — pas porté** | 2026-09-05, choix utilisateur : « plus besoin ». Aucune des features demandées n'en dépendait, et il pesait ~8 000 des ~18 000 lignes du port. L'app devient **Git + Dev**. |

**Hypothèses assumées** (corriger ici si faux, ne pas demander) :
- Le port vise la **parité fonctionnelle**, pas l'amélioration : on ne redessine pas l'UI en passant.
- L'app reste **mono-fenêtre**, local-first, sans compte ni télémétrie.
- La clé OpenRouter est **personnelle** et vit dans la base locale, jamais dans le dépôt.

**Points ouverts** : aucun.

---

## §1 Principe directeur

> **Un seul superviseur, plusieurs façades.**

Les process de dev sont des **enfants du process Lynk Dev**. Tout ce qui les pilote — l'UI, le
serveur MCP, l'IA — doit passer par le **même superviseur Rust**, jamais par un second
gestionnaire. C'est ce qui rend le MCP quasi gratuit une fois le Dev Manager porté (§3.2) : il
n'ajoute pas de logique, il **rebranche une surface existante** sur un autre transport.

Corollaire : dans le Lot 1, le superviseur (`src-tauri/src/dev/supervisor.rs`) doit être écrit
comme une **bibliothèque indépendante de Tauri** — pas de `AppHandle` dans sa signature, les
événements sortent par un canal. Sinon le MCP devra le réécrire.

---

## LOT 1 — Port Electron → Tauri/Rust (parité Git + Dev)

**Volume réel, DB Explorer retiré (D8)** : ~2 200 lignes de handlers Node à traduire en Rust
(`git-handlers.ts` 721 + `dev-handlers.ts` 1 488) + ~8 300 lignes de React à transposer
(Git 4 400 + Dev 3 700 + primitives partagées). **Le port a fondu de moitié.**
Le React se **transpose** (React 19 des deux côtés) ; le Rust se **réécrit**.

### 1.0 — Socle (préalable à tout)

- [x] ~~**Corriger l'endpoint updater**~~ — **résolu le 2026-09-05 par le renommage des dépôts**,
      sans toucher au fichier. `src-tauri/tauri.conf.json:55` pointe sur `moha528/lynk-dev` : ce nom
      désignait le dépôt **Electron**, il désigne désormais **ce dépôt**. L'URL est devenue juste.
      ⚠️ Ne pas la « corriger » en `lynk-dev-tauri` : ce dépôt n'existe plus sous ce nom.
- [x] Sidebar : entrée `db` retirée, Git + Dev actifs et cliquables, état actif stylé —
      `src/components/Sidebar.tsx` réécrit ; registre des modules dans `src/lib/modules.ts`
- [x] Module actif **persisté** — clé `active_module` du store de réglages
      (`src/stores/useSettingsStore.ts`). ⚠️ La valeur relue est **validée** par `isModuleId` :
      une base écrite par un ancien build contient encore `"db"`, qui afficherait une zone vide.
- [x] Arborescence front : `src/modules/{git,dev}/` + coquilles de vue, montées dans
      `MainLayout` derrière un `ErrorBoundary` **keyé sur le module** (un plantage d'un module ne
      doit pas emporter la fenêtre, et changer de module doit repartir propre)
- [x] Primitives UI : `Badge` (cva, variantes d'état) + `ErrorBoundary`. `confirm-dialog` **non
      porté** : `AlertDialog` et `PromptDialog` existaient déjà et couvrent le besoin.
- [x] Helper Rust `process::run` / `run_raw` — `src-tauri/src/process.rs`, avec ses tests.
      Normalise CRLF→LF, retire le saut final, remonte `stderr` en erreur, `kill_on_drop` au
      dépassement du délai, et **`CREATE_NO_WINDOW` sur Windows** (sans quoi chaque appel `git`
      fait clignoter une console — un scan de dépôts en déclenche des dizaines par seconde).
- [x] Arborescence Rust `src-tauri/src/{git,dev}/` + `commands/{git,dev}.rs` — livrée avec le
      contenu des lots 1.2 et 1.1, sans squelette vide entre-temps

### 1.1 — Git Manager

**Back Rust** — traduit de `lynk-dev-electron/electron/git-handlers.ts` (721 l.) :

- [x] `git/parse.rs` — **les analyseurs de sortie `git`, purs et testés** (14 tests) : `status
      --porcelain` (index / arbre de travail / non suivis / conflits / renommages), `rev-list
      --left-right --count`, listes de branches, `stash list`, `log`, `for-each-ref` du suivi.
      C'était la partie fragile du port : elle est isolée et vérifiable sans dépôt.
- [x] `git/repo.rs` — 35 opérations, via le **binaire `git`** (D4). 16 tests d'intégration sur un
      dépôt jetable : indexation, validation, branches, remisage, distants, diff, et le chemin qui
      compte — **un conflit de fusion rendu comme résultat, pas comme erreur**.
- [x] `git/scan.rs` — recherche de dépôts, profondeur ≤ 2, sans descendre dans un dépôt trouvé
      (sinon chaque sous-module remonterait comme dépôt de premier rang)
- [x] `git/shell.rs` — ouverture d'un terminal ; les **13 candidats Linux dans l'ordre** repris tels
      quels. Une implémentation par famille d'OS, pas des `cfg` imbriqués.
- [x] Profils Git **en base** (`migrations/0003_git_profiles.sql` + `store/git_profiles.rs`)
- [x] 40 commandes Tauri (`commands/git.rs`)
- [x] `core.quotePath=false` sur `status` — sans quoi `données/été.txt` arrive échappé en octal
      (petite amélioration sur l'original, sans effet sur le reste du parsing)

**Front** — depuis `lynk-dev-electron/packages/git-manager/` :

- [x] `types.ts` + `ipc.ts` — miroir du contrat, surface IPC unique
- [x] `store.ts` — Zustand **multi-dépôts**, avec une **borne de parallélisme à 4** : lancer douze
      `git` d'un coup sature la machine. Toute mutation est suivie d'un rafraîchissement : l'état
      vient de `git`, jamais d'une supposition de l'écran.
- [x] `GitManagerView` + `GitProfileBar` — même parti pris que le Dev Manager : un seul jeu de
      boutons (Fetch / Pull / Push) dont la portée bascule sur la sélection
- [x] `RepoList` — recherche, filtre « modifiés », sélection multiple, badges avance/retard
- [x] `RepoDetail` — 5 onglets : Modifications, Branches, Historique, Remisages, Réglages
- [x] `ChangesPanel` — sections conflits / indexé / modifié, actions par fichier au survol,
      résolution « nôtre » / « leur », zone de validation
- [x] `DiffViewer` — coloration par nature de ligne. **Sera remplacé au lot 2.2**, pas retouché.
- [x] `BranchPanel`, `HistoryPanel`, `StashPanel`, `RepoSettingsPanel` (distants, identité, suivi)
- [x] `NewGitProfileDialog` — analyser une racine, cocher les dépôts
- [x] Profil actif persisté (`git_profile_id`)
- [x] Compte rendu des opérations groupées **qui nomme les dépôts en échec** — « 2 échecs » sans
      dire lesquels oblige à tout rouvrir un par un

**Écarts assumés par rapport à l'original :**
- Le champ `behind` du résultat de `pull` est supprimé : il valait **toujours 0** côté Electron.
- `push` en échec rend un résultat plutôt qu'une exception, pour qu'une opération groupée
  continue sur les autres dépôts.

### 1.2 — Dev Manager

**Le cœur : le superviseur** (`electron/dev-handlers.ts`, 1 488 l.). À écrire comme une lib pure (cf. §1).

- [x] `dev/supervisor.rs` — table des process gérés, clé `profile_id:service_id`. **Zéro
      dépendance à Tauri** : il émet sur un canal `broadcast`, l'app et le futur MCP s'y abonnent.
- [x] Spawn via shell (`cmd /C` / `sh -c`, comme le `shell: true` d'Electron), **groupe de process
      sur POSIX** (`process_group(0)`) / **jamais** sur Windows, où `CREATE_NO_WINDOW` remplace
- [x] **Kill d'arbre** — `taskkill /T /F` (Windows) / `kill(-pid, SIGTERM)` puis `SIGKILL` après 5 s
      (POSIX, via `libc`). Attente de mort **sans course** grâce à un `watch::channel`.
- [x] Buffer de logs, flush toutes les 100 ms (`dev/supervisor.rs`, fn `pump`)
- [x] Sondes de démarrage : port TCP, URL de santé (2xx/3xx = sain), `docker compose ps
      --format json` **avec repli texte ET support du format tableau** des Docker récents
- [x] Auto-restart **borné** : 5 tentatives, backoff 2s→30s, marquage `stuck`
- [x] Libération de port : attente puis `kill_by_port` — `netstat` + `taskkill` (Windows),
      `lsof` → `ss` → `fuser` (POSIX, dans cet ordre : Alpine n'a pas `lsof`). Les analyseurs de
      sortie sont **purs et testés** (`dev/net.rs`).
- [x] Tri topologique en couches, **tolérant aux cycles** (`dev/topo.rs`, 8 tests)
- [x] Scan/détection de services (`dev/scan.rs`) : Spring Maven/Gradle, Node, Python,
      docker-compose ; extraction du port depuis `application*.yml|properties` ; profondeur ≤ 2,
      timeout 60 s, exclusions `node_modules|target|build|dist|__pycache__`
- [x] Opérations groupées (`dev/batch.rs`) — couche par couche, annonce `waiting` d'emblée,
      3 s entre couches, et `wait_for_port` au redémarrage groupé
- [x] 18 commandes Tauri (`commands/dev.rs`) : profils, scan/detect, start/stop/restart/build,
      les 3 variantes groupées, sondes de port, santé Docker, probe, liste des process
- [x] Événements conservés : `dev:service:log`, `dev:service:status`, `dev:scan:progress`
- [x] Profils Dev **en base** (`migrations/0002_dev_profiles.sql` + `store/dev_profiles.rs`) —
      plus de fichier JSON dans le dossier utilisateur. Un JSON de services corrompu vide la liste
      mais **ne fait pas disparaître le profil**.
- [x] Arrêt de tous les services **à la fermeture de l'app** (`RunEvent::Exit`) — sinon ils
      survivent à la fenêtre et gardent leurs ports
- [ ] Import/export de profil en JSON (`:1008-1052`) — pas encore porté
- [ ] `runtime:load` / `runtime:save` (`:1444-1451`) — l'ancienne reprise d'état ; à reprendre
      seulement si le front porté en a besoin, `dev_process_list` couvre déjà la réconciliation
- [ ] ⚠️ **Chemin POSIX non vérifié** : développé et compilé sous Windows uniquement. Le
      `process_group(0)` et les `libc::kill` ne seront validés que par la CI (lot 2.1) ou un
      passage sur Linux/macOS.

**Front** — depuis `lynk-dev-electron/packages/dev-manager/` :

- [x] `src/modules/dev/types.ts` — miroir **champ pour champ** de `dev/types.rs`. Une divergence
      ne casse pas la compilation, elle produit des `undefined` silencieux : d'où les tests de
      round-trip camelCase côté Rust.
- [x] `src/modules/dev/ipc.ts` — un seul endroit connaît les noms de commandes et d'événements.
      ⚠️ `listen` est **asynchrone** : le désabonnement gère le cas d'un démontage survenu avant
      la résolution de la promesse, sinon l'abonnement fuit.
- [x] `src/modules/dev/store.ts` — Zustand ; réconciliation au chargement (`processList` + `probe`,
      sans écraser l'état des services que nous gérons) ; logs **plafonnés à 5 000 lignes par
      service** (sans plafond, un `mvn` en boucle de redémarrage sature la mémoire de la fenêtre)
- [x] `DevManagerView` — orchestrateur : barre de pilotage + liste + détail
- [x] `ProfileBar` — profil, compteurs d'état, actions groupées. **Un seul jeu de boutons**, dont la
      portée bascule sur la sélection dès qu'il y en a une. Remplace `StatsBar` + `GroupActions` +
      `ProfileSelector`, trois barres qui se ressemblaient.
- [x] `ServiceList` — recherche, filtre d'état, **groupes repliables**, **sélection multiple**
      (ce qui manquait le plus : sur douze microservices, agir sur un sous-ensemble se faisait un
      service à la fois)
- [x] `ServiceDetail` — en-tête d'état + actions, onglets Logs / Configuration / Environnement,
      bandeaux dédiés pour `stuck`, `waiting` et `external`
- [x] `LogPanel` — filtre par flux, recherche surlignée, **suivi automatique qui se coupe dès qu'on
      remonte** (sinon lire un log pendant qu'un service démarre est impossible), copie, effacement
- [x] `NewProfileDialog` — choisir une racine, l'analyser, cocher ce qu'on garde. Une seule fenêtre
      au lieu de l'assistant en trois étapes : il n'y a que deux décisions à prendre.
- [x] `StatusDot` — pastille d'état, pulsée sur les états transitoires
- [x] Profil actif **persisté** (`dev_profile_id`) : l'écran rouvre là où on l'a laissé
- [x] **Édition d'un service** — `ServiceEditorDialog` : une seule fenêtre pour créer **et**
      modifier (l'original avait deux assistants distincts pour les mêmes champs). Commande, build,
      port, URL de santé, groupe, dépendances (cases à cocher sur les autres services), variables
      d'environnement, redémarrage auto. Ajout depuis la liste, modification et retrait depuis
      l'onglet Configuration.
      ⚠️ **Piège trouvé en l'écrivant** : passer par `saveProfile` aurait rappelé `selectProfile`,
      qui **reconstruit les runtimes à neuf** — modifier une commande aurait donc effacé les logs
      et l'état des services en marche. D'où `saveService` / `removeService`, qui modifient la
      table **en place**.

### 1.3 — DB Explorer — ❌ HORS PÉRIMÈTRE (D8, 2026-09-05)

Le module **n'est pas porté**. Concrètement : ne pas créer `src-tauri/src/db/`, ne pas ajouter
MySQL à `sqlx` (la dépendance reste, mais **pour la base de réglages uniquement** —
`src-tauri/Cargo.toml:29`), ne pas transposer les 22 composants de
`lynk-dev-electron/packages/db-explorer/`, ne pas porter `packages/shared/src/types/db-explorer.ts`.

Rien à supprimer ni à archiver : le code Electron reste lisible dans son dépôt si le besoin revient.

⛔ **Ne pas « re-proposer » ce module dans un futur passage** : c'est une décision prise, pas un oubli.

### 1.4 — Vérification (la porte de sortie du Lot 1)

- [x] `cargo fmt --all -- --check` — vert
- [x] `cargo clippy --all-targets --all-features -- -D warnings` — vert
- [x] `cargo test --all-features --no-fail-fast` — **100 tests**, 0 échec
- [x] `pnpm lint` (Biome) puis `pnpm build` — verts (4 avertissements pré-existants, non bloquants)
- [ ] `pnpm tauri build` — l'installeur se produit *(jamais lancé)*
- [x] **Application lancée** le 2026-09-05 (`pnpm tauri dev`, compilation 1 min 29). Vérifié :
      fenêtre ouverte, Vite en 200, **les 3 migrations appliquées en `success` sur la base
      existante** (créée par le template le 2026-05-27 — la réconciliation de checksums a tenu),
      schémas `dev_profiles` / `git_profiles` créés, aller-retour IPC prouvé (le front a écrit
      `git_profile_id` jusque dans SQLite), aucune panique, arrêt en code 0.
      ⚠️ Le seul `ERROR` du log est **attendu** : l'endpoint updater renvoie 404, `moha528/lynk-dev`
      n'ayant aucune release. Le front l'avale en silence, comme prévu.
- [x] **Scan Git sur un cas réel** — `zeitune/back` : **14 dépôts** trouvés, aucun oubli.
- [x] **Scan Dev sur un cas réel** — 14 services, et **les 11 ports Spring extraits des
      `application*.yml` correspondent exactement** à la table de `zeitune/CLAUDE.md`
      (8010, 8020, 8030, 8060, 8070, 8080, 8081, 8082, 8091, 8100, 8110).
- [x] **Deux défauts de détection trouvés par la recette, et corrigés :**
      1. `olive_ocr_service` **totalement invisible** — service Python sans `pyproject.toml` ni
         `manage.py`, seulement un `requirements.txt`. Désormais reconnu, avec le point d'entrée
         ASGI déduit (`app/main.py` → `app.main:app`) et **le port lu dans le `Dockerfile`**
         (`--port 8120`, puis `EXPOSE` en repli).
      2. `olive_common` proposé comme service alors que c'est une **bibliothèque**
         (`packaging=jar`, aucun `@SpringBootApplication`) : la commande `spring-boot:run`
         n'aurait jamais démarré. On exige maintenant le **`spring-boot-maven-plugin`**, seul
         marqueur d'une application — `olive_core` l'a, `olive_common` non.
      *(`olive_elt` vu comme `docker-compose` n'est **pas** un faux positif : il a un vrai
      `docker-compose.yml` et se lance ainsi.)*
- [x] Aucun orphelin ni port occupé après la fermeture de l'application.
- [ ] **Reste à éprouver à la main** (rien dans les traces ne dit que ça a été fait) :
  - [ ] Git : indexer, valider, lire un diff, changer de branche, remiser, résoudre un conflit
  - [ ] Dev : démarrer un service Spring, lire ses logs, l'arrêter → **vérifier qu'aucun
        `java.exe` ne survit** (`Get-Process java`)
  - [ ] Dev : tuer un service à la main → l'auto-restart reprend puis marque `stuck` après 5 essais
- [ ] Mettre à jour ce fichier : cocher, et noter ce qui a divergé du comportement Electron

---

## Après le port — élargissements et corrections

### Catalogue de détection : 6 → 22 familles

Le catalogue hérité d'Electron ne couvrait que Spring, Node, Python et compose.
`src-tauri/src/dev/detect.rs` (isolé du parcours, dans `scan.rs`) reconnaît désormais :

| Écosystème | Familles |
|---|---|
| JVM | Spring Maven, Spring Gradle |
| JavaScript | **Next, Nuxt, Angular, Nest, SvelteKit, Astro, Remix, Vite**, Node |
| Python | **Django, FastAPI, Flask**, Python |
| Autres | **Go, Rust, .NET, Laravel, Rails** |
| Conteneurs | docker-compose |

Ce que ça apporte concrètement :

- **Le gestionnaire de paquets est déduit du fichier de verrou** — `pnpm dev`, `yarn dev`,
  `bun run dev` ou `npm run dev`. Lancer `npm` dans un projet pnpm réinstalle un `node_modules`
  concurrent et casse les liens : ce n'est pas un détail cosmétique.
- **Le port vient de la source la plus fiable disponible**, dans l'ordre : le `--port` du script,
  puis `.env`, puis le défaut du cadre (3000 pour Next, 4200 pour Angular, 5173 pour Vite…).
- **Une application Tauri est lancée par son orchestrateur** (`pnpm tauri dev`) : `vite` seul
  servirait la page sans jamais ouvrir la fenêtre.
- ⛔ **Rien n'est proposé qui ne démarrerait pas.** Un `package.json` sans script lançable, un
  espace de travail Cargo sans binaire, un dossier Python sans point d'entrée : rien n'est détecté.
  Mieux vaut ne rien voir qu'offrir une commande qui échoue.

### Défaut de portabilité trouvé par la CI

Le premier passage de la CI a fait tomber `probe_reports_a_stopped_service_as_undetected` sur
Linux et macOS — et derrière le test, un vrai défaut :

> `is_port_available` définissait « libre » par « je peux m'y mettre en écoute ». Sous Unix, un
> port **< 1024 est privilégié** : un process ordinaire ne peut jamais s'y lier. **Tout service
> configuré sur le port 80 ou 443 aurait donc été signalé « déjà utilisé » en permanence**, et
> n'aurait jamais démarré.

Corrigé : en cas d'échec pour cause de **droits** — et dans ce cas seulement — on retombe sur la
seule question à laquelle on peut répondre, « quelqu'un accepte-t-il une connexion ? ».
La version Electron avait exactement le même défaut.

### Selects natifs remplacés

`src/components/ui/Select.tsx` (Radix) remplace les trois `<select>` natifs. Sous Windows ils
s'affichaient avec le chrome du système, ignoraient le thème, et devenaient illisibles au-delà
d'une poignée d'entrées — avec 22 familles, c'était intenable. Les familles sont **groupées par
écosystème** dans l'éditeur de service.

### Ergonomie de sélection

- **Maj+clic** coche toute la plage depuis la dernière case cochée, dans les deux listes. L'ancre
  ne bouge pas pendant un Maj+clic : on peut étendre la plage plusieurs fois depuis le même point.
- **Navigation au clavier** : ↑/↓ déplacent la sélection, Espace coche, Ctrl+A coche tout ce qui
  est visible.
- ⚠️ La plage suit **l'ordre à l'écran**, pas celui des données : une liste filtrée ou repliée n'a
  pas le même ordre, et cocher des lignes invisibles serait pire que ne rien faire.
- Une ligne cochée se distingue d'une ligne simplement survolée.

⚠️ **Reste à revoir côté UI** (signalé par le user) : la revue d'ensemble de l'ergonomie au-delà
de la sélection.

---

## LOT 2 — Distribution + lisibilité

### 2.1 — Mises à jour automatiques ⚡ *indépendant : réalisable dès maintenant, sans attendre le Lot 1*

- [x] `.github/workflows/ci.yml` — front lint + build ; Rust fmt/clippy/check/test sur les 3 OS
- [x] `.github/workflows/release.yml` — tag `v*.*.*` → `tauri-action`, release en brouillon,
      matrice macOS universel / Ubuntu 22.04 / Windows
- [x] `createUpdaterArtifacts: true` — `src-tauri/tauri.conf.json:31`
- [x] `CHANGELOG.md`, avec le rappel que la version vit dans **deux** fichiers qui doivent bouger
      ensemble (`package.json` et `tauri.conf.json`) — c'est celle de `tauri.conf.json` que
      l'updater compare
- [x] **Clé de signature posée** (2026-09-05). `pubkey` renseignée dans `tauri.conf.json` ;
      secrets `TAURI_SIGNING_PRIVATE_KEY` et `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` en place.
      La clé privée vit dans `~/.tauri/lynk-dev.key`, **hors du dépôt**, avec son mot de passe
      à côté (`~/.tauri/lynk-dev.password`) pour pouvoir signer en local.
      ⚠️ Le CLI **imprime la clé privée sur la sortie standard** : la génération a été redirigée
      vers un fichier temporaire, supprimé aussitôt. Ne jamais la relancer sans cette précaution.
- [x] Actions GitHub obsolètes relevées : `checkout@v4` et `setup-node@v4` (Node 20 déprécié)
      passées en `v5`.
- [x] ~~**Décider du nom des dépôts**~~ — **fait le 2026-09-05**, avant toute release : l'Electron
      est devenu `moha528/lynk-dev-electron`, puis ce dépôt a pris le nom **`moha528/lynk-dev`**.
      Le dépôt Tauri n'avait **aucune release** → aucune URL publiée cassée. L'Electron conserve la
      sienne (`v0.1.0`), désormais servie sous `lynk-dev-electron`.
      ⚠️ **Reste ouvert** : l'Electron n'est **pas archivé**. L'archiver le passerait en lecture
      seule — sans gêner son rôle de référence pour le port, qui est purement de la lecture.
- [ ] Recette : tagger `v0.1.1`, laisser la CI produire la release, installer `v0.1.0` à la main,
      la lancer → le toast doit apparaître et l'installation aboutir

### 2.2 — Viewer + coloration syntaxique

- [x] `shiki` ajouté, **surligneur unique et partagé** dans `src/lib/highlight.ts`. Moteur et
      thèmes chargés une seule fois ; **les grammaires à la demande** : le build produit un chunk
      par langage (34 au total), rien n'est téléchargé tant qu'on n'ouvre pas un fichier de ce type.
- [x] Les 7 thèmes de l'app mappés sur ceux de Shiki — c'est ce qui a fait retenir Shiki : aucune
      palette à réinventer, un diff prend exactement les couleurs de la fenêtre, et changer de
      thème repeint le viewer.
- [x] `CodeView` — langage déduit du chemin (extension **ou** nom de fichier : `Dockerfile`,
      `.env`, `Cargo.toml`), numéros de ligne, repli sur le texte brut si la grammaire manque.
      ⚠️ Au-delà de 2 000 lignes, les suivantes ne sont **pas rendues** et un pied de bloc le dit :
      c'est un plafond, **pas** de la virtualisation.
- [x] `DiffView` — **coloration du langage à l'intérieur du diff**. La grammaire `diff` de Shiki
      colorerait les `+` et les `-` en laissant le code en gris ; on fait l'inverse : marqueurs
      retirés, contenu coloré dans son vrai langage d'un seul tenant (la grammaire a besoin du
      contexte des lignes voisines), puis marqueur et fond reposés par-dessus.
- [x] `LogView` — **séquences ANSI interprétées** et ramenées sur la palette de l'app, niveaux
      `ERROR|WARN|INFO` mis en évidence, `stderr` distingué, recherche surlignée sur le texte
      démarqué, copie sans codes d'échappement, suivi automatique.
- [x] Branchements : diff du Git Manager, logs du Dev Manager. Les rendus maison sont supprimés.
- [x] Poids vérifié : le chunk principal reste à **640 ko** (198 ko gzip) ; le moteur WebAssembly
      (622 ko) et chaque grammaire sont des chunks séparés, chargés seulement à l'usage.
- [ ] Brancher `CodeView` sur un vrai viewer de fichier (Git n'expose pas encore la consultation
      d'un fichier hors diff) et sur les `.env` / YAML de configuration du Dev Manager.

---

## LOT 3 — L'IA

### 3.1 — OpenRouter

- [x] Client Rust `ai/openrouter.rs` — `reqwest`, en-têtes `HTTP-Referer` / `X-Title`, erreurs
      d'OpenRouter remontées telles quelles (elles sont explicites, y compris sur un 200).
- [x] Consignes isolées et **pures** dans `ai/prompts.rs`, avec leurs tests : c'est la partie
      qu'on relira le plus, elle doit être vérifiable sans appel réseau.
      L'entrée est **bornée à 24 000 caractères**, et la coupure est annoncée au modèle pour qu'il
      n'affirme rien sur ce qu'il ne voit pas. ⚠️ La troncature respecte les frontières de
      caractères — couper au milieu d'un « é » ferait paniquer l'indexation.
- [x] Réglages : section « IA » (clé + modèle). **La clé ne se relit jamais** : l'écran sait
      qu'elle existe, peut la remplacer ou l'effacer, pas la voir.
- [x] Catalogue **chargé en direct** (`GET /api/v1/models`), trié du moins cher au plus cher, tarif
      affiché au million de jetons. La clé saisie peut être éprouvée **avant** d'être enregistrée.
- [x] Fonction 1 — **message de commit** depuis `git diff --cached`. L'index fait foi, pas la
      sélection de l'écran : c'est exactement ce qui sera validé. Message **éditable** avant
      validation, jamais de commit automatique.
- [x] Fonction 2 — **explication d'un diff**, depuis l'en-tête du panneau de diff.
- [x] Fonction 3 — **analyse d'une sortie de service**, sur les 400 dernières lignes **visibles**
      (filtres compris) : analyser des lignes masquées produirait une réponse hors sujet.
- [x] Garde-fous : rien n'est envoyé sans un geste explicite ; sans clé ou sans modèle, l'erreur
      dit lequel manque ; jetons de sortie plafonnés par usage (400 / 500 / 700).
- [x] Le modèle n'est **jamais** figé dans le code — le catalogue et les tarifs bougent tous les
      mois. L'application charge la liste, l'utilisateur choisit.
- [ ] ⚠️ **La clé est stockée en clair** dans la base locale (`settings`), et l'écran le dit.
      Le trousseau du système serait meilleur, mais ajoute une dépendance par plateforme et un
      mode de panne (pas de service de secrets sur une session Linux sans bureau) pour une
      fonction optionnelle. **Décision à revoir** si l'app doit porter d'autres secrets.
- [ ] Indicateur de coût cumulé de la session — `estimateCost` existe côté front, rien ne l'affiche.

### 3.2 — Serveur MCP

**La fourche, tranchée (D6).** Un binaire MCP lancé par Claude Code est un process séparé : il ne
voit pas les enfants de Lynk Dev. Deux façons de garder un superviseur unique :

- **Retenu — l'app héberge un serveur MCP HTTP** sur `127.0.0.1:<port>`, jeton dans le répertoire
  de données. Le client IA pointe dessus. Zéro process supplémentaire.
- *Écarté (mais compatible)* — un binaire `lynk-mcp` en stdio qui relaie vers ce même serveur.
  À n'ajouter que si un client IA ne sait pas parler HTTP.

- [ ] Serveur MCP dans `src-tauri/src/mcp/`, démarré avec l'app, port + jeton persistés
- [ ] Réglages : activer/désactiver, port, **régénérer le jeton**, bouton « copier la config client »
- [ ] Outils **lecture** : `list_services` (nom, type, statut, PID, port, **démarré depuis**),
      `get_service_logs` (n dernières lignes, filtre par flux), `check_port`, `get_service_health`
- [ ] Outils **écriture** : `start_service`, `stop_service`, `restart_service`, `build_service`
- [ ] Chaque outil = un appel au superviseur du Lot 1.2. **Aucune logique métier dans le MCP.**
- [ ] Garde-fous : loopback strict, jeton obligatoire, écriture limitée aux services du profil
      actif, **aucun outil d'exécution de commande arbitraire** (pas de `run_command` : ce serait
      un shell distant déguisé)
- [ ] Journal des appels MCP visible dans l'app (qui a redémarré quoi, et quand)
- [ ] Recette : brancher Claude Code dessus, lui demander « quels services tournent, depuis
      quand ? », puis « redémarre celui qui est en erreur »

---

## Pièges nommés (ce qu'un implémenteur pressé va rater)

1. **Le kill d'arbre.** `child.kill()` ne tue que le shell : `mvnw` → `mvn` → `java` survit, le port
   reste pris, et le service suivant échoue au démarrage. Il faut, **exactement comme l'Electron**
   (`electron/dev-handlers.ts:352-406`) : Windows → `taskkill /pid N /T /F` ; POSIX → spawn
   `detached` (le fils devient chef de groupe) puis `kill(-pid, SIGTERM)`, et `SIGKILL` après 5 s.
   En Rust, `Command::kill()` a le même défaut que Node : il faut passer par le groupe de process.
2. **`detached` sur Windows ouvre une console.** D'où la condition `platform !== 'win32'`
   (`:195`) — ne pas « uniformiser » les deux branches.
3. **Le superviseur ne doit pas dépendre de Tauri.** S'il prend un `AppHandle`, le serveur MCP
   devra le réécrire. Il émet sur un canal ; l'app *et* le MCP s'y abonnent.
4. **Les logs sans buffer noient l'IPC.** Le flush groupé toutes les 100 ms (`:224-246`) n'est pas
   une optimisation prématurée : un `mvn` en démarrage crache des milliers de lignes.
5. **Le repli de `docker compose ps`.** `--format json` n'existe pas sur les vieux Docker : le repli
   texte (`:660-676`) est obligatoire, pas décoratif.
6. **L'ordre `lsof` → `ss` → `fuser`** (`:437-470`) : les images minimales n'ont pas `lsof`. Ne pas
   réduire à un seul outil.
7. **Le tri topologique doit tolérer les cycles** (`:906-915`) : en cas de cycle il rend le reste en
   une couche au lieu de boucler. Un profil mal configuré ne doit pas figer l'app.
8. **Les checksums de migration sqlx** : `store/db.rs:56` réaligne les checksums avant de migrer,
   parce qu'ils dépendent des fins de ligne du build. **Ne jamais changer le sens d'une migration
   publiée** — seulement en ajouter.
9. **L'endpoint updater doit être figé avant la première release.** Après, les clients installés
   interrogent l'ancienne URL et ne verront plus jamais de mise à jour (cf. §2.1).
10. **Écrire en LF.** Un fichier réécrit en CRLF fabrique un diff total et casse les checksums de migration.
11. **Ne pas deviner ce que contient un `*:default` Tauri.** Le contenu réel de chaque jeu de
    permissions est listé dans `src-tauri/gen/schemas/acl-manifests.json` — et une permission
    manquante ne casse pas la compilation, elle fait échouer l'appel **à l'exécution**.
    Vérifié le 2026-09-05 : `dialog:default` = `allow-message`, `allow-save`, **`allow-open`** ;
    `opener:default` = `allow-open-url`, **`allow-reveal-item-in-dir`**, `allow-default-urls`.
    Le sélecteur de dossier et l'ouverture de l'explorateur passent donc sans rien ajouter.

---

## Ordre d'exécution

1. **Lot 1.0** (socle) — dont la correction d'endpoint, qui est un vrai bug
2. **Lot 1.2** (Dev Manager) avant 1.1 : c'est lui qui porte le principe directeur et il conditionne le Lot 3.2
3. **Lot 1.1** (Git Manager) — conditionne le Lot 3.1
4. **Lot 1.4** (vérification) sur Git + Dev
5. **Lot 1.3** (DB Explorer) — ou décision explicite de le repousser
6. **Lot 2.1** (release) — *déplaçable n'importe quand, y compris tout de suite*
7. **Lot 2.2** (viewer), puis **3.1** (OpenRouter), puis **3.2** (MCP)

## Commandes de fin de tâche

```bash
cd src-tauri && cargo fmt --all && cargo clippy --all-targets --all-features -- -D warnings && cargo test
cd .. && pnpm lint && pnpm build
```
