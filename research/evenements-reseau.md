# Recevoir les changements de réseau en temps réel (ticket #7)

Date : 2026-10-19. Carte : #1. Faits uniquement, pas de code applicatif.

## Réponse courte

Les trois OS offrent des notifications push sans polling, mais à des niveaux différents :

- **Changement d'interface / adresse IP / route** : disponible partout (macOS : SCDynamicStore ou NWPathMonitor ; Windows : `NotifyIpInterfaceChange` ; Linux : netlink `RTMGRP_*` ou signaux D-Bus NetworkManager).
- **Perte / retour de connectivité "Internet"** (au-delà du lien local) : NWPathMonitor (macOS), `INetworkListManagerEvents` (Windows), `StateChanged` / `Connectivity` de NetworkManager (Linux).
- **SSID** : événement dédié sur chaque OS (CoreWLAN `ssidDidChange`, `WlanRegisterNotification`, propriété `ActiveAccessPoint` NetworkManager). Sur macOS le nom du SSID exige l'autorisation de localisation (voir risques).
- **Veille / réveil** : événement séparé, pas déduit du réseau. À brancher explicitement (NSWorkspace, `WM_POWERBROADCAST`, logind `PrepareForSleep`). Au réveil, supposer des événements réseau en rafale et dans un ordre non garanti : il faut re-mesurer, pas faire confiance au dernier état connu.
- **Rust** : pas de crate clé en main couvrant les 3 OS avec SSID et connectivité Internet. `netwatcher` / `netwatch` / `if-watch` couvrent le changement d'interface/adresse ; le reste demande `windows`, `objc2-*` / `system-configuration` et `zbus`.

**Recommandation** : traiter ces événements uniquement comme un signal "invalide l'état, passe en Mesure en cours, relance une mesure" (debounce 300 à 1000 ms, valeur à calibrer), jamais comme une source de vérité de la qualité. Détail plus bas.

## Tableau par OS

| Besoin | macOS | Windows | Linux |
|---|---|---|---|
| Interface / IP / route | `SCDynamicStore` (clés `State:/Network/Global/IPv4`, `State:/Network/Interface/.*`) ; `NWPathMonitor` (`nw_path_monitor_*`, macOS 10.14+) | `NotifyIpInterfaceChange`, `NotifyUnicastIpAddressChange` (IP Helper, Vista+) | netlink `NETLINK_ROUTE` + groupes `RTMGRP_LINK`, `RTMGRP_IPV4_IFADDR`, `RTMGRP_IPV6_IFADDR` ; D-Bus NetworkManager `PropertiesChanged` |
| Connectivité globale | `NWPathMonitor` : `status` satisfied / unsatisfied / requiresConnection ; `SCNetworkReachability` (ancien, par hôte) | `INetworkListManagerEvents::ConnectivityChanged` (COM, Vista+) | NetworkManager `StateChanged(u)`, propriété `Connectivity` (check actif, détecte portail captif) |
| SSID / Wi-Fi | CoreWLAN `CWWiFiClient.startMonitoringEvent` (`ssidDidChange`, `bssidDidChange`, `linkDidChange`, `powerDidChange`) | `WlanRegisterNotification` (source ACM : connexion, déconnexion) | NetworkManager : `ActiveAccessPoint` sur le device Wireless, ou `ActiveConnection` |
| Veille / réveil | `NSWorkspaceDidWakeNotification`, `NSWorkspaceWillSleepNotification` ; ou IOKit | `WM_POWERBROADCAST` : `PBT_APMSUSPEND`, `PBT_APMRESUMEAUTOMATIC` (envoyé à chaque reprise) | logind `PrepareForSleep(b)` : true avant suspension, false après reprise ; inhibiteur `delay` possible |
| Contexte du callback | file dispatch fournie par l'appelant (NWPathMonitor) ; run loop pour SCDynamicStore / CoreWLAN | callbacks IP Helper sérialisés ; NLM = COM | thread à soi (lecture netlink) ou boucle zbus |

## Détails vérifiés (sources primaires)

