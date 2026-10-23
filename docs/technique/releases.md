# Releases

Chaque évolution fusionnée sur `main` alimente une PR de release ; la fusionner crée le tag semver, le changelog, la release GitHub et les installeurs des trois systèmes.

## Le cycle

1. Une PR est fusionnée sur `main` (en squash, seul mode permis), avec un titre au format Conventional Commits.
2. Le workflow `.github/workflows/release.yml` lance `release-please` (`googleapis/release-please-action@v5`). Il ouvre, ou met à jour, une PR de release : titre `chore(main): release X.Y.Z`, label `autorelease: pending`, nouvelle version dans `package.json`, `Cargo.toml` et `.release-please-manifest.json`, entrée ajoutée à `CHANGELOG.md`.
3. Un second job remet `Cargo.lock` d'accord (`cargo update --workspace`) et pousse sur la branche de la PR de release : le verrou cite la version de chaque crate du workspace.
4. `make release` (ou `pnpm release`) fusionne cette PR en squash, sous son propre titre et sans rien ajouter au message (`scripts/merge-release.mjs`). Sans PR de release ouverte, la commande le dit et ne fait rien ; avec deux, elle refuse de choisir ; tant que `Cargo.lock` n'est pas sur la PR, elle demande de réessayer un peu plus tard.
5. Sur ce push, `release-please` crée le tag `vX.Y.Z` et la release GitHub, avec la partie du changelog qui la concerne. Le job « Installeurs » construit alors l'app sur Ubuntu 22.04, macOS (Apple Silicon) et Windows (`pnpm tauri build`) avec un jeton en lecture seule et sans garder d'identifiants Git (`persist-credentials: false`) : les scripts de build de toutes les dépendances tournent dans ce job. Les installeurs passent en artefacts ; le job « Publication des installeurs », seul à pouvoir écrire, les dépose sur la release (`gh release upload`) sans rien construire ni installer.

## Versions

- Règles semver de `release-please` : `fix` (et `perf`, `revert`) donne un correctif (`0.1.0` → `0.1.1`), `feat` une mineure (`0.2.0`), `!` ou `BREAKING CHANGE` une majeure (`1.0.0`, y compris avant 1.0). Les autres types (`chore`, `docs`, `refactor`, `test`, `ci`…) ne déclenchent pas de release seuls.
- Départ : la première release, la `0.9.0`, a été publiée à la main le 22 octobre 2026, la CI étant alors bloquée par la facturation GitHub. Les fichiers de version et le manifeste ont été portés à `0.9.0`, le tag `v0.9.0` posé sur ce commit et `CHANGELOG.md` écrit à la main : `release-please` repart de là. Les installeurs de cette version n'ont pas été construits par la CI.
- Une seule version pour l'app, à trois endroits : `.release-please-manifest.json`, `package.json` et `[workspace.package]` de `Cargo.toml` (mis à jour par chemin TOML, `$.workspace.package.version`), dont héritent toutes les crates. La configuration Tauri ne porte pas de version : elle prend celle de la crate `app`. Le test `tests/versions.spec.ts` échoue si ces versions divergent ou si `tauri.conf.json` reçoit sa propre version. La version citée en tête du README est mise à jour par la même occasion (bloc `x-release-please-start-version`).

## Installeurs

- Non signés, pour un usage personnel. macOS : au premier lancement, ouvrir l'app par un clic droit puis « Ouvrir » (ou l'autoriser dans Réglages Système, Confidentialité et sécurité). Windows : SmartScreen avertit ; « Informations complémentaires » puis « Exécuter quand même ».
- Formats : ceux de `"targets": "all"` dans `tauri.conf.json` (`.dmg` sur macOS, `.msi` et `.exe` sur Windows, `.deb`, `.rpm` et `.AppImage` sur Linux), tels que `tauri build` les produit dans `target/release/bundle/`.
- macOS Intel n'est pas construit ; l'ajouter demande une ligne de matrice avec `--target x86_64-apple-darwin`.

## Prérequis côté GitHub

- Chaque job ne reçoit que les droits qu'il utilise : contenu, issues et PR pour `release-please`, contenu en écriture pour `Cargo.lock` et la publication des installeurs, lecture seule pour leur construction. Toutes les actions sont épinglées par SHA de commit, la version en commentaire ; Dependabot (`.github/dependabot.yml`) propose chaque semaine leurs mises à jour. Les workflows sans écriture déclarent `permissions: contents: read`. Les runs se suivent sans s'annuler (`concurrency`), pour ne jamais couper une construction d'installeurs.
- Réglage du dépôt « Allow GitHub Actions to create and approve pull requests » (Settings, Actions, General), activé : sans lui, `release-please` ne pourrait pas ouvrir la PR de release. Les droits par défaut du jeton des workflows restent en lecture seule ; chaque job demande ce qu'il lui faut.
- La PR de release est créée avec le jeton du workflow : GitHub ne lance pas la CI sur elle. Elle ne change que des versions et le changelog, vérifiés par le test des versions au push suivant sur `main`.
