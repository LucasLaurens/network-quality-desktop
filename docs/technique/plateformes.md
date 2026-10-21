# Plateformes

Adaptateurs par système, permissions, barre système, veille et tâche de fond.

## Windows : manifeste de l'application

Tauri a besoin des Common Controls v6, déclarés dans un manifeste Windows (`src-tauri/windows-app-manifest.xml`). Par défaut, Tauri ne lie ce manifeste qu'au binaire de l'app : les binaires de test refusent alors de démarrer (`STATUS_ENTRYPOINT_NOT_FOUND`, code `0xc0000139`). Le script de build (`src-tauri/build.rs`) désactive le manifeste de Tauri et lie le même fichier à toutes les cibles, tests compris, avec le linker MSVC.

## Linux : droit de ping

La sonde ICMP sans privilège exige que `net.ipv4.ping_group_range` inclue le groupe de l'utilisateur (c'est le cas par défaut sur les distributions récentes basées sur systemd). Sinon, l'app mesure Internet par connexion TCP sur le port 443, et le tronçon local devient « non mesurable » : le popover le dit, avec le réglage à faire. Pour Internet, la latence reste juste, et la perte compte les connexions qui échouent, ce qui est moins fin qu'une perte de paquets ICMP.

## Contexte de connexion

- `netdev` donne sur les trois systèmes le type d'interface, l'interface par défaut, sa passerelle et l'adresse MAC de celle-ci (zéros quand elle n'est pas résolue).
- macOS : la table ARP est filtrée par la protection « Réseau local ». Un binaire lancé depuis un terminal ou un test peut ne voir aucune adresse MAC alors que `arp` en voit une dans le terminal : le réseau garde son nom générique, mais il n'est plus reconnaissable et l'indice « adresse aléatoire » du partage de connexion manque. L'app déclare `NSLocalNetworkUsageDescription` (`src-tauri/Info.plist`, fusionné dans le bundle) pour que macOS demande l'autorisation. À vérifier sur l'app empaquetée.
- Windows : le nom système d'une interface est un GUID ; les VPN sont reconnus par leur type ou par leur description (WireGuard, Wintun, TAP-Windows, OpenVPN, Tailscale). Un VPN au nom inconnu, présenté comme une carte Ethernet, passe pour une connexion Ethernet.

## Connexion limitée et changements de réseau

