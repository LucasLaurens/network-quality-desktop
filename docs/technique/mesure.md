# Mesure

## Sonde

- Port `LatencyProbe` (crate `application`) : `send_probe` renvoie une `ProbeOutcome` (réponse avec son temps, perdue mais joignable, injoignable).
- Adaptateur `FallbackLatencyProbe` (crate `infrastructure`), composé de deux briques remplaçables :
  - `IcmpEchoSender` : ICMP sans privilège via `ping-async` (sockets DGRAM sur macOS et Linux, `IcmpSendEcho2Ex` sur Windows), délai 900 ms ;
  - `TcpConnectionOpener` : poignée de main TCP sur le port 443, délai 900 ms ; un refus de connexion prouve aussi que l'hôte répond.
- Pour chaque cible : l'écho d'abord. S'il est perdu, une connexion TCP dit si la cible est encore joignable (perte) ou non (injoignable). Une réponse lente reste une latence et non une perte. Quand l'écho échoue, la vérification TCP peut faire déborder la seconde : le tick suivant est simplement retardé (`MissedTickBehavior::Delay`).
- Repli TCP permanent pour une cible quand l'ICMP est refusé par le système (Linux sans `ping_group_range`) ou filtré (10 échos perdus de suite alors que TCP répond, seuil assez haut pour qu'un Wi-Fi qui perd 30 % des paquets ne l'atteigne presque jamais). Le temps de la poignée de main sert alors de latence. Ce repli dure jusqu'au redémarrage de l'app.
- Cibles par défaut : `1.1.1.1` et `8.8.8.8` (`list_default_targets`), sondées en parallèle à chaque seconde.
- Les tests réels de l'ICMP et du TCP ne tournent qu'à la demande : `cargo test -p infrastructure -- --ignored`.

## Moteur

- Un acteur unique, `MeasurementEngine`, possède tout l'état et reçoit des messages par un canal tokio. L'app lui envoie `Tick` chaque seconde (`MissedTickBehavior::Delay`) ; tout tourne côté Rust, jamais dans le webview.
- La fenêtre compte des `Tick` : si le système retarde l'horloge (veille, App Nap), « 15 s » devient « 15 ticks ».
- À chaque `Tick` : sondes en parallèle, enregistrement dans les fenêtres de latence et de perte, détection Hors ligne, puis calcul des trois `MetricReading` (valeur, niveau, proche du seuil) et du verdict, envoyés dans un `VerdictView` au `VerdictPresenter`.
- L'horloge passe par le port `Clock` (`SystemClock` en production, horloge manuelle en test).
- Le contexte de connexion est lu par le port `ConnectionContextReader` au premier `Tick` puis toutes les 10 secondes (`CONTEXT_REFRESH_SECONDS`) : une lecture coûte quelques millisecondes d'appels système (environ 7 ms mesurées sur macOS). Il est gardé pendant une coupure et porté par chaque `VerdictView`.

## Contexte de connexion

- Seule l'interface de la route par défaut est mesurée. Sous VPN, la mesure passe par le VPN.
- `NetdevContextReader` (crate `infrastructure`) lit les interfaces avec `netdev` sur un fil bloquant (`spawn_blocking`), puis les décrit au domaine (`InterfaceSnapshot` : support, noms, adresse IPv4 et préfixe, passerelle). Le classement est pur, dans `domain` (`ConnectionContext::classify`).
- Support : Wi-Fi, Ethernet, modem cellulaire intégré (`Wwan`), tunnel (type `Tunnel`, drapeaux point à point, ou noms `utun`, `tun`, `wg`, `ppp`, `ipsec`), autre.
- VPN : quand l'interface par défaut est un tunnel, le type affiché est celui de la première autre interface active qui a une passerelle ; sans elle, « VPN » seul.
- Partage de connexion : au moins **deux** indices parmi passerelle connue des téléphones (`172.20.10.1`, `192.168.43.1`, `192.168.42.1`, `192.168.42.129`), réseau iPhone `172.20.10.0/28`, nom d'interface de téléphone (« iPhone », « Android », « RNDIS »), adresse MAC de passerelle aléatoire (bit « administrée localement »). Jamais l'adresse IP seule. La connexion limitée s'y ajoutera avec son ticket.
- Identité d'un réseau (`NetworkIdentity`) : MAC de la passerelle + type de connexion. Sans MAC résolue, le réseau n'est pas reconnaissable (`identify` rend `None`).

## Complexité

| Opération                        | Temps                          | Espace                            |
| -------------------------------- | ------------------------------ | --------------------------------- |
| Ajout d'une seconde à la fenêtre | O(1)                           | O(1), fenêtre bornée à 15 entrées |
| p90 de la fenêtre                | O(n log n), n ≤ 15             | O(n)                              |
| Médiane des cibles               | O(k log k), k = 2              | O(k)                              |
| Gigue de la fenêtre              | O(n), n ≤ 15                   | O(n)                              |
| Fenêtre de perte (ajout, taux)   | O(1), totaux tenus à jour      | O(1), 60 entrées au plus          |
| Détection Hors ligne             | O(1)                           | O(1)                              |
| Classement du contexte           | O(i·n), i interfaces de n noms | O(i)                              |
| Hystérésis                       | O(1)                           | O(1)                              |
| Dessin de l'icône                | O(s²), s = 44 px               | O(s²)                             |
