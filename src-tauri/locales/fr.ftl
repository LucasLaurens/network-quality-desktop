## Texts shown by the app, French. Rules: impersonal tone, cause at most 80 characters,
## action at most 60, decimal comma, no-break space before units, "%" and ":",
## no exclamation mark, no em dash, no emoji. A test enforces all of them.

## States

state-solid = Solide
state-unstable = Instable
state-degraded = Dégradée
state-offline = Hors ligne
state-measuring = Mesure en cours

## Causes and actions

cause-healthy = Bonne connexion pour la visio.
cause-near-threshold = Bonne connexion, proche de la limite.
cause-wifi-weak = Le Wi-Fi est en cause : signal faible.
action-wifi-weak = Se rapprocher de la box ou passer en 5 GHz.
action-wifi-weak-closer = Se rapprocher de la box.
cause-wifi-slow = Le Wi-Fi est en cause : la box répond lentement.
action-wifi-slow = Se rapprocher de la box ou passer en 5 GHz.
cause-wifi-losses = Le Wi-Fi est en cause : des paquets se perdent avant la box.
action-wifi-losses = Changer de canal Wi-Fi ou redémarrer la box.
cause-phone-link = Le lien avec le téléphone est en cause.
action-phone-link = Se rapprocher du téléphone.
cause-local-link = Le réseau local est en cause, avant la box.
action-local-link = Vérifier le câble ou redémarrer la box.
cause-operator = L'opérateur est en cause, le Wi-Fi va bien.
action-operator = Patienter, ou redémarrer la box si cela dure.
cause-operator-cable = L'opérateur est en cause, le câble va bien.
cause-operator-local = L'opérateur est en cause, le réseau local va bien.
cause-mobile-network = Le réseau mobile est en cause, le lien avec le téléphone va bien.
action-mobile-network = Changer d'endroit ou passer sur un autre réseau.
cause-mobile-network-modem = Le réseau mobile est en cause.
cause-undetermined-system = La cause exacte ne peut pas être déterminée sur ce système.
action-allow-ping = Autoriser le ping (net.ipv4.ping_group_range).
help-ping-title = Autoriser le ping sans privilège
help-ping-why = Sous Linux, un ping envoyé sans être administrateur n'est permis qu'aux groupes listés dans net.ipv4.ping_group_range. Sans ce droit, l'app ne peut pas mesurer le lien avec la box.
help-ping-now = Pour cette session :
help-ping-now-command = sudo sysctl -w net.ipv4.ping_group_range="0 2147483647"
help-ping-keep = Pour le garder après un redémarrage :
help-ping-keep-command = echo 'net.ipv4.ping_group_range = 0 2147483647' | sudo tee /etc/sysctl.d/99-ping.conf > /dev/null
help-ping-without = Sans cela, l'app mesure Internet comme d'habitude : seul le tronçon vers la box reste inconnu.
menu-ping-help = Comment autoriser le ping…
cause-undetermined-network = La cause exacte ne peut pas être déterminée sur ce réseau.
cause-recovering = La connexion s'améliore, confirmation en cours.
cause-measuring = Premières mesures en cours.
cause-new-network = Nouveau réseau, premières mesures en cours.
cause-wake = Retour de veille, premières mesures en cours.
cause-new-servers = Nouveaux serveurs de mesure, premières mesures en cours.
cause-no-server = Aucun serveur ne répond.
cause-low-throughput = Le dernier test de débit est trop lent pour la visio.
action-low-throughput = Relancer un test dans quelques minutes.
cause-no-network = Aucun réseau connecté.
action-no-network = Se connecter à un Wi-Fi ou brancher un câble.
cause-box-without-internet = La box répond, mais Internet ne répond plus.
action-box-without-internet = Redémarrer la box ou vérifier le réseau de l'opérateur.
cause-phone-without-internet = Le téléphone répond, mais Internet ne répond plus.
action-phone-without-internet = Vérifier les données mobiles du téléphone.
action-check-connection = Vérifier le Wi-Fi ou le câble.
action-open-dashboard = Ouvrir le tableau de bord

## Path from the computer to the Internet

