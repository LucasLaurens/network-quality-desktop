# Détails Wi-Fi et permission de localisation (ticket #5)

Date des vérifications : 2026-10-19.

## Réponse courte

- **macOS** : tout se lit via `CWInterface` (crate `objc2-core-wlan`) : RSSI, bruit, débit, canal, bande 2,4/5/6 GHz. Seuls **SSID et BSSID** sont conditionnés à l'autorisation Location Services (depuis Sonoma 14). Refus : `ssid()` / `bssid()` renvoient `nil` (`None` côté Rust), les autres valeurs restent lisibles (à confirmer par test, voir Risques).
- **Windows** : `windows` crate, module `Win32::NetworkManagement::WiFi`. RSSI, débit, canal, SSID, BSSID viennent de `WlanQueryInterface`. Depuis le changement de confidentialité Wi-Fi/localisation (Windows 11 24H2), `wlan_intf_opcode_current_connection` renvoie `ERROR_ACCESS_DENIED` si la localisation précise est refusée, donc SSID, BSSID et débit TX/RX tombent. `wlan_intf_opcode_rssi` et `wlan_intf_opcode_channel_number` ne figurent pas dans la liste des appels bloqués.
- **Linux** : NetworkManager via D-Bus (`zbus`) : SSID, fréquence (donc bande + canal), force en %, bitrate. **Aucune permission de localisation.** Limites : pas de dBm ni de bruit via NM.
- Le repli « Wi-Fi (nom masqué) » + segmentation par MAC de la passerelle est justifié sur macOS et Windows. Sur ces deux OS le BSSID est aussi masqué, donc la clé de segmentation doit être la MAC de la **passerelle** (ARP/table de voisinage), pas le BSSID.

## Tableau par OS

