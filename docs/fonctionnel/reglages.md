# Réglages

Chaque réglage, sa valeur par défaut et son effet. La page s'ouvre depuis le haut de la fenêtre (« Tableau de bord » / « Réglages »).

## Enregistrement

Chaque changement est enregistré aussitôt, puis confirmé sous le formulaire : « Enregistré à 14 h 05. ». Les changements partent un par un, dans l'ordre : le dernier fait foi. Un réglage refusé est expliqué au même endroit, et ce qui a été tapé reste dans le champ : le formulaire n'est jamais remis à zéro pendant la saisie. L'adresse d'un serveur n'est envoyée qu'une fois terminée, par Entrée ou en quittant le champ ; un autre réglage changé entre-temps garde le dernier serveur accepté.

Les réglages sont gardés avec l'historique, sur l'ordinateur. S'ils ne peuvent pas être gardés sur le disque, ils valent pour la session seulement, et la confirmation le dit : « Enregistré à 14 h 05, pour cette session seulement. »

## Général

| Réglage                         | Par défaut | Effet                                                                                                                                                                                                                                                      |
| ------------------------------- | ---------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Lancer à l'ouverture de session | activé     | L'app démarre avec la session, l'état visible dans la barre des menus. Le réglage de l'app fait foi : retirer l'app des éléments d'ouverture du système sans le décocher l'y remet au lancement suivant. Une version de développement ne s'inscrit jamais. |
| Apparence                       | Système    | Système suit le réglage clair ou sombre de l'ordinateur ; Clair et Sombre l'imposent à toutes les fenêtres, tout de suite.                                                                                                                                 |

## Mesure

| Réglage            | Par défaut           | Effet                                                                                                                                                                                                                                                |
| ------------------ | -------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Serveurs de mesure | Cloudflare et Google | Une petite requête par seconde vers 1.1.1.1 et 8.8.8.8. Avec un **serveur personnalisé**, seul ce serveur est mesuré : aucun autre n'est contacté. Changer de serveurs repart de zéro : « Nouveaux serveurs de mesure, premières mesures en cours. » |

Le serveur personnalisé s'écrit en adresse IP (IPv4 ou IPv6, par exemple `192.168.1.10`). Avant d'être enregistré, il est sondé une fois, comme la mesure le ferait (ping, ou connexion sur le port 443 si le ping est bloqué) : « Adresse IP invalide. » si ce n'est pas une adresse, « Ce serveur ne répond pas au ping. » s'il ne répond pas, ce qui est aussi le cas sans réseau. Un serveur déjà enregistré n'est pas sondé de nouveau quand un autre réglage change. Les noms de domaine ne sont pas encore acceptés.

Limite : un serveur du réseau local sur une connexion sans accès à Internet (pas de route par défaut) est mesuré, mais l'état reste « Aucun réseau connecté » : l'app reconnaît la connexion par sa route vers Internet.

## Notifications

| Réglage                                | Par défaut | Effet                                                                                                                                                                  |
| -------------------------------------- | ---------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Prévenir quand la connexion se dégrade | activé     | Une notification du système quand l'état empire durablement (voir [États et verdict](etats-et-verdict.md#notifications)). Désactivé, l'état change sans rien annoncer. |

## Tests de débit

| Réglage                      | Par défaut | Effet                                                                                                                                              |
| ---------------------------- | ---------- | -------------------------------------------------------------------------------------------------------------------------------------------------- |
| Autoriser les tests de débit | désactivé  | Fait apparaître Test rapide et Test complet dans le tableau de bord. Les tests ne partent jamais seuls (voir [Tests de débit](tests-de-debit.md)). |

## Données

| Action                | Effet                                                                                                                                                                                 |
| --------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Exporter l'historique | Trois fichiers CSV (mesures, événements, tests de débit) dans un nouveau dossier de Téléchargements, sur 24 heures, 7 jours, 30 jours (par défaut) ou Tout. Voir [Export](export.md). |
| Effacer l'historique  | Supprime tout l'historique, après confirmation. Voir [Historique](historique.md#effacer-lhistorique).                                                                                 |

Avant d'effacer, l'app dit ce qui part : « Toutes les mesures (minutes et heures), les événements (coupures, changements de réseau, veilles) et les tests de débit seront supprimés, sans retour possible. Les réglages sont conservés. » Rien n'est supprimé sans un clic sur « Effacer l'historique » ; « Annuler » referme l'avertissement. Le résultat s'affiche sous la section : « Historique effacé. » ou « L'historique n'a pas pu être effacé. ». Pendant un export ou un effacement, les deux boutons sont désactivés.