path-this-mac = Ce Mac
path-this-computer = Cet ordinateur
path-box = Box
path-phone = Téléphone
path-modem = Modem
path-internet = Internet
link-wifi = Wi-Fi
link-cable = Câble
link-local = Lien
link-operator = Opérateur
link-mobile-network = Réseau mobile
path-segment = { $link }, { $value }
path-unmeasurable = non mesurable
path-no-value = en attente
path-no-answer = aucune réponse
path-at-fault = en cause
path-cut = coupé
path-description = Chemin : { $computer }, { $gateway }, { $internet }. { $local }, puis { $remote }.
path-description-fault = Tronçon { $link } { $fault }.

## Network and connection

network-ethernet = Réseau Ethernet
network-wifi = Réseau Wi-Fi
network-cellular = Réseau mobile
network-hotspot = Partage de connexion
network-other = Réseau non reconnu
network-wifi-hidden = Wi-Fi (nom masqué)
action-reveal-name = Afficher le nom du réseau
connection-wifi-band = Wi-Fi { $band }
band-2-4-ghz = 2,4 GHz
band-5-ghz = 5 GHz
band-6-ghz = 6 GHz
wifi-channel = canal { $value }
network-vpn = Réseau via un VPN
connection-ethernet = Ethernet
connection-wifi = Wi-Fi
connection-cellular = Modem cellulaire
connection-hotspot = Partage de connexion d'un téléphone
connection-other = Connexion non reconnue
connection-vpn = VPN
connection-through-vpn = { $connection }, via un VPN
connection-limited = { $connection }, connexion limitée

## Menu

menu-quit = Quitter
menu-settings = Réglages…
menu-edit = Édition
menu-undo = Annuler
menu-redo = Rétablir
menu-cut = Couper
menu-copy = Copier
menu-paste = Coller
menu-select-all = Tout sélectionner
menu-window = Fenêtre
menu-minimize = Réduire
menu-close-window = Fermer la fenêtre

## History

history-set-aside = L'historique était illisible : il a été mis de côté ({ $file }) et un nouvel historique commence.
history-not-saved = L'historique ne peut pas être enregistré sur ce disque : il sera perdu à la fermeture.

## Dashboard

time-of-day = { $hours } h { $minutes }
date-time = { $day }/{ $month }, { $time }
date-hour = { $day }/{ $month }, { $hours } h
duration-seconds = { $value } s
duration-minutes = { $value } min
duration-hours = { $hours } h
duration-hours-minutes = { $hours } h { $minutes } min
duration-days = { $days } j
duration-days-hours = { $days } j { $hours } h
today-solid-share = Solide { $value } % du temps
today-not-enough = Pas encore assez de mesures
today-coverage = mesuré { $measured } sur { $elapsed }
outages-none = Aucune
outages-ongoing = En cours depuis { $duration }
outages-one = 1, à { $time } ({ $duration })
outages-one-unknown = 1, à { $time }
outages-many = { $count }, la plus longue { $duration }
outages-many-unknown = { $count }
worst-none = Aucun, tout est resté Solide
worst-moment = { $time }, { $metric } { $value }
worst-outage = { $time }, coupure
since-no-network = Aucun réseau
since-beyond = Plus de 30 jours
threshold-unstable = Instable { $value }
threshold-degraded = Dégradée { $value }
chart-network-change = Changement de réseau : { $network }
chart-coverage = Mesuré { $value } % de la période
share-solid = { $value } % Solide
shares = { $solid } % Solide, { $unstable } % Instable, { $degraded } % Dégradée, { $offline } % Hors ligne
value-unknown = valeur inconnue
value-pending = en cours

## Settings

settings-saved = Enregistré à { $time }.
settings-saved-session = Enregistré à { $time }, pour cette session seulement.
settings-not-saved = Le réglage n'a pas pu être enregistré.
server-invalid = Adresse IP invalide.
server-unreachable = Ce serveur ne répond pas au ping.

## Throughput tests

throughput-quick = Rapide
throughput-full = Complet
throughput-warning-limited = Connexion limitée : ce test consomme jusqu'à { $data }.
throughput-warning-hotspot = Partage de connexion : ce test consomme jusqu'à { $data }.
throughput-note = Les tests ne partent que sur un clic, vers Cloudflare. Rapide : jusqu'à { $quick }. Complet : jusqu'à { $full }. La latence en charge s'affiche sans compter dans l'état.
throughput-disabled = Les tests de débit sont désactivés dans les réglages.
throughput-busy = Un test est déjà en cours.
throughput-unreachable = Test indisponible : le serveur de test ne répond pas. Réessayer dans quelques minutes.

