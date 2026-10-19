# Mesure continue de latence, gigue et perte sans droits admin (ticket #3)

Date des vérifications : 2026-10-19. Sources citées en bas ; les points non vérifiés sont marqués **[non vérifié]**.

## Réponse courte

- **ICMP echo sans privilège est faisable sur les 3 OS**, avec trois mécanismes différents :
  macOS = socket `SOCK_DGRAM`/`IPPROTO_ICMP` ; Linux = idem mais soumis au sysctl `net.ipv4.ping_group_range` ; Windows = API `IcmpSendEcho2Ex` / `Icmp6SendEcho2` (iphlpapi), pas de socket.
- **Crate recommandée : `ping-async` 1.0.2** (2026-02-03) : API unique, Windows (IcmpSendEcho2Ex), macOS et Linux (sockets DGRAM), IPv6, tokio, CI sur les 3 OS. Seul bémol : petite adoption (~9,8 k téléchargements), donc à surveiller.
- **`surge-ping` 0.9.1** (2026-09-16, ~4,3 M téléchargements, MSRV 1.85) est la plus répandue et maintenue. Elle essaie DGRAM puis RAW ; sur macOS/Linux c'est correct, sur Windows elle n'a pas de branche `IcmpSendEcho` (sockets uniquement) : le fonctionnement sans admin y est **[non vérifié]**, à tester avant de la retenir.
- **Fallback obligatoire : TCP connect** (handshake chronométré) vers la cible Internet quand l'ICMP est indisponible (Linux sans `ping_group_range`, réseau filtrant l'ICMP, conteneur). Pour la passerelle, il n'y a pas de bon fallback TCP (aucun port ouvert garanti) : dégrader proprement ("segment local non mesurable").
- Précision : sur Windows l'API ne donne que des **millisecondes entières** (`RoundTripTime` est un `ULONG` en ms) ; mesurer soi-même avec un `Instant` autour de l'appel est plus fin mais inclut la latence de l'ordonnanceur. macOS/Linux : horodatage utilisateur (`Instant`), précision sub-ms mais bruitée par la charge.

## Comparaison des mécanismes

| Mécanisme | macOS | Linux | Windows | Mesure |
|---|---|---|---|---|
| ICMP non privilégié | Oui, `SOCK_DGRAM` (Go `x/net/icmp` : "Darwin et Linux" seulement) | Oui **si** `ping_group_range` couvre le GID. Défaut noyau `1 0` (personne) ; systemd >= 243 le met à `0 2147483647` ; Alpine `999 59999` | Oui, `IcmpSendEcho`/`IcmpSendEcho2Ex` (IPv4), `Icmp6SendEcho2` (IPv6), sans admin. Pas de socket DGRAM | RTT, perte, gigue. Seul mécanisme qui mesure réellement la passerelle |
| TCP connect (SYN/SYN-ACK) | Oui | Oui | Oui | RTT du handshake (~1 RTT), perte = timeout/RST. Requiert un hôte:port ouvert (443 sur un résolveur/CDN). Inutilisable pour la passerelle. Le SYN perdu est retransmis par l'OS (1 s) : imposer un timeout court |
| UDP | Oui | Oui | Oui | Sans écho serveur, pas de RTT. Seules options : DNS (UDP 53) vers un résolveur (RTT requête/réponse, inclut le temps serveur) ou QUIC/STUN. Utile en 3e choix, pas en sonde principale |
| HTTP(S) | Oui | Oui | Oui | Mesure TTFB (DNS+TCP+TLS+serveur) : bon pour le débit/"vrai" ressenti, mauvais pour latence réseau pure et gigue. Trop lourd pour 1 sonde/s (TLS, connexions, cache). Réserver aux sondes de débit/portail captif |

Détails ICMP par OS :

