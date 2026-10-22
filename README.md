# Qualité de connexion

App de bureau (macOS, Linux, Windows) qui affiche en temps réel l'état de la connexion Internet : **Solide**, **Instable**, **Dégradée**, **Hors ligne** ou **Mesure en cours**, avec la raison en une phrase et un historique par réseau. Locale d'abord et gratuite : par défaut, seuls de petits pings quittent l'ordinateur.

> État actuel (version 0.1.0) : la pastille suit en continu la latence, la gigue et la perte ; un clic ouvre le popover avec l'état, le réseau et son type de connexion, la cause et le tronçon en cause sur le chemin Ce Mac → Box → Internet, et les trois mesures, un clic droit le menu (seul accès sous Linux). Sans barre système, l'app reste ouverte dans une fenêtre. Elle tourne en tâche de fond (une seule instance), écrit la minute en cours avant la veille et repart en « Mesure en cours » au réveil. L'historique est conservé sur la machine (résumés par minute pendant 30 jours, par heure sans limite) ; le tableau de bord montre la journée, l'évolution sur 1 h à 30 jours et la comparaison des réseaux. Une notification prévient quand la connexion se dégrade durablement. Des tests de débit, désactivés par défaut, se lancent d'un clic. Des détails techniques repliés montrent le trafic observé de l'ordinateur et la ligne de la box (UPnP en lecture seule), avec l'accès annoncé ou estimé. La page Réglages règle l'ouverture de session, l'apparence, les serveurs de mesure, les notifications et les tests de débit ; elle exporte l'historique en trois fichiers CSV ou l'efface. La feuille de route est suivie dans les issues GitHub.

## Prérequis

- Rust : la version est fixée par `rust-toolchain.toml` (installée automatiquement par rustup).
- Node : version de `.node-version`. pnpm : version du champ `packageManager` de `package.json` (`npm i -g pnpm@<version>`).
- Dépendances système de Tauri 2 : voir <https://v2.tauri.app/start/prerequisites/>.
- `cargo-nextest` pour les tests Rust : `cargo install cargo-nextest --locked`.
- Pour les contrôles hors barrière quotidienne : `cargo-llvm-cov` (avec `rustup component add llvm-tools-preview`), `cargo-deny` et `cargo-mutants`, chacun par `cargo install <outil> --locked`.

## Démarrer

```bash
pnpm install
make dev
```

## Commandes

| Commande                             | Effet                                                             |
| ------------------------------------ | ----------------------------------------------------------------- |
| `make dev`                           | Lance l'app en développement (rechargement à chaud du front).     |
| `make build`                         | Construit les installeurs de l'app.                               |
| `make install`                       | Construit l'app ; sur macOS, la copie dans Applications.          |
| `make test`                          | Tous les tests (front puis Rust).                                 |
| `make test-front` / `make test-back` | Tests du front (Vitest) ou de Rust (nextest).                     |
| `make lint`                          | Lint du front (ESLint) et de Rust (rustfmt, clippy).              |
| `make fmt`                           | Formate tout le code.                                             |
| `make ci`                            | Rejoue en local toute la barrière de la CI.                       |
| `make coverage`                      | Couverture bloquante : `domain` ≥ 90 %, `application` ≥ 80 %.     |
| `make audit`                         | Licences et failles des dépendances (`cargo-deny`, `pnpm audit`). |
| `make mutants`                       | Tests de mutation du domaine (long).                              |

Les commandes sont définies une seule fois dans `package.json` ; le `Makefile` n'est qu'un raccourci (sous Windows, utiliser `pnpm <script>`).

## Documentation

- [Documentation fonctionnelle](docs/fonctionnel/) : ce que fait l'app.
- [Documentation technique](docs/technique/) : architecture, outillage, glossaire.
- [PRODUCT.md](PRODUCT.md) : utilisateurs, personnalité, principes de design.
- [DESIGN.md](DESIGN.md) : système visuel.
