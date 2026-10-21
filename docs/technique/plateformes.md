# Plateformes

Adaptateurs par OS, permissions, barre système, veille et réveil : cette partie sera écrite avec les tickets d'adaptateurs natifs.

## Windows : manifeste de l'application

Tauri a besoin des Common Controls v6, déclarés dans un manifeste Windows (`src-tauri/windows-app-manifest.xml`). Par défaut, Tauri ne lie ce manifeste qu'au binaire de l'app : les binaires de test refusent alors de démarrer (`STATUS_ENTRYPOINT_NOT_FOUND`, code `0xc0000139`). Le script de build (`src-tauri/build.rs`) désactive le manifeste de Tauri et lie le même fichier à toutes les cibles, tests compris, avec le linker MSVC.

## Linux : droit de ping

La sonde ICMP sans privilège exige que `net.ipv4.ping_group_range` inclue le groupe de l'utilisateur (c'est le cas par défaut sur les distributions récentes basées sur systemd). Sinon, l'app mesure par connexion TCP sur le port 443 : la latence reste juste, et la perte compte les connexions qui échouent, ce qui est moins fin qu'une perte de paquets ICMP.

## Icône de barre système

- L'icône est dessinée en Rust (`src-tauri/src/tray_icon.rs`), une forme par état (cinq, Hors ligne compris), avec les couleurs claires des tokens. Elle a été vérifiée lisible sur barre claire et sombre ; le gris de Hors ligne y est plus discret ; elle n'est pas redessinée quand le thème change.
- Elle porte un menu minimal (« Quitter ») : sous Linux, certains environnements masquent une icône sans menu, et c'est le seul moyen d'interagir avec elle. Le clic gauche n'ouvre pas ce menu, il est réservé au popover.
- Le popover est ancré à l'icône par le plugin positioner : centré sous l'icône sous macOS, au-dessus sous Windows (barre des tâches en bas). Sous Linux, Tauri ne transmet pas le clic sur l'icône : le popover y sera ouvert depuis le menu (ticket « Menu natif »).