- **Windows `NotifyIpInterfaceChange`** : enregistre un callback pour les changements des interfaces IP (IPv4, IPv6 ou AF_UNSPEC). Callback sérialisé. La structure reçue est incomplète : il faut rappeler `GetIpInterfaceEntry`. `InitialNotification` envoie un callback de confirmation sans changement réel. Interdit d'appeler `CancelMibChangeNotify2` depuis le callback (interblocage). L'enregistrement ne survit pas à un redémarrage. https://learn.microsoft.com/en-us/windows/win32/api/netioapi/nf-netioapi-notifyipinterfacechange
- **Windows `INetworkListManagerEvents`** : interface COM "sink" pour l'état machine, dont la connectivité Internet ; une seule méthode `ConnectivityChanged`. Vista+, desktop. https://learn.microsoft.com/en-us/windows/win32/api/netlistmgr/nn-netlistmgr-inetworklistmanagerevents
- **Windows `WlanRegisterNotification`** : source ACM pour connexion/déconnexion. La source MSM exige la capacité `wiFiControl` (consentement localisation), sinon `ERROR_ACCESS_DENIED`. La doc recommande un repli par interrogation si aucune notification n'arrive. Ne pas l'appeler depuis un callback. https://learn.microsoft.com/en-us/windows/win32/api/wlanapi/nf-wlanapi-wlanregisternotification
- **Windows `WM_POWERBROADCAST`** : `PBT_APMRESUMEAUTOMATIC` est envoyé à chaque reprise ; `PBT_APMRESUMESUSPEND` s'y ajoute seulement si l'utilisateur agit. Pas de distinction entre états de veille. Exige une fenêtre (message pump). https://learn.microsoft.com/en-us/windows/win32/power/wm-powerbroadcast
- **NetworkManager D-Bus** : signal `StateChanged(u)` (enum NMState), propriétés `State`, `Connectivity`, `PrimaryConnection` ; signal `CheckPermissions`. La méthode `Sleep(b)` sert au suivi suspend/resume de NM lui-même, pas aux clients. https://networkmanager.dev/docs/api/latest/gdbus-org.freedesktop.NetworkManager.html
- **logind** : `PrepareForSleep(b start)` ; verrou `delay` borné par `InhibitDelayMaxUSec`. https://www.freedesktop.org/software/systemd/man/latest/org.freedesktop.login1.html
- **Implémentation de `netwatcher` 0.8.0** (lecture du source sur docs.rs) : Windows = `NotifyUnicastIpAddressChange` + `NotifyIpInterfaceChange` (AF_UNSPEC), puis relecture des interfaces et diff ; Linux = socket netlink `NETLINK_ROUTE` sur les groupes LINK / IPV4_IFADDR / IPV6_IFADDR (pas de groupe de table de routage) ; un module `watch_route` utilise un socket `AF_ROUTE` (probablement macOS/BSD, non confirmé par la page). https://docs.rs/crate/netwatcher/latest/source/src/

## Non vérifié à la source (à confirmer avant d'implémenter)

- **Pages Apple** (`NWPathMonitor`, `CWWiFiClient`, `NSWorkspace`) : doc rendue en JavaScript, le fetch n'a renvoyé que le titre. Les éléments macOS ci-dessus (statuts du chemin, `ssidDidChange`, localisation exigée pour lire le SSID depuis macOS 10.15, NWPathMonitor depuis 10.14) viennent de connaissances générales : à relire sur https://developer.apple.com/documentation/network/nwpathmonitor et https://developer.apple.com/documentation/corewlan/cwwificlient .
- **Latence de notification** : aucun des OS ne documente de délai garanti et rien n'a été mesuré ici. Attendu (non mesuré) : quasi immédiat pour netlink / IP Helper (événements noyau), plus tardif pour la connectivité Internet (NLM, NM `Connectivity`) qui dépend d'un test actif du système. À mesurer en spike sur les 3 OS.
- **Comportement pendant la veille** (rafale, doublons, état intermédiaire au réveil) : non documenté de façon fiable ; supposer doublons et désordre.

## Crates et versions (crates.io / docs.rs, 2026-10-19)

