# Changelog

Le format suit [Keep a Changelog](https://keepachangelog.com/fr/1.1.0/) et le
versionnage [SemVer](https://semver.org/lang/fr/).

> ⚠️ Le numéro de version vit à **deux endroits** qui doivent bouger ensemble :
> `package.json` et `src-tauri/tauri.conf.json`. C'est celui de `tauri.conf.json`
> que l'updater compare — les désynchroniser fait proposer une mise à jour qui
> ne s'applique jamais.

## [0.1.0] — 2026-09-05

Première version publiée.

### Ajouté

- **Dev Manager** — supervision des services de développement locaux : détection
  automatique (Spring Maven/Gradle, Node, Python, docker-compose), démarrage,
  arrêt, redémarrage, build, logs en direct, sondes de port et de santé,
  redémarrage automatique borné, démarrages groupés ordonnés par dépendances.
- **Git Manager** — pilotage de plusieurs dépôts : état, indexation, validation,
  branches, fusion et résolution de conflits, remisages, historique, distants et
  identité, avec Fetch / Pull / Push groupés.
- Navigation entre modules, persistée d'une session à l'autre.
- Catalogue de détection élargi à 22 familles : Next, Nuxt, Angular, Nest, SvelteKit, Astro,
  Remix, Vite, Django, FastAPI, Flask, Go, Rust, .NET, Laravel, Rails. Le gestionnaire de
  paquets est déduit du fichier de verrou, et le port de la source la plus fiable disponible.

- **Coloration syntaxique** partout où c'est pertinent : diffs du Git Manager (langage coloré
  *à l'intérieur* du diff), logs du Dev Manager (séquences ANSI interprétées, niveaux mis en
  évidence). Les grammaires sont chargées à la demande, une par langage.
- Listes déroulantes propres à l'application, à la place des `<select>` du système.
- **Mises à jour automatiques opérationnelles** : clé de signature en place et chaîne de release
  complète. Il ne manque qu'un tag `vX.Y.Z` pour publier.
- **Serveur MCP** — l'application héberge un serveur sur `127.0.0.1`, derrière un jeton. Quatre
  outils de lecture (`list_services`, `get_service_logs`, `check_port`, `get_service_health`) et
  quatre d'écriture (`start`, `stop`, `restart`, `build`), tous branchés sur le **même**
  superviseur que l'écran. **Aucun outil d'exécution de commande arbitraire** : ce serait un
  shell distant déguisé. Journal des appels visible dans l'application.
- **Assistance par modèle** (OpenRouter) — message de commit rédigé depuis l'index, explication
  d'un diff, analyse d'une sortie de service. Toujours sur un geste explicite. Le catalogue est
  chargé en direct et trié du moins cher au plus cher : aucun identifiant de modèle n'est figé
  dans le code.
- **Les secrets vivent dans le trousseau du système** — Credential Manager, Keychain, Secret
  Service. Une clé restée en clair d'une version antérieure est déplacée au démarrage. Sans
  trousseau utilisable, l'écran affiche la cause et la fonction s'arrête là : **aucun repli en
  clair**.
- Sélection par plage au clavier et à la souris dans les listes (Maj+clic, ↑↓, Espace, Ctrl+A).
- **Politique de sécurité du contenu** (CSP) sur la fenêtre, là où il n'y en avait aucune.

### Corrigé

- Un service configuré sur un **port privilégié** (< 1024) était signalé « déjà utilisé » en
  permanence sous Unix, et ne démarrait donc jamais.
- `git add` était appelé **sans `--`** : un fichier dont le nom commence par `-` était lu comme
  une option et ne pouvait pas être indexé.
- La lecture du contenu d'un fichier pouvait **sortir du dépôt** : joindre un chemin absolu à une
  base la remplace au lieu de s'y ajouter.
- « Ouvrir dans le terminal » cassait sous Windows sur un chemin contenant `&` — un dossier
  nommé `R&D` suffisait.
- L'entrée `xterm` de l'ouverture de terminal n'ouvrait pas le bon dossier et se refermait
  aussitôt.
- « Effacer » dans la vue des logs ne vidait que l'écran : le tampon lu par le serveur MCP
  gardait les lignes.

### Notes

- Le **DB Explorer** de la version Electron n'est pas porté (décision du
  2026-09-05) : l'application est Git + Dev.
- Les chemins POSIX du superviseur (groupe de process, `kill(-pid)`) **compilent**
  sous Linux et macOS depuis le premier passage de la CI, mais leur comportement
  à l'exécution n'a encore été éprouvé sur aucune de ces deux plateformes.
- Le **verrouillage par PIN** est un verrou d'interface, pas un chiffrement : la base locale
  reste en clair et le serveur MCP continue de servir même application verrouillée.
- Aucun client MCP ne s'est encore connecté à l'application lancée : le transport est éprouvé de
  bout en bout par des tests d'intégration HTTP, pas par un client réel.
