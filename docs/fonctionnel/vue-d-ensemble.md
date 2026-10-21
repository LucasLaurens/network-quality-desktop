# Vue d'ensemble

## Ce que l'app fera

Une pastille dans la barre des menus donne l'état de la connexion Internet, mesuré en continu (latence, gigue, perte). Un clic ouvre le diagnostic : l'état, la cause en une phrase, le tronçon en cause (ce Mac, la box, l'opérateur) et quoi faire. Une fenêtre tableau de bord montre la vue d'ensemble de la journée, l'évolution dans le temps et la comparaison des réseaux utilisés.

## Ce que l'app fait aujourd'hui

Une pastille dans la barre des menus suit la latence, la gigue et la perte vers Internet, mesurées chaque seconde par un petit ping vers deux serveurs publics (voir [États et verdict](etats-et-verdict.md)). Un clic sur la pastille ouvre le popover : l'état, le réseau et son type de connexion (Ethernet, Wi-Fi, modem, partage de connexion, VPN), sa cause en une phrase avec quoi faire, le chemin Ce Mac → Box → Internet avec le tronçon en cause (voir [Diagnostic](diagnostic.md)) et les trois mesures. Un clic droit ouvre le menu de la pastille (sous Linux, c'est le seul accès). Le tableau de bord s'ouvre depuis le popover ou le menu ; il n'affiche encore que la version de l'app. Sur un bureau sans barre système, l'app reste ouverte dans une fenêtre (voir [États et verdict](etats-et-verdict.md#sans-barre-système)).
