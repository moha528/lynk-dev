# Changelog

Le format suit [Keep a Changelog](https://keepachangelog.com/fr/1.1.0/) et le
versionnage [SemVer](https://semver.org/lang/fr/).

> ⚠️ Le numéro de version vit à **deux endroits** qui doivent bouger ensemble :
> `package.json` et `src-tauri/tauri.conf.json`. C'est celui de `tauri.conf.json`
> que l'updater compare — les désynchroniser fait proposer une mise à jour qui
> ne s'applique jamais.

## [Non publié]

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

### Corrigé

- Un service configuré sur un **port privilégié** (< 1024) était signalé « déjà utilisé » en
  permanence sous Unix, et ne démarrait donc jamais.

### Notes

- Le **DB Explorer** de la version Electron n'est pas porté (décision du
  2026-09-05) : l'application est Git + Dev.
- Les chemins POSIX du superviseur (groupe de process, `kill(-pid)`) **compilent**
  sous Linux et macOS depuis le premier passage de la CI, mais leur comportement
  à l'exécution n'a encore été éprouvé sur aucune de ces deux plateformes.
