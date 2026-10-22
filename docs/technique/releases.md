# Releases

Chaque évolution fusionnée sur `main` alimente une PR de release ; la fusionner crée le tag semver, le changelog, la release GitHub et les installeurs des trois systèmes.

## Le cycle

1. Une PR est fusionnée sur `main` (en squash, seul mode permis), avec un titre au format Conventional Commits.
2. Le workflow `.github/workflows/release.yml` lance `release-please` (`googleapis/release-please-action@v5`). Il ouvre, ou met à jour, une PR de release : titre `chore(main): release X.Y.Z`, label `autorelease: pending`, nouvelle version dans `package.json`, `Cargo.toml` et `.release-please-manifest.json`, entrée ajoutée à `CHANGELOG.md`.
3. Un second job remet `Cargo.lock` d'accord (`cargo update --workspace`) et pousse sur la branche de la PR de release : le verrou cite la version de chaque crate du workspace.
4. `make release` (ou `pnpm release`) fusionne cette PR en squash, sous son propre titre et sans rien ajouter au message (`scripts/merge-release.mjs`). Sans PR de release ouverte, la commande le dit et ne fait rien ; avec deux, elle refuse de choisir.
5. Sur ce push, `release-please` crée le tag `vX.Y.Z` et la release GitHub, avec la partie du changelog qui la concerne. Le job « Installeurs » construit alors l'app sur Ubuntu 22.04, macOS (Apple Silicon) et Windows avec `tauri-apps/tauri-action@v1`, et dépose les installeurs sur cette release.

## Versions

- Règles semver de `release-please` : `fix` donne un correctif (`0.1.0` → `0.1.1`), `feat` une mineure (`0.2.0`), `!` ou `BREAKING CHANGE` une majeure (`1.0.0`, y compris avant 1.0). Les autres types (`chore`, `docs`, `refactor`, `test`, `ci`…) ne déclenchent pas de release seuls.
- Départ : `0.1.0`, inscrite dans `.release-please-manifest.json`. La première PR de release regroupe donc tout l'historique depuis le début du dépôt.
- Une seule version pour l'app, à trois endroits : `.release-please-manifest.json`, `package.json` et `[workspace.package]` de `Cargo.toml` (ligne annotée `x-release-please-version`), dont héritent toutes les crates. La configuration Tauri ne porte pas de version : elle prend celle de la crate `app`. Le test `tests/versions.spec.ts` échoue si ces versions divergent ou si `tauri.conf.json` reçoit sa propre version.

## Installeurs

- Non signés, pour un usage personnel. macOS : au premier lancement, ouvrir l'app par un clic droit puis « Ouvrir » (ou l'autoriser dans Réglages Système, Confidentialité et sécurité). Windows : SmartScreen avertit ; « Informations complémentaires » puis « Exécuter quand même ».
- Formats : ceux de `"targets": "all"` dans `tauri.conf.json` (`.dmg` et `.app` sur macOS, `.msi` et `.exe` sur Windows, `.deb`, `.rpm` et `.AppImage` sur Linux).
- macOS Intel n'est pas construit ; l'ajouter demande une ligne de matrice avec `--target x86_64-apple-darwin`.

## Prérequis côté GitHub

- Le workflow a besoin d'écrire le contenu, les issues et les PR (`permissions` du fichier).
- Réglage du dépôt « Allow GitHub Actions to create and approve pull requests » (Settings, Actions, General) : sans lui, `release-please` ne peut pas ouvrir la PR de release.
- La PR de release est créée avec le jeton du workflow : GitHub ne lance pas la CI sur elle. Elle ne change que des versions et le changelog, vérifiés par le test des versions au push suivant sur `main`.
