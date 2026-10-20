# Plateformes

Adaptateurs par OS, permissions, barre système, veille et réveil : cette partie sera écrite avec les tickets d'adaptateurs natifs.

## Windows : manifeste de l'application

Tauri a besoin des Common Controls v6, déclarés dans un manifeste Windows (`src-tauri/windows-app-manifest.xml`). Par défaut, Tauri ne lie ce manifeste qu'au binaire de l'app : les binaires de test refusent alors de démarrer (`STATUS_ENTRYPOINT_NOT_FOUND`, code `0xc0000139`). Le script de build (`src-tauri/build.rs`) désactive le manifeste de Tauri et lie le même fichier à toutes les cibles, tests compris, avec le linker MSVC.

## Linux : droit de ping

La sonde ICMP sans privilège exige que `net.ipv4.ping_group_range` inclue le groupe de l'utilisateur (c'est le cas par défaut sur les distributions récentes basées sur systemd). Sinon, aucune sonde n'aboutit et la pastille reste en « Mesure en cours ».