| Crate | Version | Rôle | Note |
|---|---|---|---|
| `netwatcher` | 0.8.0 (2026-08-17) | liste + surveillance interfaces/adresses, Win/mac/Linux/iOS/Android/BSD | callback sur thread dédié (mac, Linux) ; pas de SSID ni de connectivité Internet |
| `netwatch` (n0) | 0.19.3 (2026-08-25) | interfaces + changements de routes, utilisé par iroh | dépend de tokio et `netdev` ; deps visibles : netlink (Linux), objc2-system-configuration (mac), windows + wmi (Windows) |
| `if-watch` | 3.2.2 (2026-03-03) | flux async d'ajout/retrait d'IP (libp2p) | macOS/iOS : repli par polling toutes les 10 s selon docs.rs, à écarter pour macOS |
| `netdev` | 0.46.3 | énumération seulement | aucune notification |
| `system-configuration` | 0.8.0 | liaisons `SCDynamicStore`, `SCNetworkReachability` | callbacks via run loop CoreFoundation |
| `objc2-system-configuration` | 0.3.2 | idem via objc2 | |
| `objc2-core-wlan` | 0.3.2 | `CWWiFiClient`, `CWEventDelegate` présents, doc 100 % | SSID/BSSID |
| `objc2-core-location` | 0.3.2 | autorisation localisation | requise pour lire le SSID |
| `objc2-network` | 0.0.0 | nom réservé, vide | **inutilisable** : pas de binding Rust de `NWPathMonitor` ; passer par `SCDynamicStore` ou un pont FFI vers l'API C `nw_path_monitor` |
| `windows` | 0.62.2 | `NotifyIpInterfaceChange`, NLM COM, WLAN API, power | features à activer par module ; noms exacts à vérifier sur https://microsoft.github.io/windows-rs/features |
| `zbus` | 6.0.0 | D-Bus (NetworkManager, logind) | feature `tokio` disponible |
| `rtnetlink` | 0.23.0 | netlink route haut niveau | `netlink-sys` 0.9.0 en bas niveau |

## Risques

1. **Pas de NWPathMonitor depuis Rust** : le binding `objc2-network` est vide. Équivalent réaliste : `SCDynamicStore` (interface/IP/route) ; la perte de connectivité Internet réelle (lien Wi-Fi up mais sans accès) n'est pas visible par ce biais et reste à détecter par la mesure elle-même.
2. **SSID macOS** : sans autorisation de localisation, `ssidDidChange` peut se déclencher sans nom lisible. Garder le déclencheur, demander la permission ou ne pas afficher le nom.
3. **Rafales d'événements** (changement de réseau, réveil) : debounce obligatoire, sinon relances de mesures en boucle.
4. **Connectivité vs qualité** : un changement de réseau n'indique ni perte ni dégradation ; une dégradation sans changement d'interface (saturation, DNS) n'émet aucun événement. Les événements ne remplacent pas les mesures périodiques.
5. **Interblocages Windows** : ne pas annuler/réenregistrer depuis un callback ; NLM exige un thread COM initialisé ; la veille exige une fenêtre pour `WM_POWERBROADCAST`.
6. **Linux hétérogène** : NetworkManager absent (systemd-networkd, connman, conteneurs) ; netlink comme socle, D-Bus NM comme enrichissement facultatif.
7. **Réveil** : l'événement de veille/réveil est indépendant ; ne pas supposer qu'un événement réseau le suit.

## Recommandation

- Socle par OS : netlink (Linux) ; `NotifyIpInterfaceChange` + `NotifyUnicastIpAddressChange` (Windows) ; `SCDynamicStore` (macOS). `netwatcher` couvre l'interface/IP en une dépendance avec diff ; à retenir si le périmètre se limite à interface/IP (vérifier son backend macOS en spike).
- Enrichissements optionnels, derrière un trait unique par plateforme : SSID (CoreWLAN / WLAN API / NM), connectivité Internet (NLM / NM `Connectivity`), veille/réveil (NSWorkspace / `WM_POWERBROADCAST` / logind).
- Un seul événement normalisé côté app ("réseau potentiellement changé", avec cause) puis debounce, état "Mesure en cours", mesure fraîche.
- Spike avant implémentation : mesurer latence et ordre des événements, y compris cycle veille/réveil, sur les 3 OS.
