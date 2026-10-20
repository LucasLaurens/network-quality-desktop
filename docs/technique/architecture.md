# Architecture

## Découpage hexagonal

Le cœur est en Rust, organisé en workspace Cargo de quatre crates. Le sens des dépendances est garanti par le compilateur : une crate ne peut pas importer une crate de la couche au-dessus.

| Crate                       | Rôle                                                                                              | Dépend de                                             |
| --------------------------- | ------------------------------------------------------------------------------------------------- | ----------------------------------------------------- |
| `domain`                    | Règles pures de la mesure (verdict, seuils, hystérésis). Aucune entrée-sortie. `unsafe` interdit. | rien                                                  |
| `application`               | Cas d'usage et ports (traits) que l'infrastructure implémente. `unsafe` interdit.                 | `domain`                                              |
| `infrastructure`            | Adaptateurs par OS, stockage, clients HTTP. Seul endroit où l'`unsafe` est permis (FFI native).   | `domain`, `application`                               |
| `app` (dossier `src-tauri`) | Shell Tauri : assemble les adaptateurs et expose des commandes typées au front.                   | les trois autres, au fur et à mesure qu'elles servent |

État actuel :

- `domain` : niveaux, seuils, fenêtre de latence, médiane, hystérésis, verdict.
- `application` : moteur de mesure (acteur), ports `LatencyProbe`, `Clock`, `VerdictPresenter`, faux en mémoire.
- `infrastructure` : sonde ICMP et horloge système.
- `app` : icône de barre système, horloge à 1 Hz, commande `read_app_info`.

## Front

Vue 3 + TypeScript. Le front ne contient aucune règle métier : il affiche ce que Rust lui envoie.

- Les composants ne parlent jamais directement à Tauri. Ils passent par le port `NetworkQualityClient`, injecté par `provide`/`inject`.
- En production, `TauriNetworkQualityClient` appelle les commandes Rust.
- En test, `InMemoryNetworkQualityClient` (dans `tests/fakes`) renvoie des données préparées : aucun mock de module.

## Types partagés

Les types et les signatures des commandes sont générés depuis Rust par `tauri-specta` dans `src/bindings.ts`. Ce fichier ne se modifie jamais à la main : le test Rust `exports_up_to_date_typescript_bindings` le régénère, et la CI échoue s'il diffère de la version commitée (`pnpm bindings:check`).
