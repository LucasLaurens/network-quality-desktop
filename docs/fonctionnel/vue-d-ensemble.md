# Vue d'ensemble

## Ce que l'app fera

Une pastille dans la barre des menus donne l'état de la connexion Internet, mesuré en continu (latence, gigue, perte). Un clic ouvre le diagnostic : l'état, la cause en une phrase, le tronçon en cause (ce Mac, la box, l'opérateur) et quoi faire. Une fenêtre tableau de bord montre la vue d'ensemble de la journée, l'évolution dans le temps et la comparaison des réseaux utilisés.

## Ce que l'app fait aujourd'hui

Une pastille dans la barre des menus suit la latence, la gigue et la perte vers Internet, mesurées chaque seconde par un petit ping vers deux serveurs publics (voir [États et verdict](etats-et-verdict.md)). Un clic sur la pastille ouvre le popover : l'état, le réseau et son type de connexion (Ethernet, Wi-Fi, modem, partage de connexion, VPN), sa cause en une phrase avec quoi faire, le chemin Ce Mac → Box → Internet avec le tronçon en cause (voir [Diagnostic](diagnostic.md)) et les trois mesures. Un clic droit ouvre le menu de la pastille (sous Linux, c'est le seul accès). Le [tableau de bord](tableau-de-bord.md) s'ouvre depuis le popover ou le menu : l'état et les faits de la journée, les trois mesures expliquées, leur évolution sur 1 h à 30 jours et la comparaison des réseaux utilisés. Sur un bureau sans barre système, l'app reste ouverte dans une fenêtre (voir [États et verdict](etats-et-verdict.md#sans-barre-système)). La page [Réglages](reglages.md) règle l'ouverture de session, l'apparence et les serveurs de mesure. Elle tourne en tâche de fond, écrit la minute en cours avant la veille et repart en « Mesure en cours » au réveil (voir [Tâche de fond, veille et réveil](etats-et-verdict.md#tâche-de-fond-veille-et-réveil)).
