# Détection du type d'interface, VPN, métrique, hotspot et MAC passerelle

Ticket #6, carte #1. Recherche du 2026-10-19. Faits uniquement, pas de code applicatif.

## Réponse courte

- Aucun signal unique ne suffit. Il faut **empiler** : (1) type d'interface via `netdev`, (2) coût/metered via l'API native de l'OS, (3) heuristique réseau (plage IP / passerelle / OUI de la MAC passerelle) pour le hotspot.
- **Type d'interface (Ethernet / Wi-Fi / cellulaire intégré)** : fiable avec `netdev` 0.46.3 sur les 3 OS (`if_type`, `is_physical`, `default`).
- **Metered** : seul signal « officiel » par OS : Windows `ConnectionCost.NetworkCostType`, macOS `NWPath.isExpensive`/`isConstrained`, Linux NetworkManager `Metered` (D-Bus). Aucun n'est un détecteur de hotspot à lui seul.
- **Hotspot téléphone** : pas d'API. Heuristique : iPhone = passerelle `172.20.10.1` (réseau 172.20.10.0/28) ; Android Wi-Fi = `192.168.43.1` historique (non garanti, plage variable selon version/constructeur) ; Android USB = `192.168.42.1`. À confirmer par l'OUI de la MAC passerelle (table à construire) et le metered natif quand disponible.
- **VPN** : `netdev` ne donne que `is_tun()` (heuristique de flags) et `InterfaceType::Tunnel`. Fiabilité moyenne, à compléter par les noms d'interface (`utun*`, `tun*`, `wg*`, `ppp*`) et `NM_DEVICE_TYPE_TUN/WIREGUARD` sous Linux.
- **MAC passerelle** : fournie par `netdev` (`Interface.gateway: Option<NetworkDevice>`, adresse MAC nulle si non résolue). Pas besoin d'une autre crate.
- **Recommandation** : `netdev` comme base unique cross-platform + 3 petits adaptateurs natifs pour le coût (un trait, une impl par OS) + règle d'inférence « hotspot » = metered natif OU (plage IP connue ET type Wi-Fi/USB). Prévoir un override manuel « ce réseau est un hotspot ».

## Tableau signal x OS

Fiabilité : H = haute, M = moyenne, B = basse.

| Signal | macOS | Windows | Linux |
|---|---|---|---|
| Ethernet / Wi-Fi | `netdev` via SystemConfiguration (`SCNetworkInterfaceType` : Ethernet, IEEE80211/AirPort) : **H** | `netdev` via `GetAdaptersAddresses` `IfType` (6 Ethernet, 71 Wi-Fi) : **H** | `netdev` via sysfs (`wireless/`, `type`) : **H** |
| Modem cellulaire intégré | `SCNetworkInterfaceType = WWAN` -> `Wwanpp` : **M** (peu de Mac avec modem, non testé) | `IfType` 243/244 (WWANPP/WWANPP2) ; `PhysicalMediumType = NdisPhysicalMediumWirelessWan` : **H** | `netdev` : `Wwan` si sysfs le détecte ; sinon NM `DEVICE_TYPE_MODEM` (8) ou ModemManager D-Bus : **M** |
| USB tethering | Interface Ethernet (iPhone : « iPhone USB ») : type seul **B**, plage IP **M** | Ethernet/RNDIS : type seul **B** | Ethernet (`usb0`, `enx…`, pilotes `rndis_host`/`cdc_ether`/`ipheth`) ; pilote via sysfs **M** |
| VPN | `utun*` + flags P2P : `is_tun()` **M** | `IF_TYPE_TUNNEL` (131) ou adaptateur virtuel (`HardwareInterface` faux) : **M** | `tun*`/`wg*`/`ppp*`, NM `DEVICE_TYPE_TUN` (16) / `WIREGUARD` (29) : **M** |
| Interface par défaut | `netdev::get_default_interface()` : **H** | idem : **H** | idem : **H** |
| MAC passerelle | `netdev` `gateway` : **H** une fois la passerelle jointe, sinon zéros | idem : **H** | idem (netlink/ARP) : **H** |
| Metered natif | `NWPath.isExpensive` (+ `isConstrained` = Low Data Mode) : **H** attendu pour cellulaire/hotspot (page Apple non récupérée par fetch, à valider sur Mac réel) | `ConnectionProfile.GetConnectionCost()` : `NetworkCostType` Unrestricted/Fixed/Variable, `Roaming`, `OverDataLimit`, `ApproachingDataLimit` : **H** | NetworkManager `Metered` : YES/NO/GUESS_YES/GUESS_NO/UNKNOWN : **M** (souvent UNKNOWN/GUESS) |
| Hotspot iPhone | Wi-Fi + passerelle `172.20.10.1` (+ `isExpensive`) : **H** | idem ; le profil n'est pas toujours marqué Variable : **M** | idem, sans signal natif fiable : **M** |
| Hotspot Android | Wi-Fi + `192.168.43.1` (historique, plage variable) : **B-M** | idem : **B-M** | idem : **B-M** |