## Window mode, when the desktop shows no tray icon

tray-hint-message = Ce bureau n'affiche pas les icônes de la barre système : l'app reste ouverte dans une fenêtre. L'extension AppIndicator fait apparaître la pastille.
tray-hint-link = Ouvrir la page de l'extension
tray-hint-dismiss = Masquer

## Figures

detail-metric = { $metric } { $value }, seuil { $threshold }
detail-measuring = { $collected } sur { $needed }

## Metrics

metric-latency = Latence
metric-jitter = Gigue
metric-loss = Perte
method-latency = p90, 15 s
method-jitter = 15 s
method-loss = 60 s
metric-latency-lower = latence
metric-jitter-lower = gigue
metric-loss-lower = perte
definition-latency = Temps d'un aller-retour vers Internet.
definition-jitter = Variation de la latence d'un paquet à l'autre.
definition-loss = Paquets envoyés qui ne reviennent pas.

## Hints

hint-near-threshold = Proche du seuil
gauge-label = Solide sous { $unstable }, Dégradée au-delà de { $degraded }

## Units

unit-milliseconds = { $value } ms
unit-percent = { $value } %
unit-dbm = −{ $value } dBm
unit-megabytes = { $value } Mo
unit-gigabytes = { $value } Go
unit-megabits = { $value } Mbit/s

## Données : export et effacement de l'historique.
export-done = 3 fichiers enregistrés.
export-nothing = Rien à exporter pour l'instant.
export-reveal-finder = Afficher dans le Finder
export-reveal-explorer = Afficher dans l'Explorateur
export-reveal-folder = Afficher le dossier
export-failed = L'export n'a pas pu être écrit.
export-unavailable = L'historique n'est pas enregistré sur le disque pendant cette session : il n'y a rien à exporter.
history-erased = Historique effacé.
history-not-erased = L'historique n'a pas pu être effacé.


## Détails techniques du tableau de bord.
details-title = Détails techniques
details-connection = Connexion
details-network = Réseau
details-connection-kind = Liaison
details-gateway = Adresse de la box
details-wifi = Wi-Fi
details-wifi-signal = Signal
details-wifi-band = Bande
details-wifi-channel = Canal
details-wifi-rate = Débit négocié
details-traffic = Trafic observé
details-traffic-now = Sur { $seconds } secondes
details-traffic-peak = Pic de la dernière heure
details-traffic-waiting = Disponible dans quelques secondes
details-pair = ↓ { $received }, ↑ { $sent }
details-line = Ligne
details-access = Accès
details-line-rate = Débit synchronisé
details-box-traffic = Trafic total de la box
details-line-note = Valeurs annoncées par la box (UPnP), non mesurées. Son compteur de trafic repart souvent de zéro tous les 4 Go.
details-line-silent = La box ne répond pas en UPnP : l'accès n'est qu'estimé, d'après les tests de débit de ce réseau et la latence.
details-line-silent-unknown = La box ne répond pas en UPnP : l'accès reste inconnu tant que trois tests de débit sur ce réseau et la latence ne concordent pas.
access-reported-dsl = DSL (annoncé par la box)
access-estimated-fibre = Fibre (estimé)
access-estimated-adsl = ADSL (estimé)
access-unknown = Inconnu
details-line-failed = La ligne n'a pas pu être lue.

