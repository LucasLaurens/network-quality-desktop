# Confidentialité

Ce qui quitte l'ordinateur, vers qui, et ce qui reste local. Cette page est complétée par chaque ticket qui ajoute un échange réseau.

| Échange                       | Destinataires                                | Fréquence                                                                  | Contenu                                                         |
| ----------------------------- | -------------------------------------------- | -------------------------------------------------------------------------- | --------------------------------------------------------------- |
| Ping ICMP                     | Cloudflare (`1.1.1.1`) et Google (`8.8.8.8`) | 1 par seconde et par serveur                                               | un paquet d'écho de 32 octets, sans donnée personnelle          |
| Connexion TCP sur le port 443 | les mêmes serveurs                           | seulement quand un ping est perdu, ou chaque seconde si le ping est bloqué | une poignée de main TCP, refermée aussitôt, sans donnée envoyée |

L'app lit aussi, sur l'ordinateur seulement, les interfaces réseau et l'adresse MAC de la box ou du téléphone qui sert de passerelle, pour reconnaître le réseau. Sous macOS, cette lecture demande l'autorisation « Réseau local ». Elle demande aussi au système si la connexion est limitée (Network.framework, coût de connexion Windows, NetworkManager sous Linux). Rien de cela ne quitte l'ordinateur.

Rien n'est enregistré ni envoyé ailleurs.
