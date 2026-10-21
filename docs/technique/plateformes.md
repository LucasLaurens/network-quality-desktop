# Plateformes

Adaptateurs par OS, permissions, barre système, veille et réveil : cette partie sera écrite avec les tickets d'adaptateurs natifs.

## Windows : manifeste de l'application

Tauri a besoin des Common Controls v6, déclarés dans un manifeste Windows (`src-tauri/windows-app-manifest.xml`). Par défaut, Tauri ne lie ce manifeste qu'au binaire de l'app : les binaires de test refusent alors de démarrer (`STATUS_ENTRYPOINT_NOT_FOUND`, code `0xc0000139`). Le script de build (`src-tauri/build.rs`) désactive le manifeste de Tauri et lie le même fichier à toutes les cibles, tests compris, avec le linker MSVC.

## Linux : droit de ping

La sonde ICMP sans privilège exige que `net.ipv4.ping_group_range` inclue le groupe de l'utilisateur (c'est le cas par défaut sur les distributions récentes basées sur systemd). Sinon, l'app mesure par connexion TCP sur le port 443 : la latence reste juste, et la perte compte les connexions qui échouent, ce qui est moins fin qu'une perte de paquets ICMP.

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
