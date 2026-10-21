# Outillage

## Versions fixées

- Rust : `rust-toolchain.toml` (rustfmt et clippy inclus).
- Node : `.node-version`, lu par la CI ; `engines` de `package.json` signale une autre version majeure en local.
- pnpm : champ `packageManager` de `package.json`. Les dépendances front sont en version exacte (`save-exact`) ; pnpm refuse les paquets publiés depuis moins d'un jour et n'exécute aucun script d'installation non autorisé (`pnpm-workspace.yaml`).
- Rust : Tauri limité aux correctifs de sa version mineure (`~`), `tauri-specta` et `specta` en version exacte (versions candidates).

## Qualité

| Contrôle     | Commande                                                                                                                                                                                                 |
| ------------ | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Format Rust  | `cargo fmt --all --check`                                                                                                                                                                                |
| Lint Rust    | `cargo clippy --workspace --all-targets -- -D warnings` : les lints `clippy::all` et `clippy::pedantic` sont bloquants (`[workspace.lints]`), comme tout avertissement du compilateur au moment du lint. |
| Format front | `pnpm format:check` (Prettier)                                                                                                                                                                           |
| Lint front   | `pnpm lint` (ESLint, `typescript-eslint` strict, `eslint-plugin-vue`), 0 avertissement toléré                                                                                                            |
| Typage       | `pnpm typecheck` (`vue-tsc` strict, `noUncheckedIndexedAccess`, `exactOptionalPropertyTypes`)                                                                                                            |
| Tests        | `pnpm test:front` (Vitest), `pnpm test:back` (nextest + doctests)                                                                                                                                        |
| Bindings     | `pnpm bindings:check` (régénère `src/bindings.ts`, échoue s'il diffère de la version commitée)                                                                                                           |

`make ci` (ou `pnpm verify`) rejoue l'ensemble en local ; la CI lance exactement cette commande.

Le format et le lint ne regardent que les fichiers du projet : les fichiers ignorés par git, y compris par la configuration locale `.git/info/exclude`, sont laissés tels quels.

## Hooks git

`lefthook` installe les hooks au `pnpm install` (ou avec `pnpm exec lefthook install`) :

- avant un commit : format et lint Rust, lint et format des fichiers modifiés ;
- message de commit : format Conventional Commits (`type(portée): sujet`) vérifié par `scripts/check-commit-message.mjs`. Le sujet s'écrit en anglais ; la langue n'est pas vérifiée automatiquement ;
- avant un push : tous les tests.

Un poste peut ajouter ses propres contrôles dans `lefthook-local.yml`, non versionné.

## Intégration continue

`.github/workflows/ci.yml` lance `pnpm verify` sur Ubuntu 22.04, macOS et Windows, à chaque PR et à chaque push sur `main`.
