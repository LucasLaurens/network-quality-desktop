# Historique

Ce qui est conservé, combien de temps, et ce qui se passe si le fichier est abîmé. Le [tableau de bord](tableau-de-bord.md) en tire ses courbes et sa comparaison des réseaux.

## Ce qui est conservé

L'app ne garde aucune mesure brute. Chaque minute, elle résume ce qu'elle a mesuré :

- la répartition des latences (de quoi retrouver médiane et p90 sur n'importe quelle période) ;
- la gigue moyenne ;
- les sondes envoyées et perdues ;
- le temps passé en Solide, Instable, Dégradée et Hors ligne. Les secondes de « Mesure en cours » ne comptent dans aucun état.

Chaque heure, les minutes sont additionnées en un résumé horaire : à la fin de l'heure, ou au plus tard dès qu'une minute d'une heure suivante arrive (réseau quitté avant la fin de l'heure, veille). Les changements de réseau, les débuts et fins de coupure, les mises en veille et les réveils sont notés à la seconde. Les secondes mesurées avant qu'aucun réseau ne soit vu (pas de connexion au lancement) comptent sous « aucun réseau ».

Chaque résumé est rangé sous son réseau : la box ou le téléphone (reconnu à l'adresse MAC de sa passerelle) et le type de connexion. Le nom du Wi-Fi n'est qu'un libellé. Tant que l'adresse de la passerelle n'est pas lisible (macOS sans l'autorisation « Réseau local »), les réseaux d'un même type sont regroupés.

## Combien de temps

| Donnée                                   | Durée                                       |
| ---------------------------------------- | ------------------------------------------- |
| Résumés par minute                       | 30 jours                                    |
| Résumés par heure                        | sans limite                                 |
| Changements de réseau, coupures, veilles | sans limite                                 |
| Tests de débit                           | sans limite                                 |
| Dernière heure seconde par seconde       | en mémoire seulement, perdue à la fermeture |

Après un redémarrage, la dernière heure repart des résumés par minute. Les durées s'arrêtent plus tôt si l'historique est effacé (voir [Effacer l'historique](#effacer-lhistorique)).

## Fiabilité

- Rien n'est jamais écrit à moitié : chaque écriture est complète ou n'a pas lieu. Une coupure de courant peut faire perdre les dernières minutes écrites.
- À la fermeture de l'app et avant chaque mise en veille, la minute en cours est écrite. Une minute coupée en deux par un redémarrage est complétée au retour.
- Le temps passé en veille n'est compté nulle part : il apparaît comme un trou dans les courbes.
- Les heures laissées ouvertes par un arrêt sont terminées au lancement suivant. Une minute n'est jamais effacée tant que son heure n'est pas résumée.

## Si le fichier est abîmé

Au lancement, l'app vérifie le fichier `history.sqlite3`. S'il est abîmé (ce n'est plus une base, ou son contrôle d'intégrité échoue), elle le renomme `history.corrupt-<secondes UTC>.sqlite3` avec ses fichiers de journal (rien n'est effacé), commence un nouvel historique et le dit dans le tableau de bord. Pour toute autre raison (fichier occupé, dossier inutilisable, historique créé par une version plus récente de l'app), le fichier n'est pas touché : l'historique est gardé en mémoire pour la session et le tableau de bord le signale. Si le tableau de bord ne peut pas ouvrir un fichier que l'app enregistre bien, il paraît vide et le dit : les mesures continuent d'être gardées. Une erreur d'écriture plus tard est seulement notée dans le journal de l'app.

Le fichier se trouve dans le dossier de données de l'app :

| Système | Dossier                                                           |
| ------- | ----------------------------------------------------------------- |
| macOS   | `~/Library/Application Support/com.lucaslaurens.network-quality/` |
| Windows | `%APPDATA%\com.lucaslaurens.network-quality\`                     |
| Linux   | `~/.local/share/com.lucaslaurens.network-quality/`                |

## Effacer l'historique

Dans [Réglages](reglages.md#données), « Effacer l'historique » supprime, après confirmation, toutes les mesures (minutes et heures), les événements (coupures, changements de réseau, veilles et réveils), les tests de débit et les réseaux connus, avec leur adresse MAC et leur nom de Wi-Fi. Les réglages restent. L'effacement est définitif : les données supprimées sont écrasées et le fichier est réécrit, pour qu'elles ne restent pas sur le disque.

La dernière heure en mémoire part aussi, et la minute en cours n'est pas écrite : l'historique repart comme au premier lancement. À la seconde suivante, le réseau actuel est noté de nouveau, et une coupure en cours aussi. Pour garder une copie avant d'effacer, [exporter](export.md) d'abord.
