# Confidentialité

Ce qui quitte l'ordinateur, vers qui, et ce qui reste local. Cette page est complétée par chaque ticket qui ajoute un échange réseau.

| Échange                       | Destinataires                                | Fréquence                                                                  | Contenu                                                                                                                               |
| ----------------------------- | -------------------------------------------- | -------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------- |
| Ping ICMP                     | Cloudflare (`1.1.1.1`) et Google (`8.8.8.8`) | 1 par seconde et par serveur                                               | un paquet d'écho de 32 octets, sans donnée personnelle                                                                                |
| Connexion TCP sur le port 443 | les mêmes serveurs                           | seulement quand un ping est perdu, ou chaque seconde si le ping est bloqué | une poignée de main TCP, refermée aussitôt, sans donnée envoyée                                                                       |
| Test de débit (HTTPS)         | Cloudflare (`speed.cloudflare.com`)          | seulement sur un clic, quand les tests sont autorisés dans Réglages        | des téléchargements et des envois de zéros (5 à 160 Mo), sans donnée personnelle ; Cloudflare voit l'adresse IP, comme pour tout site |

Quand le ping est bloqué, l'app le réessaie au réveil, sur un autre réseau ou avec d'autres serveurs : un ping par seconde et par serveur, en plus de la connexion TCP, jusqu'à ce qu'il réponde ou soit perdu 10 fois de suite.

L'app pinge aussi la box ou le téléphone qui sert de passerelle, une fois par seconde : ce paquet reste sur le réseau local. Elle lit aussi, sur l'ordinateur seulement, les interfaces réseau et l'adresse MAC de la box ou du téléphone qui sert de passerelle, pour reconnaître le réseau. Sous macOS, cette lecture demande l'autorisation « Réseau local ». Elle demande aussi au système si la connexion est limitée (Network.framework, coût de connexion Windows, NetworkManager sous Linux). En Wi-Fi, elle lit le signal, la bande, le canal, le débit et, si le système le permet, le nom du réseau ; sous macOS et Windows, ce nom demande l'accès à la localisation, mais l'app ne lit jamais la position ; Windows peut afficher son icône « localisation utilisée » à chaque lecture (toutes les 10 secondes). Rien de cela ne quitte l'ordinateur.

Elle lit aussi chaque seconde les compteurs d'octets de l'interface réseau, pour le trafic observé des détails techniques : des totaux, jamais le contenu ni les destinataires des échanges. Ils restent en mémoire.

Seulement quand l'utilisateur ouvre les « Détails techniques » du tableau de bord, l'app cherche la box en UPnP (recherche SSDP en multicast sur le réseau local), puis lui demande le type de sa ligne, son débit synchronisé et ses compteurs de trafic. Ces questions sont en lecture seule : l'app n'ouvre ni ne modifie jamais de port sur la box. Elles restent sur le réseau local ; sous macOS, elles passent par la même autorisation « Réseau local ».

L'historique (résumés par minute et par heure, changements de réseau, coupures) est enregistré dans un fichier sur l'ordinateur ; voir [Historique](historique.md). Pour reconnaître les réseaux, ce fichier garde sans limite de durée l'adresse MAC de la box ou du téléphone et le nom du Wi-Fi. Rien n'est envoyé ailleurs. « Effacer l'historique », dans Réglages, supprime tout cela, réseaux compris ; seuls les réglages restent.

L'[export](export.md) écrit, à la demande, trois fichiers dans le dossier choisi : ils contiennent le nom des Wi-Fi, mais jamais l'adresse MAC de la box. Ce que l'utilisateur en fait ensuite ne passe plus par l'app.
