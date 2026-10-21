# Plateformes

Adaptateurs par système, permissions et barre système. La veille et le réveil s'y ajouteront avec leur ticket.

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

## Icône de barre système

- L'icône est dessinée en Rust (`src-tauri/src/tray_icon.rs`), une forme par état (cinq, Hors ligne compris), avec les couleurs claires des tokens. Elle a été vérifiée lisible sur barre claire et sombre ; le gris de Hors ligne y est plus discret ; elle n'est pas redessinée quand le thème change.
- Elle porte un menu natif défini une seule fois pour les trois systèmes (`src-tauri/src/tray_menu.rs`) : état et constat (lignes grisées, réécrites quand leur texte change), « Ouvrir le tableau de bord », « Quitter ». Sous macOS et Windows, il s'ouvre au clic droit ; le clic gauche est réservé au popover.
- Le popover est ancré à l'icône par le plugin positioner : centré sous l'icône sous macOS, au-dessus sous Windows (barre des tâches en bas). Sous Linux, Tauri ne transmet pas le clic sur l'icône : il n'y a pas de popover (`HAS_POPOVER`), le menu s'ouvre à tout clic.
- Fermer le tableau de bord ne quitte jamais l'app (`ShellMode::choose_close_action`) : il est masqué en mode barre système, réduit en mode fenêtre. On quitte par le menu, ou en mode fenêtre par le bouton « Quitter » du tableau de bord (commande `quit_app`).
- Sous macOS, l'app n'a pas d'icône dans le Dock (politique d'activation `Accessory`) : c'est une app de barre des menus, le tableau de bord s'ouvre depuis la pastille.
- La fenêtre popover est déclarée avec `create: false` et construite au lancement seulement sous macOS et Windows : sous Linux, elle ne servirait à rien.

## Bureaux sans barre système

- Au lancement, `SystemTrayHost` (crate `infrastructure`) demande au bus de session D-Bus si le nom `org.kde.StatusNotifierWatcher` a un propriétaire (`NameHasOwner`, sur un fil à part : au plus 500 ms d'attente au lancement, même si le bus ne répond pas). Sans propriétaire, ou sans bus, l'app passe en mode fenêtre (`ShellMode::Window`). macOS et Windows ont toujours une barre système.
- Le client D-Bus est la crate `dbus`, déjà liée par `tao` sous Linux : aucune dépendance native de plus.
- En mode fenêtre, l'icône est quand même créée ; le tableau de bord est affiché au lancement, réduit à la fermeture, et porte un bouton « Quitter ». La note vers l'extension AppIndicator vient de la commande `read_window_mode_texts` ; son lien ouvre le navigateur par `tauri-plugin-opener`, côté Rust, sans permission front.
- Limite connue : un bureau qui affiche encore les anciennes icônes XEmbed sans `StatusNotifierWatcher` passe aussi en mode fenêtre ; l'app y reste utilisable.