- **Linux** : `ping_group_range` existe depuis 2.6.39 ; défaut noyau `1 0` = aucun groupe autorisé. Plusieurs distributions systemd l'élargissent, mais cela dépend de la distribution et de la version (RHEL-like et conteneurs/rootless : cas variables). L'application doit détecter l'échec de `socket()` et basculer en TCP. Utiliser `setcap cap_net_raw` n'est pas "sans admin" pour l'utilisateur final.
- **macOS** : pas de configuration requise. **[non vérifié]** au niveau Apple/XNU primaire : confirmé indirectement par la doc Go et par le README `ping-async` (CI macOS).
- **Windows** : `IcmpSendEcho` est synchrone (bloque jusqu'au timeout) ; `IcmpSendEcho2`/`2Ex` sont asynchrones et `2Ex` permet de choisir l'adresse source (utile en multi-interface). Le pare-feu Windows peut filtrer les réponses ICMP entrantes selon le profil réseau : **[non vérifié]** en détail.
- Le noyau peut réécrire l'identifiant ICMP en DGRAM (Linux : `surge-ping` ignore l'ID ; macOS : lu dans le paquet) ; la corrélation par séquence suffit.

## Crates

| Crate | Version | Dernière MAJ | Plateformes / mécanisme | Avis |
|---|---|---|---|---|
| `ping-async` | 1.0.2 | 2026-02-03 | Win : `IcmpSendEcho2Ex`/`Icmp6SendEcho2` ; macOS/Linux : DGRAM ; tokio 1.x ; IPv6 ; statuts `Success/TimedOut/Unreachable/Unknown` ; timeout 2 s par défaut | Meilleur ajustement "sans privilège sur 3 OS". Peu adopté (9,8 k DL), 1 mainteneur |
| `surge-ping` | 0.9.1 (MSRV 1.85) | 2026-09-16 (0.9.0 : 2026-06-29) | tokio + socket2 + pnet_packet ; DGRAM puis RAW ; erreur dédiée avec conseil sysctl/setcap sur Linux ; bind par interface sous macOS/Linux | Très utilisée, maintenue. Windows : pas d'API iphlpapi, **à tester**. L'auteur signale que la charge de l'ordonnanceur biaise le RTT |
| `winping` | 0.10.1 | 2020-11-08 | Windows seulement, sans élévation | Abandonnée, remplacée par `ping-async` |
| `ping-rs` | 0.1.2 | 2023-01-07 | Windows et Linux | Pas de macOS, inactive |
| `fastping-rs` | 0.2.4 | 2023-10-05 | Sockets raw (privilèges) | À écarter |
| `tokio-ping` | 0.3.0 | 2019-09-23 | tokio ancien | À écarter |
| `pinger` | 2.2.0 | 2026-08-31 | Lance le binaire `ping` du système et parse la sortie | Fonctionne sans privilège mais fork d'un processus par sonde/flux, sortie localisée et variable selon OS ; coût batterie supérieur ; ne convient pas à ~1/s en continu |
| `socket2` | 0.6.5 | 2026-07-13 | Sockets bas niveau | Brique pour écrire soi-même DGRAM ICMP/TCP si besoin |
| `netdev` | 0.46.3 | 2026-09-13 | Interfaces et passerelle par défaut | Utile pour trouver la passerelle (hors périmètre du ticket) |

## Cibles par défaut

- Internet : `1.1.1.1` (+ `1.0.0.1`) et `8.8.8.8` (+ `8.8.4.4`), IPv6 `2606:4700:4700::1111` / `2001:4860:4860::8888`. Adresses confirmées par les docs Cloudflare et Google ; **ni l'une ni l'autre ne garantit la réponse ICMP ni l'absence de limitation de débit** : pas de promesse officielle. Prévoir 2 à 3 cibles de fournisseurs distincts et ne déclarer "Hors ligne" que si toutes échouent (sinon une cible ou un filtrage fausse le verdict). `9.9.9.9` (Quad9) comme 3e option **[non vérifié ici]**.
- Passerelle : adresse de la route par défaut ; certains routeurs limitent ou ignorent l'ICMP (priorité basse sur le plan de contrôle), ce qui gonfle la gigue/perte apparente. Comparer systématiquement les deux segments avant d'accuser un segment.
- TCP de repli : `:443` sur les mêmes IP (1.1.1.1 et 8.8.8.8 servent DoH/HTTPS) **[port ouvert non revérifié]**.

## Cadence, gigue, perte et batterie

