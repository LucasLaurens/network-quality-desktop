# Tableau de bord

La fenêtre qui répond, dans l'ordre : comment va la connexion maintenant, comment s'est passée la journée, comment elle a évolué, et quel réseau tient le mieux. Elle s'ouvre depuis le popover ou le menu de la pastille.

## État actuel

En haut, le même diagnostic que le popover : l'état (avec sa forme, jamais la couleur seule), le réseau et son type de connexion, l'heure de la dernière mise à jour, le constat, quoi faire et le chemin Ce Mac → Box → Internet avec le tronçon en cause (voir [Diagnostic](diagnostic.md)).

Dessous, quatre faits sur la journée, comptée depuis minuit à l'heure de l'ordinateur :

| Fait                 | Ce qu'il dit                                                                                                                                                                                                      |
| -------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Aujourd'hui          | La part du temps mesuré passée en Solide, arrondie vers le bas : 100 % seulement si chaque seconde était Solide. « Pas encore assez de mesures » avant la première minute écrite.                                 |
| Coupures aujourd'hui | « Aucune » ; une seule : son heure et sa durée ; plusieurs : leur nombre et la plus longue ; une coupure en cours : « En cours depuis … ». Une coupure interrompue par une fermeture de l'app compte, sans durée. |
| Pire moment          | La minute la plus pénible pour un appel (une seconde Instable compte 1, Dégradée 2, Hors ligne 3), avec la mesure la plus au-delà de son seuil Instable, ou « coupure ». « Aucun, tout est resté Solide » sinon.  |
| Sur ce réseau depuis | Depuis quand l'ordinateur est sur le réseau actuel, d'après les changements de réseau des 30 derniers jours. Un redémarrage de l'app sur le même réseau ne remet pas le compteur à zéro.                          |

## Mesures actuelles

Latence, gigue et perte vers Internet, une mesure par seconde : pour chacune, sa définition en clair, sa valeur, sa jauge avec les seuils (« Solide sous …, Dégradée au-delà de … »), sa méthode (p90 sur 15 s, moyenne sur 15 s, 60 s) et « Proche du seuil » quand elle s'approche du seuil suivant.

## Évolution

Une courbe par mesure (latence p90, gigue moyenne, perte) sur 1 h, 24 h, 7 j ou 30 j :

| Période | Un point par | D'où viennent les points                                                                                         |
| ------- | ------------ | ---------------------------------------------------------------------------------------------------------------- |
| 1 h     | minute       | la mémoire depuis le lancement ; avant, les résumés par minute (la dernière heure survit ainsi à un redémarrage) |
| 24 h    | 10 minutes   | les résumés par minute                                                                                           |
| 7 j     | heure        | les résumés par heure, et les minutes de l'heure en cours                                                        |
| 30 j    | 3 heures     | les résumés par heure, et les minutes de l'heure en cours                                                        |

- Les seuils Instable et Dégradée sont tracés et nommés à gauche de la courbe.
- Là où rien n'a été mesuré (app fermée, veille), la courbe s'interrompt : aucun trait ne relie les deux bords du trou.
- Un changement de réseau est marqué d'un trait vertical avec le nom du nouveau réseau. Un redémarrage sur le même réseau n'en est pas un ; deux Wi-Fi de noms différents restent deux réseaux, même avant que l'adresse de la box soit lue.
- La bande sous la courbe montre l'état qui a duré le plus longtemps dans chaque point (le pire en cas d'égalité) ; gris clair quand rien n'a été mesuré.
- Sous la bande, la part de la période réellement mesurée, arrondie vers le bas (« Mesuré 62 % de la période »).
- Survoler la courbe, ou la parcourir au clavier (flèches gauche et droite), affiche l'heure, la valeur et l'état du point. « Voir les valeurs » ouvre le tableau des points mesurés.
- Changer de mesure ne relit pas l'historique ; changer de période, si. Le tout se rafraîchit toutes les 30 secondes.

## Tes réseaux

Chaque réseau des 30 derniers jours, le plus utilisé en premier : son nom (celui du Wi-Fi quand il est connu, sinon un nom générique), son type de connexion, la répartition de son temps entre les états, sa latence p90 et sa perte. Le réseau actuel porte la mention « actuel ». Quand au moins deux réseaux ont été mesurés une heure ou plus, celui qui a passé la plus grande part de son temps en Solide est désigné « le plus fiable pour la visio ».

## Premier lancement

Avant la première minute écrite, la courbe laisse la place à « L'historique se remplit au fil des mesures. La première courbe apparaît après une minute. » et le tableau des réseaux à « Chaque réseau utilisé apparaîtra ici, avec le temps passé dans chaque état. »

Si l'historique ne peut être gardé que pour la session (voir [Historique](historique.md#si-le-fichier-est-abîmé)), le tableau de bord ne voit que la dernière heure en mémoire.
