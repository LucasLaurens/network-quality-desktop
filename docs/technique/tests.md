# Tests

## Points d'entrée

| Point d'entrée     | Ce qu'on vérifie                                                                                                                                                      | Où                                   |
| ------------------ | --------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------ |
| Moteur (principal) | Messages envoyés à `MeasurementEngine` avec des faux en mémoire (`InMemoryNetwork`, `ManualClock`, `InMemoryVerdictPresenter`) ; on vérifie le `VerdictView` présenté | `crates/application/tests/engine.rs` |
| Front              | Composants montés avec `InMemoryNetworkQualityClient` injecté                                                                                                         | `tests/`                             |
| Adaptateurs        | Test réel de la sonde ICMP, lancé à la demande                                                                                                                        | `crates/infrastructure`              |
| Shell              | Dessin des icônes, couleurs identiques aux tokens CSS, export des bindings                                                                                            | `src-tauri`                          |

Les faux en mémoire sont des implémentations concrètes des ports (`crates/application/src/test_support.rs`), jamais des mocks. Ils ne sont compilés que pour les tests, via la feature `test-support`. Les tests du moteur utilisent des adresses de documentation (RFC 5737) : le moteur ne dépend pas des cibles choisies.

## Preuve que les tests détectent les erreurs

Chaque règle a été cassée volontairement pour vérifier qu'un test échoue : délais d'hystérésis raccourcis (2 tests rouges), p50 au lieu de p90 (3 tests rouges), couleur modifiée dans `tokens.css` (1 test rouge).
