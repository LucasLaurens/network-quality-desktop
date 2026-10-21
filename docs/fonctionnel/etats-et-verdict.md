# États et verdict

La pastille de la barre des menus montre l'état de la connexion. Chaque état a une forme et une couleur, pour rester lisible sans distinguer les couleurs.

| État            | Forme                 | Quand                                                                                                                                                                   |
| --------------- | --------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Solide          | disque plein vert     | les trois mesures sont bonnes                                                                                                                                           |
| Instable        | demi-disque ambre     | la pire mesure est dans la zone intermédiaire                                                                                                                           |
| Dégradée        | triangle rouge        | la pire mesure dépasse son seuil haut                                                                                                                                   |
| Hors ligne      | anneau barré gris     | aucun serveur ne répond, ni au ping ni en TCP, pendant 5 secondes                                                                                                       |
| Mesure en cours | anneau pointillé bleu | au démarrage, au retour de connexion, au réveil, quand un autre réseau prend le relais ou que les serveurs de mesure changent, tant que 10 mesures n'ont pas été reçues |

Survoler la pastille affiche le nom de l'état (sauf sous Linux). Un clic sur la pastille (macOS et Windows) ouvre le **popover** : l'état, le [diagnostic](diagnostic.md) avec le chemin Ce Mac → Box → Internet et le tronçon en cause, et les trois mesures avec leur méthode et une jauge qui situe la valeur entre les zones Solide, Instable et Dégradée. Le popover se ferme dès qu'on clique ailleurs, sur un nouveau clic sur la pastille ou avec Échap ; son bouton « Ouvrir le tableau de bord » ouvre la fenêtre principale.

## Réseau et type de connexion

Le popover nomme le réseau à droite de l'état, avec le type de connexion en dessous.

| Type                                | Nom affiché                                                                                           | Comment l'app le reconnaît                                                                                                                                            |
| ----------------------------------- | ----------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Ethernet                            | Réseau Ethernet                                                                                       | type d'interface                                                                                                                                                      |
| Wi-Fi                               | nom du réseau ; « Wi-Fi (nom masqué) » sans localisation ; « Réseau Wi-Fi » avant la première lecture | type d'interface                                                                                                                                                      |
| Modem cellulaire                    | Réseau mobile                                                                                         | modem intégré à l'ordinateur                                                                                                                                          |
| Partage de connexion d'un téléphone | Partage de connexion                                                                                  | au moins deux indices indépendants : adressage habituel des téléphones, nom d'interface de téléphone (USB), adresse de passerelle aléatoire ; jamais l'adressage seul |
| VPN                                 | Réseau via un VPN, ou le nom de la connexion en dessous                                               | la mesure passe par le VPN ; le type affiché est « Wi-Fi, via un VPN » quand la connexion physique est connue, sinon « VPN »                                          |
| Autre                               | Réseau non reconnu                                                                                    | connexion d'un type inconnu, ou lien PPP (DSL, clé 3G)                                                                                                                |

Un réseau est reconnu par l'adresse MAC de sa passerelle et son type ; le nom du Wi-Fi n'est qu'un libellé. Le contexte est relu toutes les 10 secondes.

En Wi-Fi, le popover affiche le nom du réseau, la bande (« Wi-Fi 5 GHz ») et une ligne de détails : signal, canal et débit (« −58 dBm, canal 36, 866 Mbit/s »). macOS et Windows ne donnent le nom qu'avec l'accès à la localisation : sans lui, le popover affiche « Wi-Fi (nom masqué) » et un lien **Afficher le nom du réseau**. Sous macOS, le lien fait apparaître la demande du système la première fois, puis ouvre les réglages de localisation ; sous Windows, il ouvre les réglages (Windows pose lui-même la question une fois). L'app ne lit jamais la position. Refuser laisse le signal et le canal mesurés ; sous Windows, le débit demande aussi la localisation. Linux donne le nom sans permission. Sur un partage de connexion, un signal faible est annoncé comme « Le lien avec le téléphone est en cause. ».

Quand les données sont comptées (forfait mobile, partage de connexion, mode « données réduites » de macOS), le type de connexion l'indique : « Wi-Fi, connexion limitée ». Sans réponse du système, un modem cellulaire ou un partage de connexion est supposé limité.

Dès que le système signale un changement d'interface ou d'adresse, l'app relit le réseau. Si c'est un autre réseau (autre passerelle, autre type, VPN activé ou coupé), elle oublie les mesures et repasse en **Mesure en cours**. Sinon, rien ne change : une adresse renouvelée ou une interface virtuelle ne remet pas l'état à zéro. Sans événement, la relecture toutes les 10 secondes remarque aussi le changement.

