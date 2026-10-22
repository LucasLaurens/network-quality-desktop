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
menu-quick-test = Test rapide
menu-full-test = Test complet
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
value-unknown = –

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