- 1 sonde/s par cible = 2 paquets/s ; le coût réseau est négligeable. Le coût énergétique vient des réveils CPU/radio : **[estimation, non mesuré]**. Garder une seule boucle de timer (une tâche tokio, `interval` avec `MissedTickBehavior::Delay`), pas un thread par cible, grouper les 2 sondes dans le même tick, et passer à 5-10 s en veille/écran éteint/sur batterie. Wi-Fi en économie d'énergie ajoute lui-même de la latence sur les premiers paquets après inactivité : un cadencement à 1 s l'évite en partie, un cadencement lent le masque moins bien.
- Gigue : calcul sur fenêtre glissante (ex. 30 à 60 échantillons) ; perte = échantillons sans réponse dans le timeout (1 à 2 s) / total ; ne compter la perte qu'après le timeout, pas à la retardée.
- Aucun accès à des compteurs d'énergie n'a été utilisé ; à valider avec Activity Monitor / `powertop` / Windows Energy en phase de prototype.

## Risques

1. Linux sans `ping_group_range` : ICMP indisponible, perte du segment passerelle. Mitigation : détection au démarrage, bascule TCP pour Internet, message clair pour la passerelle (et documenter `sysctl -w net.ipv4.ping_group_range="0 2147483647"` comme action volontaire de l'utilisateur).
2. Windows : précision à la ms, `surge-ping` non validée ; `ping-async` peu adoptée (risque de maintenance).
3. Réseaux qui filtrent l'ICMP (entreprise, hôtels) : ne pas confondre avec "hors ligne" ; croiser avec TCP.
4. Cibles tierces peuvent limiter l'ICMP : multi-cibles + médiane.
5. Charge de l'ordonnanceur (tokio) ajoute du bruit au RTT ; acceptable pour un point de menu-bar, pas pour un diagnostic fin.
6. Sandbox/App Store macOS et Flatpak/Snap : sockets ICMP possiblement restreints **[non vérifié]**.

## Recommandation pour les tickets de décision (#11, #12)

1. Sonde principale : **ICMP echo non privilégié**, 1/s, via `ping-async` (3 OS) ; garder `surge-ping` comme alternative si un spike Windows la valide.
2. Cible Internet : liste de 2 à 3 IP (Cloudflare + Google), verdict "Hors ligne" seulement si toutes échouent ET le TCP :443 échoue aussi.
3. Passerelle : ICMP uniquement, état "indisponible" explicite sinon (pas de fallback fiable).
4. Fallback TCP connect (443) pour le segment Internet quand l'ICMP est refusé.
5. HTTP réservé au débit/portail captif (autres tickets), pas à la gigue.
6. À prototyper avant décision : (a) `ping-async` sur 3 OS en CI/VM, (b) `surge-ping` sous Windows non admin, (c) mesure de batterie à 1 Hz vs 0,2 Hz.

## Sources

- Linux `icmp(7)`, `ping_group_range` : https://man7.org/linux/man-pages/man7/icmp.7.html
- Défauts systemd/distros (fil passt-dev et forum Arch, indirect) : https://lists.passt.top/hyperkitty/list/passt-dev@passt.top/thread/YDM5QFLCZBTWCOISOZ2YGPCX2HUJLB7Q/ ; https://bbs.archlinux.org/viewtopic.php?id=258253
- Windows `IcmpSendEcho` : https://learn.microsoft.com/en-us/windows/win32/api/icmpapi/nf-icmpapi-icmpsendecho
- Windows `ICMP_ECHO_REPLY` (RTT en ms) : https://learn.microsoft.com/en-us/windows/win32/api/ipexport/ns-ipexport-icmp_echo_reply
- Go `x/net/icmp` (Darwin et Linux) : https://pkg.go.dev/golang.org/x/net/icmp
- `surge-ping` : https://github.com/kolapapa/surge-ping ; versions : https://crates.io/api/v1/crates/surge-ping/versions ; `client.rs` : https://raw.githubusercontent.com/kolapapa/surge-ping/main/src/client.rs
- `ping-async` : https://github.com/hankbao/ping-async
- Métadonnées crates (versions/dates/téléchargements) : https://crates.io/api/v1/crates/NOM
- Cloudflare 1.1.1.1 : https://developers.cloudflare.com/1.1.1.1/ip-addresses/ ; Google DNS : https://developers.google.com/speed/public-dns/docs/using