## Détails par signal

### `netdev` (crate recommandée)

- v0.46.3 (2026-09-13), MIT, mainteneur shellrow. Lecture du code source du paquet : struct `Interface` avec `if_type: InterfaceType`, `mac_addr`, `ipv4/ipv6`, `flags`, `oper_state`, `gateway: Option<NetworkDevice>`, `dns_servers`, `default: bool`, `mtu`, débit, stats.
- `InterfaceType` couvre `Ethernet`, `Wireless80211`, `Wwan`, `Wwanpp`, `Wwanpp2`, `GenericModem`, `Tunnel`, `Ppp`, `Bridge`, `PeerToPeerWireless` (AWDL), `Loopback`.
- Helpers : `Interface::default()`, `is_tun()` (heuristique : up + point-à-point + non broadcast + non loopback ; la doc dit elle-même qu'elle n'est pas exhaustive), `is_physical()` (type + flags + OUI de MAC virtuelles connues).
- Sources par OS : macOS = SystemConfiguration (`objc2-system-configuration`), Windows = `windows-sys` (`GetAdaptersAddresses`/`GetIfEntry2`), Linux = sysfs + netlink. Pas de dépendance à NetworkManager.
- Champ `gateway` : « adresse MAC nulle » si la passerelle est connue mais non résolue. Le code ne prévoit ni metered ni détection hotspot.

### Autres crates

| Crate | Version | Verdict |
|---|---|---|
| `if-addrs` | 0.15.0 (2026-02) | Seulement noms/IP/flags. Insuffisant (ni type, ni MAC, ni passerelle). |
| `default-net` | 0.22.0 (2024-02) | Ancêtre de `netdev` (même auteur), sans release depuis 2024. À écarter. |
| `network-interface` | 2.0.5 (2026-01) | Liste d'interfaces, MAC ; ni type ni passerelle (vérifié via description, pas via le code). |
| `mac_address` | 1.2.1 (2026-10) | MAC locale uniquement, inutile pour la passerelle. |
| `system-configuration` | 0.8.0 | Accès SCNetworkReachability (flag `IsWWAN`) comme repli macOS. |
| `windows` | 0.62.2 | Projection WinRT : `Windows.Networking.Connectivity` (ConnectionCost, ConnectionProfile). |
| `objc2-network` | 0.0.0 (coquille vide) | **Pas de binding `NWPath` utilisable.** Voir risques. |
| `zbus` | 6.0.0 | Client D-Bus pur Rust pour NetworkManager / ModemManager. |

### Windows