- macOS : `nw_path_monitor` de Network.framework par FFI direct (`block2`, `dispatch2`, sans Swift), dans `crates/infrastructure/src/metering/macos.rs`. Le moniteur tourne sur sa propre file pour toute la vie de l'app et garde la dernière réponse (`isExpensive` puis `isConstrained`). La toute première lecture répond « inconnu » : la première mise à jour arrive un instant après.
- Windows : `ConnectionCost` du profil de connexion Internet (WinRT, crate `windows` déjà présente via Tauri) : `Fixed` et `Variable` sont limitées.
- Linux : propriété `Metered` de NetworkManager sur le bus système (crate `dbus`) ; les réponses devinées par NetworkManager comptent. Sans NetworkManager, la réponse est « inconnue » et le domaine se rabat sur le type de lien.
- Changements : `netwatcher` (netlink sous Linux, IP Helper sous Windows, socket de routage sous macOS) sur un fil dédié. Il signale aussi le bruit (interfaces virtuelles, adresses IPv6 temporaires) : le moteur décide en comparant le réseau relu. Sous Linux, il suit les interfaces et les adresses mais pas la table de routage : une route par défaut qui bascule sans changement d'adresse n'est vue qu'à la relecture suivante (10 s au plus).
- Essai réel sous macOS (partage de connexion Android, reconnu à l'option DHCP `ANDROID_METERED`) : `isExpensive` vaut `false`. macOS ne signale pas un partage Android comme payant ; seul le partage iPhone l'est (non vérifié faute d'iPhone). Sur ce Mac, un partage Android n'a donc qu'un indice (adresse de passerelle aléatoire, quand elle est lisible) et reste « Wi-Fi ».

## Wi-Fi

- Lu toutes les 10 secondes avec le contexte, par le port `WifiReader` (`SystemWifiReader`, `crates/infrastructure/src/wifi/`), sur un fil bloquant.
- macOS : `CWInterface` de CoreWLAN (`objc2-core-wlan`) : signal en dBm, débit, canal, bande 2,4 / 5 / 6 GHz, lus dans un `autoreleasepool` (le fil bloquant n'en a pas). Le nom (SSID) demande l'accès à la localisation (`CLLocationManager`, `objc2-core-location`, clés `NSLocationUsageDescription` et `NSLocationWhenInUseUsageDescription` dans `src-tauri/Info.plist`). Vérifié sur macOS 26 avec un binaire de test sans autorisation : nom absent, mais signal (−59 dBm), débit (325 Mbit/s), canal 149 et bande 5 GHz lisibles. Les builds signés portent l'entitlement `com.apple.security.personal-information.location` (`src-tauri/entitlements.plist`) ; macOS 11 minimum (lecture de l'autorisation par instance).
- Windows : WLAN API (`WlanQueryInterface`). Le signal en dBm et le canal ne figurent pas parmi les appels bloqués (non vérifié sur Windows 11 24H2) ; la connexion courante (nom, débit) renvoie `ERROR_ACCESS_DENIED` sans localisation depuis 24H2. La bande reste inconnue : la 6 GHz réutilise les numéros de canal de la 2,4 et de la 5 GHz.
- Linux : NetworkManager (périphérique Wi-Fi activé, point d'accès actif) : nom, fréquence (d'où bande et canal), débit. La force arrive en pourcentage, reconvertie en dBm avec l'échelle de NetworkManager (−100 dBm = 0 %, −40 dBm = 100 %, linéaire). Sans NetworkManager, pas de détail Wi-Fi.
- Demande de permission : commande `reveal_network_name`, exécutée sur le fil principal (CoreLocation répond par la boucle principale). macOS montre sa demande tant qu'elle n'a pas reçu de réponse, puis l'app ouvre les réglages de localisation ; Windows ouvre directement `ms-settings:privacy-location`.
- `make dev` : le binaire tourne hors d'un `.app`, sans `Info.plist` ; sous macOS, le nom suit l'autorisation de localisation du terminal qui a lancé l'app (non vérifié). Les valeurs radio restent lisibles. L'autorisation de l'app elle-même se vérifie sur l'app empaquetée (`make build`).

## Veille et réveil

Port `PowerEventSource` (`crates/infrastructure/src/power/`). Chaque adaptateur écoute sur son propre fil ou sa propre file, ne fait que transmettre l'avis, et attend la réponse du moteur (`EngineInbox::relay_power_event`, 1,5 s au plus) avant de laisser la veille se faire. L'event loop Tauri n'est jamais touchée.

| Système | Avis                                                                                                                   | Délai avant la veille                                      |
| ------- | ---------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------- |
| macOS   | IOKit `IORegisterForSystemPower` (`objc2-io-kit`) sur une file dispatch dédiée ; `IOAllowPowerChange` après l'écriture | jusqu'à 30 s ; la veille d'inactivité n'est jamais refusée |
| Windows | `PowerRegisterSuspendResumeNotification` en mode callback, sans fenêtre ; réveil par `PBT_APMRESUMEAUTOMATIC` seul     | environ 2 s, sans prolongation possible                    |
| Linux   | signal logind `PrepareForSleep` (crate `dbus`, bus système) avec un verrou `delay`, repris après chaque réveil         | 5 s par défaut (`InhibitDelayMaxUSec`)                     |

Points laissés ouverts par la recherche (#19), tranchés :

- **`objc2-io-kit`** (0.3.2) couvre `IORegisterForSystemPower`, `IOAllowPowerChange` et `IONotificationPortSetDispatchQueue` (features `libc`, `dispatch2`, `pwr_mgt`) : une file dispatch remplace le fil à run loop prévu, sans FFI maison.
- **Windows** : la fonction est dans la crate `windows` 0.62.2 sous les features `Win32_System_Power` et `Win32_UI_WindowsAndMessaging`. Comportement en **Modern Standby** : non vérifiable sans machine qui le gère ; le filet du trou de mesure couvre un avis absent ou tardif (matrice de tests manuels).
- **Linux** : ni `zbus` ni `logind-zbus`. La crate `dbus`, déjà utilisée pour la barre système, NetworkManager et le Wi-Fi, suffit ; `zbus` n'arrive que par `tauri-plugin-single-instance`. Sans logind (pas de systemd, conteneur), l'adaptateur le note dans le journal et le filet prend le relais.
- **Filet** : plus de 10 s d'heure murale entre deux ticks, ou pendant les sondes d'un tick, valent un réveil (voir [Mesure](mesure.md#moteur)). Il couvre aussi l'hibernation et une coupure de courant.

## Réglages

- Apparence : `AppHandle::set_theme` applique Clair ou Sombre à toutes les fenêtres (Système : `None`). Le webview suit le thème de sa fenêtre, donc les media queries `prefers-color-scheme` des tokens CSS aussi : pas de second jeu de couleurs. Non vérifié dans l'app réelle : à contrôler à la main sur les trois systèmes (barre de titre et page).
- Ouverture de session : `tauri-plugin-autostart` 2.7.0, appelé depuis Rust seulement (aucune permission front) : LaunchAgent sous macOS, clé `Run` du registre sous Windows, fichier `.desktop` dans `~/.config/autostart` sous Linux. Le réglage est appliqué au lancement, avant toute fenêtre, et à chaque enregistrement : le réglage de l'app fait foi sur l'état du système. Une version de développement (`debug_assertions`) ne s'inscrit jamais.

## Notifications

- `tauri-plugin-notification` 2.7.0, appelé depuis Rust seulement (aucune permission front). Le texte vient du catalogue, comme le popover.
- macOS : le système demande l'autorisation à la première notification de l'app empaquetée ; un binaire de développement lancé hors d'un `.app` peut n'en afficher aucune (non vérifié). Windows : notification « toast » au nom de l'app installée. Linux : serveur de notifications D-Bus du bureau.
- Non vérifié dans l'app réelle sur les trois systèmes : à contrôler à la main (provoquer une dégradation, par exemple en saturant le lien).

## Tâche de fond et App Nap

- macOS : `keep_measuring_in_background` déclare au lancement une activité `NSActivityUserInitiatedAllowingIdleSystemSleep` (`NSProcessInfo`), pour toute la vie de l'app : App Nap ne peut plus espacer les ticks quand aucune fenêtre n'est visible, et le Mac peut toujours se mettre en veille quand il est inactif.
- Mesures sur ce Mac (macOS 26), sans fenêtre visible, ticks par minute lus dans l'historique :
  - binaire de développement lancé depuis un terminal, 6 à 7 min : 60 ticks par minute avec ou sans l'activité, 11 à 12 s de CPU pour 400 s (build debug). Hors LaunchServices, App Nap ne s'est pas déclenché ;
  - app empaquetée (release), lancée par `open -g`, 10 min chacune : avec l'activité, 60 ticks par minute sur les 9 minutes complètes et 10,3 s de CPU pour 600 s (environ 1,7 % d'un cœur) ; sans l'activité, 60 ticks par minute aussi et 11,1 s de CPU.
- Conclusion : sur ce Mac, sur secteur et en 10 minutes, App Nap n'a pas ralenti l'app, avec ou sans l'activité, qui ne coûte rien de mesurable. Elle reste déclarée : App Nap dépend de conditions (batterie, inactivité plus longue, version du système) qui n'ont pas été réunies ici. À remesurer sur batterie, sur une durée plus longue, avant une version.
- Windows et Linux n'ont pas d'équivalent à App Nap pour une app de bureau.

## Icône de barre système

- L'icône est dessinée en Rust (`src-tauri/src/tray_icon.rs`), une forme par état (cinq, Hors ligne compris), avec les couleurs claires des tokens. Elle a été vérifiée lisible sur barre claire et sombre ; le gris de Hors ligne y est plus discret ; elle n'est pas redessinée quand le thème change.
- Elle porte un menu natif défini une seule fois pour les trois systèmes (`src-tauri/src/tray_menu.rs`) : état et constat (lignes grisées, réécrites quand leur texte change), « Ouvrir le tableau de bord », « Quitter ». Sous macOS et Windows, il s'ouvre au clic droit ; le clic gauche est réservé au popover.
- Le popover est ancré à l'icône par le plugin positioner : centré sous l'icône sous macOS, au-dessus sous Windows (barre des tâches en bas). Sous Linux, Tauri ne transmet pas le clic sur l'icône : il n'y a pas de popover (`HAS_POPOVER`), le menu s'ouvre à tout clic.
- Fermer le tableau de bord ne quitte jamais l'app (`ShellMode::choose_close_action`) : il est masqué en mode barre système, réduit en mode fenêtre. On quitte par le menu, ou en mode fenêtre par le bouton « Quitter » du tableau de bord (commande `quit_app`).
- Sous macOS, l'app n'a pas d'icône dans le Dock (politique d'activation `Accessory`) : c'est une app de barre des menus, le tableau de bord s'ouvre depuis la pastille.
- Sous macOS, le menu d'application de Tauri par défaut est remplacé (`src-tauri/src/app_menu.rs`) : Édition et Fenêtre seulement, pour garder ⌘C, ⌘V, ⌘W et ⌘M dans le tableau de bord, sans « Quitter » ni ⌘Q.
- Une seule instance (`tauri-plugin-single-instance`, enregistré en premier) : un second lancement ouvre le tableau de bord de l'app déjà lancée et s'arrête, sans jamais ouvrir le fichier d'historique une seconde fois.
- La fenêtre popover est déclarée avec `create: false` et construite au lancement seulement sous macOS et Windows : sous Linux, elle ne servirait à rien.

## Bureaux sans barre système

- Au lancement, `SystemTrayHost` (crate `infrastructure`) demande au bus de session D-Bus si le nom `org.kde.StatusNotifierWatcher` a un propriétaire (`NameHasOwner`, sur un fil à part : au plus 500 ms d'attente au lancement, même si le bus ne répond pas). Sans propriétaire, ou sans bus, l'app passe en mode fenêtre (`ShellMode::Window`). macOS et Windows ont toujours une barre système.
- Le client D-Bus est la crate `dbus`, déjà liée par `tao` sous Linux : aucune dépendance native de plus.
- En mode fenêtre, l'icône est quand même créée ; le tableau de bord est affiché au lancement, réduit à la fermeture, et porte un bouton « Quitter ». La note vers l'extension AppIndicator vient de la commande `read_window_mode_texts` ; son lien ouvre le navigateur par `tauri-plugin-opener`, côté Rust, sans permission front.
- Limite connue : un bureau qui affiche encore les anciennes icônes XEmbed sans `StatusNotifierWatcher` passe aussi en mode fenêtre ; l'app y reste utilisable.
