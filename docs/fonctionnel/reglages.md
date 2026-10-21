# Réglages

Chaque réglage, sa valeur par défaut et son effet. La page s'ouvre depuis le haut de la fenêtre (« Tableau de bord » / « Réglages »).

## Enregistrement

Chaque changement est enregistré aussitôt, puis confirmé sous le formulaire : « Enregistré à 14 h 05. ». Un réglage refusé est expliqué au même endroit, et ce qui a été tapé reste dans le champ : le formulaire n'est jamais remis à zéro pendant la saisie. L'adresse d'un serveur n'est envoyée qu'une fois terminée, par Entrée ou en quittant le champ.

Les réglages sont gardés avec l'historique, sur l'ordinateur. Si l'historique ne peut pas être gardé sur le disque, ils valent pour la session seulement.

## Général

| Réglage                         | Par défaut | Effet                                                                                                                                             |
| ------------------------------- | ---------- | ------------------------------------------------------------------------------------------------------------------------------------------------- |
| Lancer à l'ouverture de session | activé     | L'app démarre avec la session, l'état visible dans la barre des menus. Une version de développement ne s'inscrit jamais à l'ouverture de session. |
| Apparence                       | Système    | Système suit le réglage clair ou sombre de l'ordinateur ; Clair et Sombre l'imposent à toutes les fenêtres, tout de suite.                        |

## Mesure

| Réglage            | Par défaut           | Effet                                                                                                                                                                                                                                                |
| ------------------ | -------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Serveurs de mesure | Cloudflare et Google | Une petite requête par seconde vers 1.1.1.1 et 8.8.8.8. Avec un **serveur personnalisé**, seul ce serveur est mesuré : aucun autre n'est contacté. Changer de serveurs repart de zéro : « Nouveaux serveurs de mesure, premières mesures en cours. » |

Le serveur personnalisé s'écrit en adresse IP (IPv4 ou IPv6, par exemple `192.168.1.10`). Avant d'être enregistré, il est pingé une fois : « Adresse IP invalide. » si ce n'est pas une adresse, « Ce serveur ne répond pas au ping. » s'il ne répond pas. Les noms de domaine ne sont pas encore acceptés.

Les réglages des notifications et des tests de débit s'ajouteront à cette page avec leurs fonctions.
