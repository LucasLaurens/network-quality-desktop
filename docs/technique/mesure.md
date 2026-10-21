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

- Seule l'interface qui porte les mesures est mesurée : celle de la route vers la première cible, trouvée en connectant un socket UDP (aucun paquet n'est envoyé) puis en cherchant l'interface qui possède l'adresse locale choisie. Sous VPN, la mesure passe par le VPN ; un VPN partiel qui ne route pas les cibles n'est pas signalé.
- `NetdevContextReader` (crate `infrastructure`) lit les interfaces avec `netdev` sur un fil bloquant (`spawn_blocking`), puis les décrit au domaine (`InterfaceSnapshot` : support, nom de téléphone, adresse IPv4 et préfixe, passerelle IPv4 éventuelle et son adresse MAC). Les noms d'interface restent dans l'adaptateur ; le classement est pur, dans `domain` (`ConnectionContext::classify`).
- Support : Wi-Fi, Ethernet, modem cellulaire intégré (`Wwan`), tunnel (type `Tunnel`, drapeaux point à point, ou noms `utun`, `tun`, `wg`, `ppp`, `ipsec`), autre.
- Support : le type cellulaire (`Wwan`) est lu en premier, car un modem est aussi point à point. Tunnel : type `Tunnel` ou « virtuel propriétaire » (WireGuard, Wintun sous Windows), noms `utun`, `tun`, `wg`, `ipsec`, ou nom affiché contenant WireGuard, Wintun, TAP-Windows, OpenVPN, Tailscale, VPN. Lien point à point (PPP, PPPoE) : un VPN si une autre interface a une passerelle, sinon la connexion elle-même.
- VPN : le type affiché est celui de l'interface physique qui a une passerelle, de préférence avec une adresse MAC résolue ; avec plusieurs (Wi-Fi et Ethernet), la première listée l'emporte (limite connue). Sans interface physique visible : « VPN » seul.
- Partage de connexion : au moins **deux** indices indépendants parmi : adressage de téléphone (passerelle `172.20.10.1`, `192.168.43.1`, `192.168.42.1`, `192.168.42.129`, ou réseau iPhone `172.20.10.0/28`, comptés ensemble comme un seul indice), nom d'interface de téléphone (« iPhone », « Android », « RNDIS »), adresse MAC de passerelle aléatoire (bit « administrée localement »), connexion déclarée payante par le système (`Metering::Metered`). L'adressage seul ne suffit jamais. Un routeur maillé à adresse aléatoire configuré en `192.168.43.1` serait pris pour un partage (risque accepté).
- Identité d'un réseau (`NetworkIdentity`) : MAC de la passerelle + type de connexion, y compris sur un réseau IPv6 seul. Sans MAC résolue, le réseau garde son nom générique mais n'est pas reconnaissable d'une fois sur l'autre (`identify` rend `None`).
- Après une coupure, le contexte est relu à chaque seconde : on peut revenir sur un autre réseau. La lecture se fait en même temps que les sondes.
- Connexion limitée (`Metering`) : `Metered` (payante au volume), `Constrained` (données réduites demandées par l'utilisateur), `Unmetered`, `Unknown`. `is_limited` vaut toujours pour un partage de connexion reconnu (macOS dit « libre » pour un partage Android), et pour un modem cellulaire quand le système ne dit rien.
- Changement de réseau : le port `NetworkChangeWatcher` envoie `EngineMessage::NetworkChanged` ; le moteur relit le contexte au tick suivant (une rafale d'événements dans la même seconde coûte une seule lecture). Il compare au dernier réseau vu, même après un instant sans route (passer d'un Wi-Fi à un autre en traverse un). S'il voit un autre réseau (`is_same_network` : type, VPN, MAC et adresse de la passerelle quand les deux lectures les connaissent), il repart de zéro avec la cause « Nouveau réseau ». Ce qu'on apprend plus tard du même lien ne compte pas : MAC de passerelle enfin résolue, connexion limitée, partage de connexion reconnu grâce à un indice arrivé ensuite. La relecture périodique fait de même sans événement. Quand une lecture du même réseau ne connaît pas la passerelle, celle de la lecture précédente est gardée (`complete_with`) : une autre box à la même adresse IP est reconnue dès que sa MAC se lit.

## Tronçon local et attribution

- Le moteur pinge la passerelle chaque seconde par le port `GatewayProbe` (`IcmpGatewayProbe`, même émetteur ICMP que les cibles), en même temps que les cibles, **sans repli TCP** : une connexion TCP à une box ne dit rien de fiable sur le Wi-Fi. Pas de ping pour un modem cellulaire (pas de box) ni sans adresse de passerelle.
- `GatewayWindow` (domaine) garde latence p90 sur 15 s et perte sur 60 s, comme le tronçon Internet. Le tronçon local est « non mesurable » (`Unmeasurable`) quand le système refuse le ping, quand la passerelle n'a répondu à aucun ping de la fenêtre (une box réglée pour ignorer le ping ; un lien vraiment coupé couperait aussi Internet), ou sans passerelle visible.
- Le verdict reste celui du tronçon Internet. Quand il n'est pas Solide, `Fault::attribute` dit où : lien local mauvais (p90 > 30 ms ou perte > 1 %) → lien local ; lien local bon → opérateur, ou réseau mobile en partage de connexion ; modem intégré → réseau mobile ; non mesurable → indéterminé. Un lien local mauvais avec un Internet bon ne change rien.
- Hors ligne : sans route, « Aucun réseau connecté » ; si la passerelle a répondu à la dernière seconde, « La box répond, mais Internet ne répond plus » ; sinon « Aucun serveur ne répond ».
- Le chemin du popover (`PathDisplay`) vient du même `VerdictView` : latence de chaque tronçon, tronçon en cause marqué par un mot.

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
