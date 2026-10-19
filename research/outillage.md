# Outillage Rust, Vue et CI : comparatif (ticket #10)

Données vérifiées le 2026-10-19 (crates.io, npm, GitHub, docs officielles). Les versions changent vite : à revalider au bootstrap.

## Réponse courte

| Sujet | Recommandation | Confiance |
|---|---|---|
| Types TS depuis Rust | `tauri-specta` v2 (RC) + `specta-typescript` si on accepte une RC ; sinon `ts-rs` 12 (stable) pour les seuls types de données | moyenne |
| SQLite | `rusqlite` (feature `bundled`) + `rusqlite_migration`, accès côté Rust uniquement | haute |
| Lint/format front | ESLint + `eslint-plugin-vue` + Prettier ; Biome seulement pour le script TS, pas pour les `.vue` | haute |
| Typage front | `vue-tsc --noEmit` en mode strict, avec TypeScript 5.x/6 tant que la compatibilité TS 7 n'est pas confirmée | moyenne |
| Tests | Vitest + `@vue/test-utils` (front), `cargo-nextest` (Rust) | haute |
| CI | GitHub Actions, matrice `ubuntu-22.04` / `macos-latest` / `windows-latest`, actions officielles ; `tauri-action@v1` seulement pour la release | haute |

## 1. Génération de types TS depuis Rust

| Option | Version | Maintenance | Compat. Tauri 2 | Coût |
|---|---|---|---|---|
| `tauri-specta` | 2.0.0-rc.25 (2026-05-08) ; 1.0.2 = Tauri 1 seulement | actif (RC depuis janv. 2025, 25 RC) | oui (dépend de `tauri ^2`) ; README : Specta v2 requis | moyen : `Builder` + `collect_commands!`, export des bindings au démarrage en debug |
| `specta` + `specta-typescript` | 2.0.0-rc.25 / 0.0.12 | actif, mais API pré-1.0 | indirecte (sert de base à tauri-specta) | élevé si utilisé seul : on câble soi-même commandes et événements |
| `ts-rs` | 12.0.1 (2026-01-31), ~19 M téléchargements | stable, mainteneurs actifs | agnostique : types seulement, rien sur les commandes | faible : `#[derive(TS)]` + `#[ts(export)]`, génération via `cargo test`, dossier via `TS_RS_EXPORT_DIR` |

Points clés :
- Seul `tauri-specta` génère aussi les signatures typées des `invoke` et des événements. Avec `ts-rs`, les noms de commandes et leurs arguments restent écrits à la main côté TS (une duplication à encadrer par un wrapper unique).
- Risque `tauri-specta` : version RC, la doc docs.rs de rc.25 n'a pas compilé (liens vers rc.21), API susceptible de bouger. Atténuation : épingler exactement (`=2.0.0-rc.25`, `specta` en `=`), et vérifier en CI que les bindings générés sont à jour (diff git).
- `ts-rs` : `i64/u64` deviennent `bigint` par défaut (réglable avec `TS_RS_LARGE_INT`) ; attributs serde supportés, `skip_deserializing` ignoré.
- Dans les deux cas, bloquer en CI : régénérer puis `git diff --exit-code` sur le fichier de bindings.

Sources : https://github.com/specta-rs/tauri-specta , https://docs.rs/tauri-specta/2.0.0-rc.25/tauri_specta/ , https://github.com/Aleph-Alpha/ts-rs , https://crates.io/crates/tauri-specta , https://crates.io/crates/ts-rs

## 2. SQLite

| Option | Version | Points forts | Points faibles |
|---|---|---|---|
| `rusqlite` | 0.40.2 (2026-08-08), ~129 M téléchargements | synchrone, léger, feature `bundled` (SQLite 3.53.2 embarqué, builds identiques sur 3 OS), `Connection::open_in_memory()` pour les tests sans fake | nécessite un compilateur C pour `bundled` (présent sur les runners) ; API bloquante : à isoler dans un thread/`spawn_blocking` |
| `sqlx` | 0.9.0 (2026-05-21), ~168 M | async, requêtes vérifiées à la compilation (macro `query!` demande une base ou un cache hors-ligne) | plus lourd, sur-dimensionné pour un SQLite local mono-utilisateur |
| `tauri-plugin-sql` | 2.6.0 stable ; 3.0.0-alpha.3 (Tauri 3) | officiel, migrations dans le builder | repose sur sqlx, pensé pour un accès **depuis le JS** : requêtes SQL écrites côté front, donc logique métier hors de Rust et hors du typage généré ; permissions à configurer ; incompatible avec l'objectif « types générés depuis Rust » |

Migrations :
| Option | Version | Remarque |
|---|---|---|
| `rusqlite_migration` | 2.6.0 (2026-05-28) | taille minimale, migrations en SQL ou Rust, suit `PRAGMA user_version`, testable en mémoire |
| `refinery` | 0.10.0 (2026-10-02) | plus général multi-SGBD, intégration rusqlite ; plus de surface que nécessaire |
| `sqlx::migrate!` | avec sqlx | seulement si sqlx est retenu |

