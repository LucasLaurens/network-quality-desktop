# Mesure

## Sonde de latence

- Port `LatencyProbe` (crate `application`), adaptateur `IcmpLatencyProbe` (crate `infrastructure`) basé sur `ping-async` : ICMP sans privilège, sockets DGRAM sur macOS et Linux, `IcmpSendEcho2Ex` sur Windows.
- Délai maximal d'une sonde : 900 ms, pour qu'elle ne déborde jamais sur la seconde suivante. Une réponse plus lente compte comme perdue.
- Cibles par défaut : `1.1.1.1` et `8.8.8.8` (`list_default_targets`), sondées en parallèle à chaque seconde.
- Linux : le réglage `net.ipv4.ping_group_range` doit inclure l'utilisateur, sinon toutes les sondes échouent (repli TCP prévu par le ticket « Verdict complet »).
- Le test réel de la sonde ne tourne qu'à la demande : `cargo test -p infrastructure -- --ignored`.

## Moteur

- Un acteur unique, `MeasurementEngine`, possède tout l'état et reçoit des messages par un canal tokio. L'app lui envoie `Tick` chaque seconde (`MissedTickBehavior::Delay`) ; tout tourne côté Rust, jamais dans le webview.
- La fenêtre compte des `Tick` : si le système retarde l'horloge (veille, App Nap), « 15 s » devient « 15 ticks ».
- À chaque `Tick` : sondes en parallèle, médiane des réponses, ajout à la fenêtre, p90, niveau, hystérésis, puis envoi d'un `VerdictView` au `VerdictPresenter`.
- L'horloge passe par le port `Clock` (`SystemClock` en production, horloge manuelle en test).

## Complexité

| Opération                        | Temps              | Espace                            |
| -------------------------------- | ------------------ | --------------------------------- |
| Ajout d'une seconde à la fenêtre | O(1)               | O(1), fenêtre bornée à 15 entrées |
| p90 de la fenêtre                | O(n log n), n ≤ 15 | O(n)                              |
| Médiane des cibles               | O(k log k), k = 2  | O(k)                              |
| Hystérésis                       | O(1)               | O(1)                              |
| Dessin de l'icône                | O(s²), s = 44 px   | O(s²)                             |
