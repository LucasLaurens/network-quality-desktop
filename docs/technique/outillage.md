# Outillage

## Versions fixées

- Rust : `rust-toolchain.toml` (rustfmt et clippy inclus).
- Node : `.node-version` (24). La CI utilise cette version ; en local, une version plus récente fonctionne avec un avertissement de Vitest.
- pnpm : champ `packageManager` de `package.json`. Les versions des dépendances sont exactes (`save-exact`).
- Tauri 2.12, `tauri-specta` et `specta` en version exacte (versions candidates), TypeScript 6.0.

## Qualité

| Contrôle     | Commande                                                                                                                                                                                 |
| ------------ | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Format Rust  | `cargo fmt --all --check`                                                                                                                                                                |
| Lint Rust    | `cargo clippy --workspace --all-targets` : tous les lints `clippy::all` et `clippy::pedantic` sont bloquants, ainsi que les avertissements du compilateur (réglage `[workspace.lints]`). |
| Format front | `pnpm format:check` (Prettier)                                                                                                                                                           |
| Lint front   | `pnpm lint` (ESLint, `typescript-eslint` strict, `eslint-plugin-vue`), 0 avertissement toléré                                                                                            |
| Typage       | `pnpm typecheck` (`vue-tsc` strict, `noUncheckedIndexedAccess`, `exactOptionalPropertyTypes`)                                                                                            |
| Tests        | `pnpm test:front` (Vitest), `pnpm test:back` (nextest + doctests)                                                                                                                        |
| Bindings     | `pnpm bindings:check`                                                                                                                                                                    |

`make ci` (ou `pnpm verify`) rejoue l'ensemble en local.

## Hooks git

`lefthook` installe les hooks au `pnpm install` (ou avec `pnpm exec lefthook install`) :

- avant un commit : format Rust, lint et format des fichiers modifiés ;
- message de commit : Conventional Commits en anglais (`type(portée): sujet`), vérifié par `scripts/check-commit-message.mjs` ;
- avant un push : tous les tests.

Un poste peut ajouter ses propres contrôles dans `lefthook-local.yml`, non versionné.

## Intégration continue

`.github/workflows/ci.yml` lance tous les contrôles sur Ubuntu 22.04, macOS et Windows, à chaque PR et à chaque push sur `main`.