## Interface: every word of the windows, handed over by the `read_interface_texts` command.
## A `{ $name }` reaches the window as `{name}`, which the window fills.
ui-app-title = Qualité de connexion
ui-pages = Pages
ui-version = Version { $version }
ui-version-unknown = inconnue
ui-dashboard = Tableau de bord
ui-settings = Réglages
ui-cancel = Annuler
ui-last-month = 30 derniers jours
ui-updated-at = Mis à jour à { $time }
ui-overview = Vue d'ensemble
ui-overview-today = Aujourd'hui
ui-overview-outages = Coupures aujourd'hui
ui-overview-worst = Pire moment
ui-overview-since = Sur ce réseau depuis
ui-metrics-title = Mesures actuelles
ui-metrics-subtitle = Vers Internet, une mesure par seconde
ui-evolution-title = Évolution
ui-evolution-metric = Mesure affichée
ui-evolution-period = Période
ui-metric-latency = Latence
ui-metric-jitter = Gigue
ui-metric-loss = Perte
ui-period-hour = 1 h
ui-period-day = 24 h
ui-period-week = 7 j
ui-period-month = 30 j
ui-tick-hour-start = Il y a 1 h
ui-tick-hour-middle = Il y a 30 min
ui-tick-day-start = Il y a 24 h
ui-tick-day-middle = Il y a 12 h
ui-tick-week-start = Il y a 7 j
ui-tick-week-middle = Il y a 3 j
ui-tick-month-start = Il y a 30 j
ui-tick-month-middle = Il y a 15 j
ui-tick-now = Maintenant
ui-chart-label = { $metric } sur { $period }, avec les seuils Instable et Dégradée
ui-chart-empty = L'historique se remplit au fil des mesures. La première courbe apparaît après une minute.
ui-chart-band = Bande : l'état à chaque instant
ui-chart-no-value = Pas de mesure
ui-chart-values = Voir les valeurs
ui-chart-start = Début
ui-chart-value = Valeur
ui-chart-state = État
ui-networks-title = Tes réseaux
ui-networks-empty = Chaque réseau utilisé apparaîtra ici, avec le temps passé dans chaque état.
ui-networks-network = Réseau
ui-networks-shares = Temps par état
ui-networks-latency = Latence p90
ui-networks-measured = Durée mesurée
ui-networks-current = actuel
ui-networks-most-reliable = le plus fiable pour la visio
ui-throughput-title = Débit
ui-throughput-off = Tests de débit désactivés : aucune donnée n'est consommée.
ui-throughput-off-short = Tests de débit désactivés :
ui-throughput-enable = Activer dans Réglages
ui-throughput-last = Dernier test, { $when } : réception { $download }, envoi { $upload }
ui-throughput-when = Quand
ui-throughput-test = Test
ui-throughput-download = Réception
ui-throughput-upload = Envoi
ui-throughput-loaded-latency = Latence en charge
ui-throughput-data = Données
ui-quick-test = Test rapide
ui-full-test = Test complet
ui-test-running = Test en cours…
ui-test-confirm = Lancer le test
ui-details-title = Détails techniques
ui-details-asking-box = Interrogation de la box…
ui-settings-general = Général
ui-settings-launch = Lancer à l'ouverture de session
ui-settings-launch-hint = L'état reste visible dans la barre des menus.
ui-settings-appearance = Apparence
ui-settings-appearance-hint = Système suit le réglage clair ou sombre de l'ordinateur.
ui-appearance-system = Système
ui-appearance-light = Clair
ui-appearance-dark = Sombre
ui-settings-measure = Mesure
ui-settings-servers = Serveurs de mesure
ui-settings-servers-hint = Une petite requête par seconde pour mesurer latence, gigue et perte.
ui-servers-public = Cloudflare et Google (recommandé)
ui-servers-own = Serveur personnalisé
ui-settings-server = Adresse du serveur
ui-settings-server-example = ex. { $address }
ui-settings-server-hint = Une adresse IP qui répond au ping, ou sur le port 443. Seul ce serveur est alors mesuré.
ui-settings-notifications = Notifications
ui-settings-notify = Prévenir quand la connexion se dégrade
ui-settings-notify-hint = Seulement si la dégradation dure plus de 10 secondes.
ui-settings-throughput = Tests de débit
ui-settings-throughput-allow = Autoriser les tests de débit
ui-settings-throughput-hint = Désactivé par défaut. Les tests passent par Cloudflare (gratuit, sans compte) et ne partent jamais seuls : uniquement sur un clic sur Test rapide ou Test complet.
ui-settings-data = Données
ui-settings-export = Exporter l'historique
ui-settings-export-action = Exporter…
ui-settings-exporting = Export en cours…
ui-settings-export-hint = Trois fichiers CSV dans le dossier de votre choix : mesures, événements, tests de débit. Sans adresse matérielle de la box.
ui-export-last-day = 24 dernières heures
ui-export-last-week = 7 derniers jours
ui-export-all = Tout
ui-settings-erase = Effacer l'historique
ui-settings-erase-action = Effacer…
ui-settings-erase-warning = Toutes les mesures (minutes et heures), les événements (coupures, changements de réseau, veilles), les tests de débit et les réseaux connus seront supprimés, sans retour possible. Les réglages sont conservés.