| Donnée | macOS (CoreWLAN) | Windows (WLAN API) | Linux (NetworkManager D-Bus) |
|---|---|---|---|
| SSID | `CWInterface.ssid()` -> `Option<NSString>`. **Location requis** | `WLAN_CONNECTION_ATTRIBUTES.wlanAssociationAttributes.dot11Ssid` via `wlan_intf_opcode_current_connection`. **Location requis (24H2)** | `AccessPoint.Ssid` (`ay`) |
| BSSID | `bssid()` -> `Option<NSString>`. **Location requis** | `dot11Bssid` (même appel). **Location requis** | `AccessPoint.HwAddress` (`s`) |
| RSSI (dBm) | `rssiValue()` (`NSInteger`, 0 en cas d'erreur) | `wlan_intf_opcode_rssi` (`LONG`, dBm). Pas dans la liste bloquée | Non : `Strength` est un pourcentage (`y`). dBm via nl80211 / `iw` / wpa_supplicant |
| Bruit (dBm) | `noiseMeasurement()` | Non exposé (WLAN_STATISTICS partiel, à explorer) | Non exposé par NM (nl80211 survey) |
| Débit PHY | `transmitRate()` (`f64`, Mb/s) | `ulTxRate` / `ulRxRate` (Kb/s) dans l'association. **Bloqué sans localisation** | `Device.Wireless.Bitrate` (`u`, Kb/s) ; `AccessPoint.MaxBitrate` |
| Canal | `wlanChannel()?.channelNumber()` | `wlan_intf_opcode_channel_number` (`ULONG`) | Dérivé de `AccessPoint.Frequency` (MHz) |
| Bande | `wlanChannel()?.channelBand()` : `Band2GHz`, `Band5GHz`, `Band6GHz`, `BandUnknown` (feature `CoreWLANTypes`) | Pas de champ dans `WLAN_ASSOCIATION_ATTRIBUTES`. Le canal seul est ambigu en 6 GHz (numéros recoupant la 5 GHz). `WlanGetNetworkBssList` donne la fréquence mais est bloqué sans localisation | Dérivée de `Frequency` : 2400-2500 = 2,4 ; 5150-5895 = 5 ; 5925-7125 = 6 |
| Largeur de canal | `channelWidth()` | Non | `AccessPoint.Bandwidth` (MHz) |
| Permission localisation | Oui (SSID/BSSID) | Oui (24H2, voir plus bas) | Non |
| Valeur si refusée | `nil` / `None` | `ERROR_ACCESS_DENIED` | n/a |

## macOS

### Lecture (objc2-core-wlan)

- Types : `CWWiFiClient` (interface par défaut) -> `CWInterface` -> `CWChannel`. Docs : https://docs.rs/objc2-core-wlan/latest/objc2_core_wlan/struct.CWInterface.html et https://docs.rs/objc2-core-wlan/latest/objc2_core_wlan/struct.CWChannel.html
- Toutes les méthodes sont `unsafe`. `ssid`, `bssid`, `wlanChannel` renvoient `Option<Retained<...>>` ; `rssiValue`, `noiseMeasurement` renvoient `NSInteger` (0 si erreur) ; `transmitRate` renvoie `c_double`.
- Features Cargo à activer : `CWInterface`, `CWChannel`, `CWWiFiClient`, `CoreWLANTypes` (pour `CWChannelBand`). `Band6GHz` existe : https://docs.rs/objc2-core-wlan/latest/objc2_core_wlan/struct.CWChannelBand.html
- `nil` pour `ssid` a deux causes : permission absente, ou interface non associée (ou SSID non décodable en UTF-8/WinLatin1).

### Permission

- Apple (DTS + équipe CoreWLAN, rdar 108641482) : depuis Sonoma, l'accès à `ssid` exige que Location Services soit activé et que l'app soit autorisée. Le commentaire de `CWInterface.h` le dit. Source : https://developer.apple.com/forums/thread/732431
- Aucune entitlement ne contourne cette règle (fil forum managed devices : https://developer.apple.com/forums/thread/833749).
- Demande : `CLLocationManager` (crate `objc2-core-location` : `requestWhenInUseAuthorization`, `requestAlwaysAuthorization`, `authorizationStatus`, delegate derrière la feature `CLLocationManagerDelegate`). Statuts : notDetermined / restricted / denied / authorized.
- Clé Info.plist macOS : `NSLocationUsageDescription` (macOS 10.14+, **obligatoire** si l'app accède à la localisation) : https://developer.apple.com/tutorials/data/documentation/bundleresources/information-property-list/nslocationusagedescription.md . `NSLocationWhenInUseUsageDescription` est la clé famille iOS ; un fil DTS réussit avec `NSLocationAlwaysAndWhenInUseUsageDescription` + `requestAlwaysAuthorization`. Mettre `NSLocationUsageDescription` et, par prudence, la variante WhenInUse.
- Tauri 2 : créer `src-tauri/Info.plist`, fusionné avec le plist généré ; les entitlements passent par `bundle.macOS.entitlements` dans `tauri.conf.json` (si sandbox/signature : entitlement `com.apple.security.personal-information.location`). Source : https://v2.tauri.app/distribute/macos-application-bundle/
- Le `CLLocationManager` doit être créé et sa boucle d'exécution (run loop) tournée sur le thread principal ; la réponse arrive au delegate (`locationManagerDidChangeAuthorization`). Les fils DTS 748518 et 769950 signalent des cas de SSID `nil` malgré l'autorisation quand la run loop est bloquée ou le process n'est pas une vraie app GUI.

### Comportement quand c'est refusé

- `ssid()`/`bssid()` = `None`. App affiche « Wi-Fi (nom masqué) ».
- RSSI, bruit, débit, canal, bande : lecture non gatée d'après les retours de la communauté, **non confirmé par une source Apple** (voir Risques).

## Windows

### Lecture (crate `windows` 0.62.2, feature `Win32_NetworkManagement_WiFi`)

- `WlanOpenHandle`, `WlanEnumInterfaces`, `WlanQueryInterface`, `WlanFreeMemory`, `WlanCloseHandle`.
- `wlan_intf_opcode_current_connection` -> `WLAN_CONNECTION_ATTRIBUTES` (SSID, BSSID, `wlanSignalQuality` 0-100, `ulRxRate`, `ulTxRate`, type PHY).
- `wlan_intf_opcode_rssi` -> `LONG` (dBm). `wlan_intf_opcode_channel_number` -> `ULONG`.
- Liste des opcodes : https://learn.microsoft.com/en-us/windows/win32/api/wlanapi/nf-wlanapi-wlanqueryinterface
- `wlanSignalQuality` n'est pas du dBm ; la formule documentée par Microsoft va de -100 dBm (0) à -50 dBm (100). Préférer l'opcode RSSI.
- Alternative SSID sans scan : WinRT `WlanConnectionProfileDetails.GetConnectedSsid` (recommandé par Microsoft dans la page ci-dessous). Non vérifié si elle reste fonctionnelle quand la localisation est refusée.

### Permission

Source unique : https://learn.microsoft.com/en-us/windows/win32/nativewifi/wi-fi-access-location-changes (mise à jour 2025-06). Points :

- Les API donnant accès aux BSSID exigent que l'utilisateur ait autorisé l'app à accéder à la localisation précise (Paramètres > Confidentialité et sécurité > Localisation ; niveau appareil, utilisateur ou app).
- Sans consentement : invite système **une fois par app** au premier appel bloqué (si le process tourne dans le contexte utilisateur et hors `C:\Windows\System32`).
- `ERROR_ACCESS_DENIED` pour : `WlanGetAvailableNetworkList`, `WlanGetNetworkBssList`, `WlanScan`, et `WlanQueryInterface` **uniquement avec `wlan_intf_opcode_current_connection`**. `WlanRegisterNotification` avec `WLAN_NOTIFICATION_SOURCE_MSM` exige la capability `wiFiControl`.
- WinRT : `WiFiAdapter.RequestAccessAsync` renvoie `DeniedBySystem` ; les autres API `Windows.Devices.WiFi` lèvent « Access is denied ».
- Demander proprement : appeler `WiFiAdapter.RequestAccessAsync` ou `Geolocator.RequestAccessAsync` au moment d'une action utilisateur ; lire l'état avec `AppCapability.CheckAccess` (capability `wiFiControl`, retour `UserPromptRequired` si une invite va s'afficher) ; écouter `AppCapability.AccessChanged` ; en cas de refus, ouvrir `ms-settings:privacy-location`.
- Ces appels apparaissent dans l'icône « localisation en cours d'utilisation » ; limiter la fréquence de polling.
- Le doc parle de la « fall 2024 feature release » (Windows 11 24H2) sans citer 24H2 nommément ; l'équivalence 24H2 est une déduction de calendrier.

### Comportement quand c'est refusé

- `WlanQueryInterface(current_connection)` -> `ERROR_ACCESS_DENIED` : plus de SSID, BSSID, ni débit TX/RX.
- RSSI et canal : non listés comme bloqués, donc probablement lisibles (à tester sur 24H2).

## Linux

- Pas de permission de localisation ni d'API équivalente.
- D-Bus (`org.freedesktop.NetworkManager`) : `Device.Wireless.ActiveAccessPoint` (chemin) puis propriétés de `AccessPoint` : `Ssid` (`ay`), `Frequency` (`u`, MHz), `HwAddress` (BSSID), `Strength` (`y`, %), `MaxBitrate` (`u`, Kb/s), `Bandwidth` (`u`, MHz). `Device.Wireless.Bitrate` (`u`, Kb/s) = débit courant. Docs : https://networkmanager.dev/docs/api/latest/gdbus-org.freedesktop.NetworkManager.Device.Wireless.html et https://networkmanager.dev/docs/api/latest/gdbus-org.freedesktop.NetworkManager.AccessPoint.html
- Canal calculé depuis la fréquence (2,4 GHz : (f-2407)/5 ; 5 GHz : (f-5000)/5 ; 6 GHz : (f-5950)/5).
- Pas de dBm ni de bruit dans NM. Si requis : nl80211 (crate `neli`/`wl-nl80211`) ou wpa_supplicant D-Bus ; hors périmètre à ce stade.
- Hôtes sans NetworkManager (iwd, systemd-networkd seul) : lecture impossible par cette voie, afficher « Wi-Fi » sans détails.
- Crate `zbus` 6.0.0 (crates.io, 2026-10-16).

## Crates et versions (crates.io, 2026-10-19)

| Crate | Version stable |
|---|---|
| `objc2` | 0.6.5 |
| `objc2-core-wlan` | 0.3.2 |
| `objc2-core-location` | 0.3.2 |
| `objc2-foundation` | 0.3.2 |
| `windows` | 0.62.2 |
| `zbus` | 6.0.0 |

## Risques

1. **macOS : bruit/RSSI/canal sans permission non confirmé par Apple.** Les retours sont communautaires. À valider par un test manuel (autorisation refusée) avant de promettre « les métriques radio restent visibles ».
2. **macOS, mode `tauri dev`** : le binaire n'est pas un `.app` ; l'invite de localisation et la lecture du SSID peuvent ne pas fonctionner sans bundle avec `Info.plist`. Non vérifié (aucune source Tauri trouvée) ; tester sur un build `tauri build` signé.
3. **macOS Sequoia 15.1** : cas de SSID `nil` même autorisé, cause non résolue (fil 769950) ; garder le repli « nom masqué ».
4. **Windows : bande 6 GHz** non déductible du seul numéro de canal ; sans localisation on n'a ni fréquence ni BSS. Afficher « bande inconnue » dans ce cas.
5. **Windows : prise de poignée** (`WlanOpenHandle`) suivie d'appels fréquents -> icône localisation affichée en permanence. Throttler (ex. toutes les 5 s au plus).
6. **Windows : 24H2 / invite** : le doc est rédigé au futur et a été en préversion ; vérifier le comportement réel sur une machine 24H2.
7. **Linux** : dBm et bruit absents de NM ; fonctionnalité partielle selon l'hôte.
8. **Pas d'API stable côté crates Rust prête à l'emploi** : tout passe par du FFI `unsafe` ; isoler derrière un trait par OS.

## Recommandation

1. Définir un modèle commun avec champs optionnels explicites : SSID, BSSID, RSSI dBm, bruit dBm, débit Mb/s, canal, bande ; un état de permission (`Granted` / `Denied` / `NotDetermined` / `NotApplicable`).
2. macOS : `objc2-core-wlan` + `objc2-core-location`, `NSLocationUsageDescription` dans `src-tauri/Info.plist`, entitlement localisation si sandbox/signature ; demander l'autorisation lors d'une action explicite (bouton dans l'UI), pas au lancement.
3. Windows : `windows` crate ; ouvrir une seule poignée WLAN ; lire RSSI/canal via les opcodes non bloqués ; gérer `ERROR_ACCESS_DENIED` en passant à « Wi-Fi (nom masqué) » ; proposer un lien `ms-settings:privacy-location`.
4. Linux : `zbus`, pas de permission ; afficher les champs disponibles uniquement.
5. Segmenter l'historique par MAC de la passerelle sur tous les OS quand le BSSID est masqué.
6. Avant d'implémenter : deux tests manuels courts (macOS refusé : RSSI/canal lisibles ? ; Windows 24H2 refusé : RSSI/canal lisibles ?).
