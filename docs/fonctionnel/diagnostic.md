# Diagnostic

Le « pourquoi » s'affiche sous l'état, dans le popover. Il se compose de trois éléments :

1. **Constat** : ce qui se passe et, quand l'état n'est pas Solide, où.
2. **Action** : quoi faire, seulement quand une action existe.
3. **Détail chiffré** : la mesure en cause et son seuil, sur une ligne à part, pour que la phrase reste stable pendant que les chiffres bougent.

## Le chemin

Sous le constat, le popover dessine le chemin **Ce Mac → Box → Internet** (« Cet ordinateur » hors macOS, « Téléphone » pour un partage de connexion, « Modem » pour un modem intégré). Chaque tronçon porte son lien et sa latence (« Wi-Fi, 4 ms », « Opérateur, 180 ms ») ; hors ligne, « aucune réponse » ; un lien non mesurable l'indique. Le tronçon fautif est marqué du mot **en cause**, ou **coupé** quand la box répond sans Internet : la couleur n'est jamais le seul repère.

## Où est le problème

L'état vient de la mesure de bout en bout (vers Internet). L'app mesure aussi le lien avec la box par un ping, chaque seconde, et s'en sert seulement pour dire où est le problème :

| Lien avec la box                                                               | Bout en bout | Constat                                                                                                               |
| ------------------------------------------------------------------------------ | ------------ | --------------------------------------------------------------------------------------------------------------------- |
| mauvais (plus de 30 ms, ou plus de 1 % de pertes avec au moins 2 pings perdus) | mauvais      | le Wi-Fi, le câble ou le lien avec le téléphone est en cause ; en Wi-Fi, un signal sous −70 dBm est nommé comme cause |
| bon                                                                            | mauvais      | l'opérateur est en cause (le réseau mobile pour un partage de connexion ou un modem)                                  |
| mauvais                                                                        | bon          | rien : l'état reste Solide                                                                                            |
| non mesurable                                                                  | mauvais      | la cause exacte ne peut pas être déterminée                                                                           |

Le lien avec la box n'est pas mesurable quand le système refuse le ping (Linux sans réglage), quand la box n'a répondu à aucun ping pendant une minute, quand aucune box n'est visible ou sous VPN (les mesures passent par le tunnel), et pendant ses dix premières secondes. L'app ne devine pas.

## Causes affichées

| Situation                                                     | Constat                                                           | Action                                                  |
| ------------------------------------------------------------- | ----------------------------------------------------------------- | ------------------------------------------------------- |
| Tout va bien                                                  | Bonne connexion pour la visio.                                    | aucune                                                  |
| Une mesure approche de son seuil                              | Bonne connexion, proche de la limite.                             | aucune                                                  |
| Wi-Fi au signal faible (sous −70 dBm)                         | Le Wi-Fi est en cause : signal faible.                            | Se rapprocher de la box ou passer en 5 GHz.             |
| Wi-Fi lent jusqu'à la box                                     | Le Wi-Fi est en cause : la box répond lentement.                  | Se rapprocher de la box ou passer en 5 GHz.             |
| Pertes avant la box en Wi-Fi                                  | Le Wi-Fi est en cause : des paquets se perdent avant la box.      | Changer de canal Wi-Fi ou redémarrer la box.            |
| Lien avec le téléphone                                        | Le lien avec le téléphone est en cause.                           | Se rapprocher du téléphone.                             |
| Câble ou autre lien local                                     | Le réseau local est en cause, avant la box.                       | Vérifier le câble ou redémarrer la box.                 |
| Opérateur                                                     | L'opérateur est en cause, le Wi-Fi va bien.                       | Patienter, ou redémarrer la box si cela dure.           |
| Réseau mobile                                                 | Le réseau mobile est en cause, le lien avec le téléphone va bien. | Changer d'endroit ou passer sur un autre réseau.        |
| Ping refusé par le système                                    | La cause exacte ne peut pas être déterminée sur ce système.       | Autoriser le ping (net.ipv4.ping_group_range).          |
| Box muette ou cachée                                          | La cause exacte ne peut pas être déterminée sur ce réseau.        | aucune                                                  |
| Les mesures sont revenues dans les seuils, l'état attend 30 s | La connexion s'améliore, confirmation en cours.                   | aucune                                                  |
| Premières mesures (lancement, retour d'une coupure)           | Premières mesures en cours.                                       | aucune, progression « 6 sur 10 »                        |
| Un autre réseau a pris le relais                              | Nouveau réseau, premières mesures en cours.                       | aucune, progression                                     |
| Box sans Internet                                             | La box répond, mais Internet ne répond plus.                      | Redémarrer la box ou vérifier le réseau de l'opérateur. |
| Téléphone sans Internet                                       | Le téléphone répond, mais Internet ne répond plus.                | Vérifier les données mobiles du téléphone.              |
| Aucun réseau                                                  | Aucun réseau connecté.                                            | Se connecter à un Wi-Fi ou brancher un câble.           |
| Rien ne répond, box comprise                                  | Aucun serveur ne répond.                                          | Vérifier le Wi-Fi ou le câble.                          |

Écarts volontaires avec la décision sur le ton :

- « Le Wi-Fi est en cause : la box répond lentement. » couvre un Wi-Fi lent dont le signal n'est pas faible.
- « Redémarrer la box ou vérifier le réseau de l'opérateur. » au lieu de « … l'état du réseau de l'opérateur. », qui dépassait 60 caractères.
- Les causes « Lien avec le téléphone », « Câble ou autre lien local », « Téléphone sans Internet » et « Rien ne répond » complètent le catalogue, avec les variantes de « Opérateur » (câble, autre lien) et de « Réseau mobile » (modem intégré), pour les connexions qu'il ne couvrait pas.
- Le « lien d'aide » de Linux est une aide repliée dans l'app, sous l'action « Autoriser le ping » du popover et du tableau de bord : le dépôt n'est pas public, un lien vers sa documentation mènerait nulle part. Dépliée, elle dit pourquoi le ping est refusé, donne les deux commandes à copier (pour la session, puis pour la garder après un redémarrage) et ce que l'app mesure sans cela. Le menu de la pastille y mène aussi par « Comment autoriser le ping… », qui ouvre le tableau de bord avec l'aide dépliée : sous Linux, il n'y a pas de popover.
- « La cause exacte ne peut pas être déterminée sur ce réseau. » s'ajoute pour une box muette ou cachée, que « sur ce système » décrirait mal.

Sous chaque mesure proche de son seuil, le popover indique « Proche du seuil ». Chaque jauge est lue par les lecteurs d'écran avec ses seuils, par exemple « Solide sous 100 ms, Dégradée au-delà de 300 ms ».

## Ton et typographie

Ton impersonnel (actions à l'infinitif), constat de 80 caractères au plus, action de 60 au plus, virgule décimale, espace insécable avant les unités, « % » et « : », ni point d'exclamation, ni tiret long, ni emoji. Un test vérifie ces règles sur tout le catalogue.
