# Mesure

## Sonde

- Port `LatencyProbe` (crate `application`) : `send_probe` renvoie une `ProbeOutcome` (réponse avec son temps, perdue mais joignable, injoignable).
- Adaptateur `FallbackLatencyProbe` (crate `infrastructure`), composé de deux briques remplaçables :
  - `IcmpEchoSender` : ICMP sans privilège via `ping-async` (sockets DGRAM sur macOS et Linux, `IcmpSendEcho2Ex` sur Windows), délai 600 ms ;
  - `TcpConnectionOpener` : poignée de main TCP sur le port 443, délai 350 ms ; un refus de connexion prouve aussi que l'hôte répond.
- Pour chaque cible : l'écho d'abord. S'il est perdu, une connexion TCP dit si la cible est encore joignable (perte) ou non (injoignable). Les deux délais tiennent dans la seconde.
- Repli TCP permanent pour une cible quand l'ICMP est refusé par le système (Linux sans `ping_group_range`) ou filtré (3 échos perdus de suite alors que TCP répond). Le temps de la poignée de main sert alors de latence. Ce repli dure jusqu'au redémarrage de l'app.
- Cibles par défaut : `1.1.1.1` et `8.8.8.8` (`list_default_targets`), sondées en parallèle à chaque seconde.
- Les tests réels de l'ICMP et du TCP ne tournent qu'à la demande : `cargo test -p infrastructure -- --ignored`.

## Moteur

- Un acteur unique, `MeasurementEngine`, possède tout l'état et reçoit des messages par un canal tokio. L'app lui envoie `Tick` chaque seconde (`MissedTickBehavior::Delay`) ; tout tourne côté Rust, jamais dans le webview.
- La fenêtre compte des `Tick` : si le système retarde l'horloge (veille, App Nap), « 15 s » devient « 15 ticks ».
- À chaque `Tick` : sondes en parallèle, enregistrement dans les fenêtres de latence et de perte, détection Hors ligne, puis calcul des trois `MetricReading` (valeur, niveau, proche du seuil) et du verdict, envoyés dans un `VerdictView` au `VerdictPresenter`.
- L'horloge passe par le port `Clock` (`SystemClock` en production, horloge manuelle en test).

## Complexité

| Opération                        | Temps                     | Espace                            |
| -------------------------------- | ------------------------- | --------------------------------- |
| Ajout d'une seconde à la fenêtre | O(1)                      | O(1), fenêtre bornée à 15 entrées |
| p90 de la fenêtre                | O(n log n), n ≤ 15        | O(n)                              |
| Médiane des cibles               | O(k log k), k = 2         | O(k)                              |
| Gigue de la fenêtre              | O(n), n ≤ 15              | O(n)                              |
| Fenêtre de perte (ajout, taux)   | O(1), totaux tenus à jour | O(1), 60 entrées au plus          |
| Détection Hors ligne             | O(1)                      | O(1)                              |
| Hystérésis                       | O(1)                      | O(1)                              |
| Dessin de l'icône                | O(s²), s = 44 px          | O(s²)                             |
