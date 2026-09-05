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

### Notes

- Le **DB Explorer** de la version Electron n'est pas porté (décision du
  2026-09-05) : l'application est Git + Dev.
- Les chemins POSIX du superviseur (groupe de process, `kill(-pid)`) sont
  écrits mais **non éprouvés** : le développement s'est fait sous Windows.
