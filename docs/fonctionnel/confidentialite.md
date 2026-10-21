# Confidentialité

Ce qui quitte l'ordinateur, vers qui, et ce qui reste local. Cette page est complétée par chaque ticket qui ajoute un échange réseau.

| Échange                       | Destinataires                                | Fréquence                                                                  | Contenu                                                         |
| ----------------------------- | -------------------------------------------- | -------------------------------------------------------------------------- | --------------------------------------------------------------- |
| Ping ICMP                     | Cloudflare (`1.1.1.1`) et Google (`8.8.8.8`) | 1 par seconde et par serveur                                               | un paquet d'écho de 32 octets, sans donnée personnelle          |
| Connexion TCP sur le port 443 | les mêmes serveurs                           | seulement quand un ping est perdu, ou chaque seconde si le ping est bloqué | une poignée de main TCP, refermée aussitôt, sans donnée envoyée |

L'app pinge aussi la box ou le téléphone qui sert de passerelle, une fois par seconde : ce paquet reste sur le réseau local. Elle lit aussi, sur l'ordinateur seulement, les interfaces réseau et l'adresse MAC de la box ou du téléphone qui sert de passerelle, pour reconnaître le réseau. Sous macOS, cette lecture demande l'autorisation « Réseau local ». Elle demande aussi au système si la connexion est limitée (Network.framework, coût de connexion Windows, NetworkManager sous Linux). En Wi-Fi, elle lit le signal, la bande, le canal, le débit et, si le système le permet, le nom du réseau ; sous macOS et Windows, ce nom demande l'accès à la localisation, mais l'app ne lit jamais la position ; Windows peut afficher son icône « localisation utilisée » à chaque lecture (toutes les 10 secondes). Rien de cela ne quitte l'ordinateur.

L'historique (résumés par minute et par heure, changements de réseau, coupures) est enregistré dans un fichier sur l'ordinateur ; voir [Historique](historique.md). Rien n'est envoyé ailleurs.