Recommandation : `rusqlite` + `rusqlite_migration`. Un repository derrière un trait, deux implémentations : SQLite réel (en mémoire dans les tests d'intégration) et fake en mémoire, ce qui respecte la règle « pas de mocks ». Écartez `tauri-plugin-sql` (accès JS direct).

Sources : https://crates.io/crates/rusqlite , https://github.com/rusqlite/rusqlite , https://crates.io/crates/sqlx , https://v2.tauri.app/plugin/sql/ , https://crates.io/crates/rusqlite_migration , https://crates.io/crates/refinery

## 3. Lint, format et typage front

| Option | Version | Vue | Setup |
|---|---|---|---|
| ESLint + `eslint-plugin-vue` + `typescript-eslint` | eslint 10.13.0, eslint-plugin-vue 10.11.1, typescript-eslint 8.71.2 | complet : règles de template, a11y, composition API | moyen (flat config) ; référence de l'écosystème Vue |
| Prettier | 3.9.10 | formatage complet des `.vue` | faible |
| Biome | 2.5.15 (2026-09-30) | **expérimental** pour Vue (depuis 2.3) : parse/format/lint du `<script>` et `<style>`, template formaté avec les règles HTML ; opt-in `html.experimentalFullSupportEnabled` ; support des plugins « non démarré » ; pas de règles spécifiques Vue documentées | très faible (un seul binaire, un seul fichier de config), très rapide |

Typage : `vue-tsc` 3.3.13 (peer `typescript >=5.0.0`). TypeScript 7.0.2 vient de sortir (compilateur natif) ; côté `vuejs/language-tools`, des PR de migration sont fusionnées en juillet-août 2026 mais l'issue « vue-tsc does not work with TypeScript 7 » a été fermée sans résolution visible. Recommandation : épingler TypeScript 5.x/6 jusqu'à confirmation. Activer `"strict": true`, plus `noUncheckedIndexedAccess`, `exactOptionalPropertyTypes` ; lancer `vue-tsc --noEmit` en CI bloquant.

Recommandation : ESLint + Prettier (couverture Vue réelle, bloquants en CI : `eslint --max-warnings 0`, `prettier --check`). Biome reste une option à revisiter quand le support Vue sortira de l'expérimental ; l'utiliser en mélange (Biome pour `.ts`, ESLint pour `.vue`) double la configuration pour un gain faible. Côté Rust : `cargo fmt --check` et `cargo clippy -- -D warnings`.

Sources : https://biomejs.dev/internals/language-support/ , https://biomejs.dev/blog/biome-v2-3/ , https://www.npmjs.com/package/vue-tsc , https://github.com/vuejs/language-tools/issues?q=typescript+7 , https://www.npmjs.com/package/eslint-plugin-vue

## 4. Tests

| Option | Version | Remarque |
|---|---|---|
| Vitest | 5.0.5 (2026-10-15) ; exige Node `^22.12 \|\| ^24 \|\| >=26` | natif Vite, `@vue/test-utils` 2.5.1, injection par `provide` pour les fakes (pas de `vi.mock`, sauf contrats Inertia non concernés ici) |
| `cargo test` | inclus | exécute aussi les doctests ; sert à l'export `ts-rs` |
| `cargo-nextest` | 0.9.148 (2026-10-08) | un processus par test, sortie lisible, retries, JUnit, Linux/macOS/Windows ; ne lance pas les doctests (garder `cargo test --doc` à part). Installation CI via l'action `taiki-e/install-action` (à vérifier au bootstrap) |

Recommandation : Vitest + nextest, plus `cargo test --doc` si des doctests existent. Si `ts-rs` est retenu, l'export passe par `cargo test` (nextest le lance aussi, car ce sont des tests normaux).

Sources : https://www.npmjs.com/package/vitest , https://github.com/nextest-rs/nextest/releases , https://nexte.st

## 5. CI GitHub Actions (macOS, Linux, Windows)

Base : le guide officiel Tauri 2 (https://v2.tauri.app/distribute/pipelines/github/) utilise `actions/checkout@v7`, `actions/setup-node@v6`, `dtolnay/rust-toolchain@stable`, `swatinem/rust-cache@v2`, `tauri-apps/tauri-action@v1` (v1.0.0 publiée le 2026-06-29).

- Matrice : `ubuntu-22.04` (et `ubuntu-22.04-arm` sur dépôt public), `macos-latest` (aarch64 ; ajouter une cible x86_64 pour l'Intel), `windows-latest`, `fail-fast: false`.
- Linux : paquets apt de la page prérequis : `libwebkit2gtk-4.1-dev build-essential curl wget file libxdo-dev libssl-dev libayatana-appindicator3-dev librsvg2-dev` ; le guide CI cite aussi `libappindicator3-dev`, `patchelf`, `xdg-utils`. Aucun paquet supplémentaire pour les tests unitaires Rust hors fenêtre, mais la compilation de `tauri` exige quand même webkit2gtk. Pas d'écran requis tant qu'on ne lance pas l'app.
- Windows : MSVC + WebView2 déjà présents sur les runners ; aucune étape d'installation dans l'exemple officiel.
- macOS : rien à installer ; la signature/notarisation n'est nécessaire qu'à la release (signature ad hoc sinon, selon le guide).
- Découpage : jobs de qualité (fmt, clippy, eslint, prettier, vue-tsc, vitest, nextest, vérif des bindings à jour) sur les 3 OS ; `tauri-action` (build + bundles) réservé à un workflow de release sur tag, car il alourdit chaque PR.
- Coût : cache `rust-cache` indispensable (compilation Tauri longue) ; Windows est le runner le plus lent.

## Risques à porter au ticket de décision

1. `tauri-specta` reste en RC : choisir entre cohérence (commandes typées) et stabilité (`ts-rs`). Épingler et tester la régénération en CI atténue le risque.
2. Support Vue de Biome expérimental, TypeScript 7 et `vue-tsc` non confirmé : figer les versions.
3. `tauri-plugin-sql` et `tauri-specta` ne vont pas ensemble pour la règle « une seule définition de type » : privilégier `rusqlite` côté Rust.
4. Tauri 3 est en alpha (3.0.0-alpha.5) : rester sur Tauri 2.12.x et vérifier que les versions de plugins suivent.
