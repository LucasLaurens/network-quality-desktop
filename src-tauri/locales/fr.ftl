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
cause-mobile-network = Le réseau mobile est en cause, le lien avec le téléphone va bien.
action-mobile-network = Changer d'endroit ou passer sur un autre réseau.
cause-undetermined-system = La cause exacte ne peut pas être déterminée sur ce système.
action-allow-ping = Autoriser le ping (net.ipv4.ping_group_range).
cause-undetermined-network = La cause exacte ne peut pas être déterminée sur ce réseau.
cause-recovering = La connexion s'améliore, confirmation en cours.
cause-measuring = Premières mesures en cours.
cause-new-network = Nouveau réseau, premières mesures en cours.
cause-no-server = Aucun serveur ne répond.
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
path-at-fault = en cause
path-cut = coupé
path-description = Chemin : { $computer }, { $gateway }, { $internet }.
path-description-fault = Tronçon { $link } { $fault }.

## Network and connection

network-ethernet = Réseau Ethernet
network-wifi = Réseau Wi-Fi
network-cellular = Réseau mobile
network-hotspot = Partage de connexion
network-other = Réseau non reconnu
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

## Hints

hint-near-threshold = Proche du seuil
gauge-label = Solide sous { $unstable }, Dégradée au-delà de { $degraded }

## Units

unit-milliseconds = { $value } ms
unit-percent = { $value } %
