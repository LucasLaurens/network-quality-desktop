# Persistance

Schéma SQLite, agrégation, transactions, migrations et budgets de complexité.

## Principe

Chaque donnée stockée sert un affichage ; ce qui se déduit n'est pas stocké. Aucune mesure brute : le verdict travaille en mémoire sur 15 à 60 s, la dernière heure est un tampon circulaire en mémoire (`SecondRing`, 3 600 points, O(1) par seconde), le reste lit les agrégats.

## Fichier

`history.sqlite3` dans `app_data_dir` (Tauri), via `rusqlite` (feature `bundled` : SQLite compilé avec l'app) et `rusqlite_migration`. Ouverture : `PRAGMA quick_check`, puis `journal_mode=WAL`, `synchronous=NORMAL`, `foreign_keys=ON`, puis migrations. Horodatages en secondes UTC (entiers).

Fichier illisible ou contrôle d'intégrité en échec : renommé `history.corrupt-<secondes UTC>.sqlite3` (journaux `-wal` et `-shm` supprimés), nouvelle base créée, message rendu par la commande `read_history_notice`. Ouverture impossible sur disque : base SQLite en mémoire pour la session, avec un message.

## Schéma (migration `0001_create_history.sql`)

| Table                                                              | Colonnes                                                                                                                                                                                                     | Rétention                                   |
| ------------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | ------------------------------------------- |
| `networks`                                                         | `id`, `gateway_mac` (6 octets, NULL si illisible), `connection_kind`, `label` ; UNIQUE (`gateway_mac`, `connection_kind`)                                                                                    | illimitée                                   |
| `aggregates` (WITHOUT ROWID, clé `network_id, granularity, start`) | `latency_hist` (24 compteurs u32 en petit-boutiste), `jitter_sum_us`, `jitter_count`, `probes_sent`, `probes_lost`, `s_solid`, `s_unstable`, `s_degraded`, `s_offline` ; `granularity` 0 = minute, 1 = heure | minute : 30 j ; heure : illimitée           |
| `events`                                                           | `ts`, `network_id`, `kind` (`network_changed`, `offline_started`, `offline_ended`, `sleep`, `wake`)                                                                                                          | illimitée                                   |
| `throughput_tests`                                                 | `ts`, `network_id`, `profile`, `down_kbps`, `up_kbps`, `loaded_latency_ms`, `bytes`, `duration_ms`, `status`                                                                                                 | illimitée (remplie avec les tests de débit) |
| `settings`                                                         | une seule ligne typée (`CHECK (id = 1)`) : `autostart`, `theme`, `ping_target`, `notify`, `tests_allowed`                                                                                                    | — (lue avec la page Réglages)               |

Les migrations sont numérotées, jamais modifiées une fois publiées ; un test les valide sur une base vide. Une nouvelle colonne ou table = un nouveau fichier.

Le réseau d'une ligne est identifié par la MAC de la passerelle et le type ; le libellé (nom du Wi-Fi) est mis à jour quand il devient lisible, jamais effacé.

## Histogramme de latence

24 tranches (`LatencyHistogram`, domaine) : une sous 1 ms, 22 réparties en échelle logarithmique de 1 ms à 2 s, une au-delà. Les histogrammes s'additionnent : médiane et p90 d'une période quelconque se calculent sur la somme de ses minutes ou de ses heures, à la précision d'une tranche (le percentile rendu est la borne haute de sa tranche).

## Écriture

- L'acteur de mesure ne touche jamais la base. Son `HistoryTracker` (couche application) cumule la minute (`MinuteAccumulator`) et envoie des `HistoryCommand` par le port `HistoryRecorder`, sans attendre.
- `SqliteHistoryRecorder` (infrastructure) : un fil d'écriture unique, propriétaire de la base, reçoit ces commandes par un canal et les applique par `HistoryWriter` (couche application) dans l'ordre.
- Chaque écriture est une transaction : ajout d'une minute (fusionnée avec une minute déjà gardée pour le même début, ce qui complète une minute coupée par un redémarrage), écriture d'une heure (remplace), purge, événement.
- Dernière minute d'une heure : les 60 minutes sont relues et fusionnées en une ligne heure, puis les minutes de plus de 30 jours sont purgées.
- Au démarrage du fil : les heures qui ont des minutes mais pas de ligne heure, avant l'heure courante, sont terminées.
- Changement de réseau : la minute en cours part sous l'ancien réseau, un événement `network_changed` est noté. Coupure : `offline_started` puis `offline_ended`. Sans route, les secondes comptent pour le dernier réseau vu.
- Fermeture : `RunEvent::Exit` envoie `EngineMessage::Flush` ; l'app attend au plus 2 s que le fil confirme l'écriture de la minute en cours. Veille et réveil s'y ajouteront avec leur ticket.

## Contrat

Le port `MeasurementRepository` a deux implémentations : `InMemoryMeasurementRepository` (faux pour les tests) et `SqliteMeasurementRepository`. Une seule suite, `application::repository_contract::check_contract`, tourne sur les deux (SQLite en mémoire) : ajout et fusion d'une minute, remplacement d'une heure, plage triée, réseaux séparés, heures ouvertes, purge, événements, libellé appris plus tard.

## Complexité

| Opération                                        | Temps                        | Espace  |
| ------------------------------------------------ | ---------------------------- | ------- |
| Seconde (cumul de la minute, tampon d'une heure) | O(1)                         | O(1)    |
| Fin de minute (écriture depuis la mémoire)       | O(B), B = 24                 | O(B)    |
| Fusion horaire                                   | O(60·B)                      | O(60·B) |
| Percentile sur un histogramme fusionné           | O(B)                         | O(1)    |
| Période sur la clé primaire                      | O(log n + k)                 | O(k)    |
| Purge des minutes                                | suppression par plage de clé | O(1)    |
| Lecture de la dernière heure                     | O(n), n ≤ 3 600              | O(n)    |
