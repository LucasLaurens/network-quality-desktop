# Persistance

Schéma SQLite, agrégation, transactions, migrations et budgets de complexité.

## Principe

Chaque donnée stockée sert un affichage ; ce qui se déduit n'est pas stocké. Aucune mesure brute : le verdict travaille en mémoire sur 15 à 60 s, la dernière heure est un tampon circulaire en mémoire (`SecondRing`, 3 600 points, O(1) par seconde), le reste lit les agrégats.

## Fichier

`history.sqlite3` dans `app_data_dir` (Tauri), via `rusqlite` (feature `bundled` : SQLite compilé avec l'app) et `rusqlite_migration`. Ouverture : `PRAGMA quick_check`, puis `journal_mode=WAL`, `synchronous=NORMAL`, `foreign_keys=ON`, puis migrations. Horodatages en secondes UTC (entiers).

`busy_timeout` de 2 s. Seule une base abîmée est mise de côté : erreur SQLite `NOTADB` ou `CORRUPT`, ou `quick_check` différent de `ok`. Elle est renommée `history.corrupt-<secondes UTC>.sqlite3`, ses journaux `-wal` et `-shm` renommés avec elle (ils peuvent contenir des écritures validées), puis une nouvelle base est créée et un message rendu par la commande `read_history_notice`. Toute autre erreur (base occupée, permissions, schéma plus récent que l'app) laisse le fichier intact : base SQLite en mémoire pour la session, avec un message.

## Schéma (migration `0001_create_history.sql`)

| Table                                                              | Colonnes                                                                                                                                                                                                     | Rétention                                   |
| ------------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | ------------------------------------------- |
| `networks`                                                         | `id`, `gateway_mac` (6 octets, NULL si illisible), `connection_kind`, `label` ; UNIQUE (`gateway_mac`, `connection_kind`)                                                                                    | illimitée                                   |
| `aggregates` (WITHOUT ROWID, clé `network_id, granularity, start`) | `latency_hist` (24 compteurs u32 en petit-boutiste), `jitter_sum_us`, `jitter_count`, `probes_sent`, `probes_lost`, `s_solid`, `s_unstable`, `s_degraded`, `s_offline` ; `granularity` 0 = minute, 1 = heure | minute : 30 j ; heure : illimitée           |
| `events`                                                           | `ts`, `network_id`, `kind` (`network_changed`, `offline_started`, `offline_ended`, `sleep`, `wake`)                                                                                                          | illimitée                                   |
| `throughput_tests`                                                 | `ts`, `network_id`, `profile`, `down_kbps`, `up_kbps`, `loaded_latency_ms`, `bytes`, `duration_ms`, `status`                                                                                                 | illimitée (remplie avec les tests de débit) |
| `settings`                                                         | une seule ligne typée (`CHECK (id = 1)`) : `autostart`, `theme`, `ping_target`, `notify`, `tests_allowed`                                                                                                    | — (lue avec la page Réglages)               |

Les migrations sont numérotées, jamais modifiées une fois publiées ; un test les valide sur une base vide. Une nouvelle colonne ou table = un nouveau fichier. `0002_index_purge_and_unreadable_gateways.sql` ajoute l'index `(granularity, start)` pour la purge, et un index unique partiel : une seule ligne `networks` par type quand la MAC est illisible (UNIQUE traite les NULL comme distincts).

Le réseau d'une ligne est identifié par la MAC de la passerelle et le type ; le libellé (nom du Wi-Fi) est mis à jour quand il devient lisible, jamais effacé.

## Histogramme de latence

24 tranches (`LatencyHistogram`, domaine) : une sous 1 ms, 22 réparties en échelle logarithmique de 1 ms à 2 s, une au-delà. Les histogrammes s'additionnent : médiane et p90 d'une période quelconque se calculent sur la somme de ses minutes ou de ses heures, à la précision d'une tranche (le percentile rendu est la borne haute de sa tranche ; pour la dernière, « 2 s ou plus »).

## Écriture

- L'acteur de mesure ne touche jamais la base. Son `HistoryTracker` (couche application) cumule la minute (`MinuteAccumulator`) et envoie des `HistoryCommand` par le port `HistoryRecorder`, sans attendre.
- `SqliteHistoryRecorder` (infrastructure) : un fil d'écriture unique, propriétaire de la base, reçoit ces commandes par un canal et les applique par `HistoryWriter` (couche application) dans l'ordre.
- Chaque écriture est une transaction : ajout d'une minute (fusionnée avec une minute déjà gardée pour le même début, ce qui complète une minute coupée par un redémarrage), écriture d'une heure (remplace), purge, événement.
- Dernière minute d'une heure : les 60 minutes sont relues et fusionnées en une ligne heure. Première minute d'une heure nouvelle : toutes les heures précédentes encore ouvertes, de tous les réseaux, sont fusionnées (réseau quitté avant la fin de l'heure, veille), puis les minutes de plus de 30 jours sont purgées.
- La purge ne supprime que les minutes dont l'heure est conservée (`EXISTS` sur la clé primaire) : une heure jamais fusionnée garde ses minutes.
- Au démarrage du fil : les heures qui ont des minutes mais pas de ligne heure, avant l'heure courante, sont terminées.
- Changement de réseau : la minute en cours part sous l'ancien réseau, un événement `network_changed` est noté. Le même réseau relu sans sa MAC ou sans son nom garde ceux appris avant (`complete_with`). Coupure : `offline_started` puis `offline_ended`. Sans route, les secondes comptent pour le dernier réseau vu ; avant le premier réseau vu, sous « aucun réseau » (type `other`, sans MAC).
- La gigue d'une seconde n'est comptée que si la seconde précédente est mesurée : pas d'écart calculé à travers une veille ou un tick sauté. L'heure UTC d'un tick est lue avant les sondes : deux ticks ne tombent jamais dans la même seconde.
- Fermeture : `RunEvent::Exit` envoie `EngineMessage::Flush` ; l'app attend au plus 2 s en tout, envoi compris (boîte pleine, fil arrêté), que le fil confirme l'écriture de la minute en cours.
- Veille : `EngineMessage::Sleep` ferme une coupure en cours (`offline_ended`), note `sleep`, écrit la minute en cours puis répond ; l'avis de veille attend au plus 1,5 s (`SLEEP_TIMEOUT`, sous les 2 s de Windows). Réveil : `wake` est noté, et la gigue ne se calcule pas à travers la veille. Si la veille n'a pas été annoncée (réveil vu par le filet) et qu'une coupure était en cours, elle se termine à la dernière seconde mesurée avant la veille : la nuit n'est pas comptée comme une coupure.
- La dernière heure et le réseau courant se lisent par `EngineMessage::ReadLiveHistory` (`LiveHistory`).

## Lecture

- Le tableau de bord lit par le port `HistoryReader` (`list_all_aggregates`, `list_events`), séparé de `MeasurementRepository` : l'écrivain n'en a pas besoin. `list_all_aggregates` rend les lignes de tous les réseaux d'une granularité et d'une plage, par l'index `(granularity, start)`.
- Le shell ouvre une seconde connexion SQLite en lecture seule (`SqliteMeasurementRepository::open_read_only`), après l'écrivain : en WAL, elle lit pendant qu'il écrit, sans l'attendre, et ne peut rien écrire. Si l'historique vit en mémoire (repli), le lecteur est un objet nul qui ne rend rien : seule la dernière heure en mémoire s'affiche.
- Les calculs sont des fonctions de la couche application (`dashboard.rs`) : `read_evolution` (seaux de largeur fixe, vides là où rien n'a été mesuré, changements de réseau, couverture), `read_day_overview` (journée, coupures, pire minute, arrivée sur le réseau courant), `list_network_summaries` (30 jours par réseau, le plus fiable). Les périodes de 7 et 30 jours lisent les heures closes puis les minutes de l'heure en cours. La dernière heure lit les minutes écrites avant le début de la mémoire, puis la mémoire, sans compter deux fois la minute où la mémoire commence.
- Deux événements `network_changed` successifs du même type, dont l'un n'a pas de MAC (lue plus tard) et dont les noms ne diffèrent pas, ne sont pas un changement (`NetworkKey::is_likely_same`). Le regroupement par réseau suit l'identité stockée (`is_same_identity`).

## Réglages

- La ligne typée `settings` (id 1, créée par la première migration) garde `autostart`, `theme` (`system`, `light`, `dark`), `notify` et `ping_target` (adresse IP du serveur personnalisé, `NULL` pour les serveurs publics). Une adresse illisible relue retombe sur les serveurs publics.
- Le port `SettingsRepository` (`read_settings`, `save_settings`, une transaction) est servi par une connexion d'écriture de plus, ouverte après l'écrivain (`SqliteMeasurementRepository::open_alongside`) : une écriture rare, qui attend l'écrivain au plus `BUSY_TIMEOUT`. Suite de contrat : `check_settings_contract`, sur le faux en mémoire et sur SQLite.

## Contrat

Les ports `MeasurementRepository` et `HistoryReader` ont deux implémentations : `InMemoryMeasurementRepository` (faux pour les tests) et `SqliteMeasurementRepository`. Une seule suite, `application::repository_contract::check_contract`, tourne sur les deux (SQLite en mémoire) : ajout et fusion d'une minute, remplacement d'une heure, plage triée, réseaux séparés, heures ouvertes, purge, minutes d'une heure non conservée gardées, événements triés, libellé appris plus tard ou par un événement et jamais effacé, réseaux sans MAC regroupés par type, lignes de tous les réseaux d'une plage.

## Complexité

| Opération                                        | Temps                                                                                        | Espace                   |
| ------------------------------------------------ | -------------------------------------------------------------------------------------------- | ------------------------ |
| Seconde (cumul de la minute, tampon d'une heure) | O(1)                                                                                         | O(1)                     |
| Fin de minute (écriture depuis la mémoire)       | O(B), B = 24                                                                                 | O(B)                     |
| Fusion horaire                                   | O(60·B)                                                                                      | O(60·B)                  |
| Percentile sur un histogramme fusionné           | O(B)                                                                                         | O(1)                     |
| Période sur la clé primaire                      | O(log n + k)                                                                                 | O(k)                     |
| Purge des minutes                                | O(log n + k) par l'index `(granularity, start)`, une vérification sur clé primaire par ligne | O(1)                     |
| Heures restées ouvertes                          | O(m log m), m minutes gardées                                                                | O(m)                     |
| Ouverture (`quick_check`)                        | O(taille du fichier)                                                                         | O(1)                     |
| Lecture de la dernière heure                     | O(n), n ≤ 3 600                                                                              | O(n)                     |
| Courbe d'une période                             | O(log n + r + s + e), r lignes, s ≤ 3 600 secondes en mémoire, e événements                  | O(r + p), p ≤ 241 points |
| Réseaux sur 30 jours                             | O(log n + r + k log k), k réseaux                                                            | O(r + k)                 |
