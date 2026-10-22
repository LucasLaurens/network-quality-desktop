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

Les règles de code décidées sont vérifiées par la machine. Chaque vérification a été vue échouer sur un cas fautif ajouté exprès, puis passer une fois le cas retiré : un nom de fonction fautif (`total_price` en Rust, `totalPrice` en TypeScript), une PR de code sans docs (tests de la règle), un seuil de couverture au-dessus de la valeur réelle, une licence retirée de `deny.toml`.

- **Noms de fonctions** : une fonction est une action, son nom commence par un verbe. Les premiers mots permis vivent dans un seul fichier, `scripts/method-verbs.txt` (un mot par ligne) : les verbes, les idiomes `from`, `to`, `is`, `has`, et `get`, réservé à un accesseur sur un champ déjà en main (règle relue en revue, la machine ne peut pas la vérifier). On ajoute un verbe seulement s'il dit vrai de ce que fait la fonction.
  - Rust : la crate `conventions` parcourt chaque source du workspace avec `syn` (test `names_every_function_with_a_verb`, lancé par `pnpm test:back`). Sont vérifiées les fonctions libres, les méthodes propres et les méthodes déclarées dans un trait, privées comprises. Exemptions : `new`, `main`, les méthodes d'une implémentation de trait (le trait impose le nom) et les tests (`#[test]`, `#[tokio::test]`), dont le nom se lit comme une phrase.
  - TypeScript : `scripts/check-method-names.mjs` lit avec l'API du compilateur ce que chaque module exporte, dans `src/`, `scripts/`, `tests/fakes/` et `tests/preview/` : fonctions (y compris par `export { … }`), constantes qui valent une fonction, méthodes et propriétés-fonctions des classes et des interfaces (lancé par `pnpm lint`). Comme en Rust, un `_` en tête est ignoré. Exemptions : constructeurs, accesseurs `get`. Le ticket demande les fonctions exportées : les fonctions locales et le `<script setup>` des `.vue`, qui n'exporte rien, ne sont pas relus. `src/bindings.ts`, généré à partir des commandes Rust, non plus.
- **Documentation** : une PR qui touche `crates/`, `src/`, `src-tauri/`, `scripts/` ou `tests/` sans toucher `docs/` ni `README.md` échoue au contrôle « Documentation à jour » (`.github/workflows/docs.yml`, règle dans `scripts/docs-check.mjs`), sauf si elle porte le label `sans-doc`. Le contrôle est relancé quand on pose ou retire un label et quand la branche de base change. Les fichiers de configuration (`Cargo.toml`, `package.json`, workflows) ne comptent pas comme du code.
- **Couverture** : `pnpm coverage:back` (`scripts/coverage-back.mjs`, `cargo-llvm-cov`) échoue sous 90 % des lignes de `domain` et 80 % de celles d'`application`, sans compter les faux et le contrat partagé des dépôts (`test_support.rs`, `repository_contract.rs`).
- **Dépendances** : `pnpm deps:audit` lance `cargo deny check advisories licenses` (`deny.toml` : licences permissives et MPL-2.0 permises ; toute faille bloque, un crate abandonné seulement si l'app en dépend directement, car Tauri tire sous Linux des liaisons GTK abandonnées qu'on ne peut pas remplacer) puis `pnpm audit`.
- **Mutants** : `pnpm mutants:domain` (`cargo-mutants`) modifie le domaine pour vérifier qu'un test échoue à chaque fois ; environ 500 mutants, donc lancé à la main. Un premier passage sur `access.rs` a révélé trois variantes inutiles de `WanAccessType` et un test manquant sur la médiane, corrigés.

## Hooks git

`lefthook` installe les hooks au `pnpm install` (ou avec `pnpm exec lefthook install`) :

- avant un commit : format et lint Rust, lint et format des fichiers modifiés ;
- message de commit : format Conventional Commits (`type(portée): sujet`) vérifié par `scripts/check-commit-message.mjs`. Le sujet s'écrit en anglais ; la langue n'est pas vérifiée automatiquement ;
- avant un push : tous les tests.

Un poste peut ajouter ses propres contrôles dans `lefthook-local.yml`, non versionné.

## Intégration continue

- `.github/workflows/ci.yml` lance `pnpm verify` sur Ubuntu 22.04, macOS et Windows, à chaque PR et à chaque push sur `main`, et la couverture sur Ubuntu.
- `.github/workflows/docs.yml` vérifie qu'une PR qui change le code change aussi la documentation.
- `.github/workflows/dependencies.yml` vérifie licences et failles chaque lundi à 6 h UTC, à la demande, et sur toute PR qui change `Cargo.lock`, `pnpm-lock.yaml` ou `deny.toml`.
- `.github/workflows/mutants.yml` lance les mutants du domaine à la demande (onglet Actions).
- Ces trois contrôles lancent les outils directement : un script pnpm installerait d'abord toutes les dépendances du front.
- La branche `main` n'est pas protégée : GitHub ne le permet sur un dépôt privé qu'avec l'offre Pro. Les contrôles échouent visiblement mais n'empêchent pas une fusion.