## Menu de la pastille

Un clic droit sur la pastille (macOS et Windows) ouvre un menu : l'état, le constat, « Ouvrir le tableau de bord » et « Quitter ». L'état et le constat se mettent à jour quand la connexion change. Sous Linux, ce même menu remplace le popover : la barre système n'y transmet pas le clic.

Les entrées « Réglages » et « Tests de débit » s'ajouteront au menu avec leurs pages.

## Sans barre système

Certains bureaux n'affichent pas les icônes de la barre système (GNOME sans l'extension AppIndicator). L'app le détecte au lancement et passe en **mode fenêtre** : le tableau de bord s'ouvre au lancement, reste dans la barre des tâches et se réduit au lieu de se fermer ; on quitte par son bouton « Quitter ». Une note discrète explique pourquoi et mène à la page de l'extension ; « Masquer » la fait disparaître pour les lancements suivants.

L'app s'ouvre à l'ouverture de session tant que le réglage « Lancer à l'ouverture de session » est activé (voir [Réglages](reglages.md)).

## Notifications

Quand l'état affiché empire durablement, une notification du système le dit : son titre est le nouvel état (« Instable », « Dégradée », « Hors ligne »), son texte le constat (« L'opérateur est en cause, le Wi-Fi va bien. »), sans action ni chiffre.

- Empirer durablement : un état plus mauvais ne s'affiche qu'après 10 secondes soutenues ; une coupure est annoncée après 10 secondes sans aucun serveur (« Hors ligne » s'affiche à 5 secondes), même juste après un démarrage ou un réveil.
- Jamais pour une amélioration, ni pour le premier niveau mesuré après « Mesure en cours » (démarrage, réveil, nouveau réseau, nouveaux serveurs) : il n'empire rien de connu.
- Un lien qui oscille entre Instable et Dégradée peut notifier à chaque passage vers le pire : au plus une fois toutes les 40 secondes, puisqu'un retour vers le mieux demande 30 secondes.
- Une fois par dégradation : Solide puis Instable puis Dégradée donne deux notifications, une à chaque marche.
- Le réglage « Prévenir quand la connexion se dégrade » les coupe toutes (voir [Réglages](reglages.md#notifications)).

## Tâche de fond, veille et réveil

L'app tourne en tâche de fond : fermer le tableau de bord le cache sans arrêter la mesure, et sous macOS elle n'a pas d'icône dans le Dock. On ne la quitte que par « Quitter » dans le menu de la pastille (ou le bouton du mode fenêtre) : ⌘Q dans le tableau de bord ne fait rien. Relancer l'app alors qu'elle tourne déjà ouvre simplement son tableau de bord.

Avant la mise en veille, l'app écrit la minute en cours puis ne mesure plus rien. Une coupure en cours s'arrête là : la veille n'est pas comptée comme une coupure. Au réveil, elle affiche « Retour de veille, premières mesures en cours. » et repart de zéro. Si le système ne prévient pas, l'app s'en aperçoit d'elle-même : plus de 10 secondes écoulées entre deux mesures valent un réveil, et des réponses qui ont attendu pendant la veille sont écartées.

## Les trois mesures

| Mesure  | Ce qu'elle dit                                                          | Solide          | Instable     | Dégradée       |
| ------- | ----------------------------------------------------------------------- | --------------- | ------------ | -------------- |
| Latence | temps d'un aller-retour vers Internet (p90 des 15 dernières secondes)   | moins de 100 ms | 100 à 300 ms | plus de 300 ms |
| Gigue   | variation de la latence d'une seconde à l'autre (15 dernières secondes) | moins de 30 ms  | 30 à 50 ms   | plus de 50 ms  |
| Perte   | part des pings sans réponse (dernière minute, deux serveurs)            | moins de 1 %    | 1 à 5 %      | plus de 5 %    |

L'état affiché est celui de la **pire** des trois. Une mesure à moins de 20 % du seuil suivant est signalée « Proche du seuil » dans le popover.

## Stabilité

- Une **dégradation** n'apparaît qu'après **10 secondes** de suite, une **amélioration** qu'après **30 secondes**.
- **Hors ligne** s'affiche dès 5 secondes sans aucun serveur joignable. Au retour, l'app repart de zéro : Mesure en cours, puis le premier état mesuré s'affiche aussitôt.
- Un ping perdu alors que le serveur répond encore en TCP compte comme une **perte**, pas comme une coupure. Une coupure de moins de 5 secondes compte aussi comme perte.
- La perte n'est notée qu'après 30 secondes de mesure : avant, une seule perte suffirait à faire passer l'état à Instable.
