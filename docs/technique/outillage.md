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
| Lint front   | `pnpm lint` (ESLint, `typescript-eslint` strict, `eslint-plugin-vue`), 0 avertissement toléré, puis les noms de fonctions exportées (voir plus bas)                                                      |
| Typage       | `pnpm typecheck` (`vue-tsc` strict, `noUncheckedIndexedAccess`, `exactOptionalPropertyTypes`)                                                                                                            |
| Tests        | `pnpm test:front` (Vitest), `pnpm test:back` (nextest + doctests)                                                                                                                                        |
| Bindings     | `pnpm bindings:check` (régénère `src/bindings.ts`, échoue s'il diffère de la version commitée)                                                                                                           |

`make ci` (ou `pnpm verify`) rejoue l'ensemble en local ; la CI lance exactement cette commande.

Le format et le lint ne regardent que les fichiers du projet : les fichiers ignorés par git, y compris par la configuration locale `.git/info/exclude`, sont laissés tels quels.

## Règles encodées

Les règles de code décidées sont vérifiées par la machine :

- **Noms de fonctions** : une fonction est une action, son nom commence par un verbe. La liste des verbes permis est unique, `scripts/method-verbs.txt` (un verbe par ligne) ; on y ajoute un verbe seulement s'il dit vrai de ce que fait la fonction.
  - Rust : la crate `conventions` parcourt chaque source du workspace avec `syn` (test `names_every_function_with_a_verb`, lancé par `pnpm test:back`). Sont vérifiées les fonctions libres, les méthodes propres et les méthodes déclarées dans un trait. Exemptions : `new`, `main`, les préfixes `from_`, `to_`, `is_`, `has_`, les méthodes d'une implémentation de trait (le trait impose le nom) et les tests (`#[test]`, `#[tokio::test]`), dont le nom se lit comme une phrase.
  - TypeScript : `scripts/check-method-names.mjs` lit avec l'API du compilateur les fonctions exportées, les méthodes des classes et des interfaces exportées de `src/`, `scripts/` et `tests/fakes/` (lancé par `pnpm lint`). Exemptions : constructeurs, accesseurs `get`, préfixes `from`, `to`, `is`, `has` ; `src/bindings.ts`, généré à partir des commandes Rust, n'est pas relu.
  - Chaque vérification a été vue échouer sur un nom fautif ajouté exprès (`total_price`, `totalPrice`), puis passer une fois le nom retiré.
- **Documentation** : une PR qui touche `crates/`, `src/`, `src-tauri/`, `scripts/` ou `tests/` sans toucher `docs/` ni `README.md` échoue au contrôle « Documentation à jour » (`.github/workflows/docs.yml`, règle dans `scripts/docs-check.mjs`), sauf si elle porte le label `sans-doc`. Le contrôle est relancé quand on pose ou retire un label.
- **Couverture** : `pnpm coverage:back` (`cargo-llvm-cov`) échoue sous 90 % des lignes de `domain` et 80 % de celles d'`application`. Au moment de l'écrire : 91,3 % et 98,3 %.
- **Dépendances** : `pnpm deps:audit` lance `cargo deny check advisories licenses` (`deny.toml` : licences permissives et MPL-2.0 permises ; toute faille bloque, un crate abandonné seulement si l'app en dépend directement, car Tauri tire sous Linux des liaisons GTK abandonnées qu'on ne peut pas remplacer) puis `pnpm audit`.
- **Mutants** : `pnpm mutants:domain` (`cargo-mutants`) modifie le domaine pour vérifier qu'un test échoue à chaque fois ; environ 500 mutants, donc lancé à la main. Un premier passage sur `access.rs` a révélé trois variantes inutiles de `WanAccessType` et un test manquant sur la médiane, corrigés.

## Hooks git

`lefthook` installe les hooks au `pnpm install` (ou avec `pnpm exec lefthook install`) :

- avant un commit : format et lint Rust, lint et format des fichiers modifiés ;
- message de commit : format Conventional Commits (`type(portée): sujet`) vérifié par `scripts/check-commit-message.mjs`. Le sujet s'écrit en anglais ; la langue n'est pas vérifiée automatiquement ;
- avant un push : tous les tests.

Un poste peut ajouter ses propres contrôles dans `lefthook-local.yml`, non versionné.

## Intégration continue

- `.github/workflows/ci.yml` lance `pnpm verify` sur Ubuntu 22.04, macOS et Windows, à chaque PR et à chaque push sur `main`, et la couverture (`pnpm coverage:back`) sur Ubuntu.
- `.github/workflows/docs.yml` vérifie qu'une PR qui change le code change aussi la documentation.
- `.github/workflows/dependencies.yml` lance `pnpm deps:audit` chaque lundi à 6 h UTC, et à la demande.
- `.github/workflows/mutants.yml` lance `pnpm mutants:domain` à la demande (onglet Actions).
