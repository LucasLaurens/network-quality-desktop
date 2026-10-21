# Confidentialité

Ce qui quitte l'ordinateur, vers qui, et ce qui reste local. Cette page est complétée par chaque ticket qui ajoute un échange réseau.

| Échange                       | Destinataires                                | Fréquence                                                                  | Contenu                                                         |
| ----------------------------- | -------------------------------------------- | -------------------------------------------------------------------------- | --------------------------------------------------------------- |
| Ping ICMP                     | Cloudflare (`1.1.1.1`) et Google (`8.8.8.8`) | 1 par seconde et par serveur                                               | un paquet d'écho de 32 octets, sans donnée personnelle          |
| Connexion TCP sur le port 443 | les mêmes serveurs                           | seulement quand un ping est perdu, ou chaque seconde si le ping est bloqué | une poignée de main TCP, refermée aussitôt, sans donnée envoyée |

Rien n'est enregistré ni envoyé ailleurs.