- `ConnectionCost` (Windows 10+) vient de `ConnectionProfile.GetConnectionCost()`. Propriétés : `NetworkCostType`, `Roaming`, `OverDataLimit`, `ApproachingDataLimit`, `BackgroundDataUsageRestricted`. La doc recommande d'évaluer chaque indicateur plutôt que le seul type de coût.
- `MIB_IF_ROW2` : `Type` (IANA : 6, 71, 131, 243, 244), `PhysicalMediumType` (WirelessWan=8, Native802_11=9, 802_3=14), `ConnectionType`, `HardwareInterface`, `EndPointInterface` (doc : positionné par des appareils type smartphone qui n'offrent pas de connectivité externe).
- Mobile Broadband API (`Windows.Networking.NetworkOperators`, `MbnInterface`) : à réserver aux détails modem (opérateur, signal). Pour « cellulaire ou non », `IfType` 243/244 suffit.
- Metered automatique via DHCP `ANDROID_METERED` (option 43) : Microsoft le liste dans MS-DHCPE, mais un forum rapporte que Windows ne le suit pas. Non confirmé, ne pas s'y fier.

### macOS

- Type via SystemConfiguration (déjà dans `netdev`). `NWPath.isExpensive` = signal officiel « cellulaire ou partage de connexion », `isConstrained` = Low Data Mode. Pas de binding Rust mûr (`objc2-network` vide). Options : petit pont Swift/Obj-C, FFI vers l'API C `nw_path_*`, ou repli `SCNetworkReachability` (`IsWWAN`, couvre le cellulaire mais pas un hotspot Wi-Fi). Non testé ici.
- iPhone Personal Hotspot : passerelle `172.20.10.1`, réseau 172.20.10.0/28 (une source web, `speedtesthq.com`, cohérente avec le ticket ; une autre page citait 192.168.0.1/1.1, jugée peu fiable).
- iOS ne reconnaît pas l'option DHCP `ANDROID_METERED` (OpenWrt) : le metered d'un hotspot iPhone ne peut pas passer par le DHCP.

### Linux

- NetworkManager : `NMMetered` 0..4 (UNKNOWN, YES, NO, GUESS_YES, GUESS_NO) et `NMDeviceType` (ETHERNET=1, WIFI=2, MODEM=8, TUN=16, WIREGUARD=29) via D-Bus (`org.freedesktop.NetworkManager`, crate `zbus`).
- ModemManager (`org.freedesktop.ModemManager1`, interface `…Modem`) : seulement pour détails modem. Pour savoir si l'interface par défaut est un modem, NM `DEVICE_TYPE_MODEM` ou sysfs suffisent.
- Sans NetworkManager (systemd-networkd, serveur) : aucun signal metered, retour à l'heuristique réseau.
- USB tethering Android : interface Ethernet, passerelle `192.168.42.1` (busuk.org, forum Zebra, cohérents). Le pilote sysfs est un bon indice.

### Hotspot Android

- Wi-Fi : `192.168.43.1` historique, non garanti (les versions récentes / constructeurs peuvent utiliser d'autres sous-réseaux). 192.168.49.1 non confirmé par la recherche (souvent associé à Wi-Fi Direct).

## Risques et limites

1. **Faux positifs RFC1918** : `192.168.43.x` ou `172.20.10.x` existent aussi sur des LAN normaux. Combiner avec type d'interface, OUI et metered natif.
2. **VPN en route par défaut** : l'interface par défaut devient `utun`/`tun`/`wg` ; il faut remonter à l'interface physique sous-jacente pour typer la connexion (non fourni par `netdev`).
3. **macOS `NWPath`** : pas de binding Rust mûr ; coût d'intégration réel et non testé.
4. **MAC passerelle** : zéros tant que le voisin n'est pas résolu ; derrière VPN ou en 4G la passerelle peut être un relais virtuel.
5. **Cellulaire intégré sur Linux/macOS** : rare, difficile à tester sans matériel.
6. La page Apple `isExpensive` n'a pas pu être lue ; comportement hotspot à valider sur Mac réel.

## Recommandation

1. `netdev` (0.46.x) comme source unique cross-platform : type, MAC, passerelle, MAC passerelle, interface par défaut.
2. Un trait « coût de connexion » avec trois implémentations : Windows (crate `windows`, `ConnectionCost`), Linux (`zbus` + NetworkManager `Metered`, `Unknown` si absent), macOS (pont natif `NWPath`, spike requis).
3. Hotspot inféré : metered natif vrai ET interface Wi-Fi/USB ; sinon plage IP connue ET passerelle correspondante ; sinon override manuel. Jamais l'IP seule.
4. Désactiver les tests de débit si metered, hotspot inféré ou cellulaire.
5. Spike avant dev : binding `NWPath` macOS ; tests réels iPhone USB / Wi-Fi et Android sur les 3 OS.

## Sources

- netdev 0.46.3 (code source du paquet crates.io) : https://docs.rs/netdev/0.46.3/netdev/
- Windows ConnectionCost : https://learn.microsoft.com/en-us/uwp/api/windows.networking.connectivity.connectioncost
- Windows MIB_IF_ROW2 : https://learn.microsoft.com/en-us/windows/win32/api/netioapi/ns-netioapi-mib_if_row2
- NetworkManager types D-Bus : https://networkmanager.dev/docs/api/latest/nm-dbus-types.html
- ModemManager : https://www.freedesktop.org/software/ModemManager/doc/latest/ModemManager/
- DHCP `ANDROID_METERED` : https://openwrt.org/docs/guide-user/network/wifi/android-meteredconnection
- Plages hotspot : https://speedtesthq.com/guides/network/192-168-43-1 , https://busuk.org/ip/192-168-42-1/
- Apple NWPath.isExpensive : https://developer.apple.com/documentation/network/nwpath/isexpensive (non lisible via fetch)
- Versions crates : API crates.io, relevé du 2026-10-19.
