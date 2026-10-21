# Tests

## Points d'entrée

| Point d'entrée     | Ce qu'on vérifie                                                                                                                                                                       | Où                                   |
| ------------------ | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------ |
| Moteur (principal) | Messages envoyés à `MeasurementEngine` avec des faux en mémoire (`InMemoryNetwork`, `ManualClock`, `InMemoryVerdictPresenter`) ; on vérifie le `VerdictView` présenté                  | `crates/application/tests/engine.rs` |
| Front              | Composants montés avec `InMemoryNetworkQualityClient` injecté (popover : affichage, mise à jour, bouton, Échap, arrêt de l'écoute ; tableau de bord : note du mode fenêtre)            | `tests/`                             |
| Adaptateurs        | Logique de repli de la sonde avec un écho et un TCP en mémoire ; tests réels ICMP et TCP lancés à la demande                                                                           | `crates/infrastructure`              |
| Shell              | Dessin des icônes, couleurs identiques aux tokens CSS, objet d'affichage et textes, règles typographiques du catalogue, définition du menu, choix du mode fenêtre, export des bindings | `src-tauri`                          |

Les faux en mémoire sont des implémentations concrètes des ports (`crates/application/src/test_support.rs`), jamais des mocks. Ils ne sont compilés que pour les tests, via la feature `test-support`. Les tests du moteur utilisent des adresses de documentation (RFC 5737) : le moteur ne dépend pas des cibles choisies.

## Preuve que les tests détectent les erreurs

Chaque règle a été cassée volontairement pour vérifier qu'un test échoue : délais d'hystérésis raccourcis (2 tests rouges), p50 au lieu de p90 (3 tests rouges), couleur modifiée dans `tokens.css` (1 test rouge), seuil de bascule TCP abaissé (2 tests rouges), cible injoignable classée « perdue » (1 test rouge), perte notée dès la première sonde (1 test rouge), gigue calculée sur la médiane des cibles (1 test rouge), « 60s » ou « 5% » dans le catalogue (1 test rouge chacun), retour au calme diagnostiqué « hors seuils » (1 test rouge), affichage écrasé par une première lecture arrivée en retard (1 test rouge), constat retiré du menu (1 test rouge), partage de connexion déduit d'un seul indice (2 tests rouges), contexte relu à chaque seconde ou jamais au premier tick (1 test rouge chacun).

## Aperçu visuel

Deux pages hors build, servies par `pnpm dev:web`, affichent les écrans avec des données préparées : `tests/preview/popover.html` (`?scenario=degraded` ou `offline`) et `tests/preview/dashboard.html` (`?scenario=window` pour la note du mode fenêtre). Elles servent aux contrôles visuels à la main (navigateur ou Playwright, dans les deux thèmes) : aucun débordement, deux rayons seulement (2 px et 8 px), focus visible au clavier. Ces contrôles ne sont pas automatisés.

## Matrice de tests manuels Linux

La barre système Linux dépend du bureau et ne peut pas être testée en CI. À vérifier à la main avant chaque version :

| Bureau                                                             | Attendu                                                                                                                                                                           | Dernière vérification |
| ------------------------------------------------------------------ | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------- |
| Ubuntu GNOME, extension AppIndicator active (installée par défaut) | Pastille visible ; tout clic ouvre le menu (état, constat, tableau de bord, Quitter) ; état et constat suivent la connexion ; pas de popover                                      | pas encore vérifié    |
| Fedora GNOME, sans extension                                       | Mode fenêtre : tableau de bord ouvert au lancement, présent dans la barre des tâches, réduit à la fermeture, « Quitter » ferme l'app ; note avec lien vers l'extension, masquable | pas encore vérifié    |
| KDE Plasma                                                         | Pastille visible et menu comme sous Ubuntu                                                                                                                                        | pas encore vérifié    |

Le test `asks_the_session_bus_without_failing` (crate `infrastructure`, ignoré par défaut) interroge le vrai bus de session : `cargo test -p infrastructure -- --ignored`.
