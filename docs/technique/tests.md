# Tests

## Points d'entrée

| Point d'entrée     | Ce qu'on vérifie                                                                                                                                                      | Où                                   |
| ------------------ | --------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------ |
| Moteur (principal) | Messages envoyés à `MeasurementEngine` avec des faux en mémoire (`InMemoryNetwork`, `ManualClock`, `InMemoryVerdictPresenter`) ; on vérifie le `VerdictView` présenté | `crates/application/tests/engine.rs` |
| Front              | Composants montés avec `InMemoryNetworkQualityClient` injecté (popover : affichage, mise à jour, bouton, arrêt de l'écoute)                                           | `tests/`                             |
| Adaptateurs        | Logique de repli de la sonde avec un écho et un TCP en mémoire ; tests réels ICMP et TCP lancés à la demande                                                          | `crates/infrastructure`              |
| Shell              | Dessin des icônes, couleurs identiques aux tokens CSS, objet d'affichage et textes, règles typographiques du catalogue, export des bindings                           | `src-tauri`                          |

Les faux en mémoire sont des implémentations concrètes des ports (`crates/application/src/test_support.rs`), jamais des mocks. Ils ne sont compilés que pour les tests, via la feature `test-support`. Les tests du moteur utilisent des adresses de documentation (RFC 5737) : le moteur ne dépend pas des cibles choisies.

## Preuve que les tests détectent les erreurs

Chaque règle a été cassée volontairement pour vérifier qu'un test échoue : délais d'hystérésis raccourcis (2 tests rouges), p50 au lieu de p90 (3 tests rouges), couleur modifiée dans `tokens.css` (1 test rouge), seuil de bascule TCP abaissé (2 tests rouges), cible injoignable classée « perdue » (1 test rouge), perte notée dès la première sonde (1 test rouge), gigue calculée sur la médiane des cibles (1 test rouge), « 60s » ou « 5% » dans le catalogue (1 test rouge chacun), retour au calme diagnostiqué « hors seuils » (1 test rouge), affichage écrasé par une première lecture arrivée en retard (1 test rouge).

## Aperçu visuel

`tests/preview/popover.html` (servie par `pnpm dev:web`, hors build) affiche le popover avec des données préparées (`?scenario=degraded` ou `offline`). Elle sert aux contrôles visuels à la main (navigateur ou Playwright, dans les deux thèmes) : aucun débordement, deux rayons seulement (2 px et 8 px), focus visible au clavier. Ces contrôles ne sont pas automatisés.
