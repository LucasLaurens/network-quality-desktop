# États et verdict

La pastille de la barre des menus montre l'état de la connexion. Chaque état a une forme et une couleur, pour rester lisible sans distinguer les couleurs.

| État            | Forme                 | Quand                                                                            |
| --------------- | --------------------- | -------------------------------------------------------------------------------- |
| Solide          | disque plein vert     | les trois mesures sont bonnes                                                    |
| Instable        | demi-disque ambre     | la pire mesure est dans la zone intermédiaire                                    |
| Dégradée        | triangle rouge        | la pire mesure dépasse son seuil haut                                            |
| Hors ligne      | anneau barré gris     | aucun serveur ne répond, ni au ping ni en TCP, pendant 5 secondes                |
| Mesure en cours | anneau pointillé bleu | au démarrage et au retour de connexion, tant que 10 mesures n'ont pas été reçues |

Survoler la pastille affiche le nom de l'état (sauf sous Linux). Un clic sur la pastille (macOS et Windows) ouvre le **popover** : l'état, le [diagnostic](diagnostic.md), et les trois mesures avec leur méthode et une jauge qui situe la valeur entre les zones Solide, Instable et Dégradée. Le popover se ferme dès qu'on clique ailleurs, sur un nouveau clic sur la pastille ou avec Échap ; son bouton « Ouvrir le tableau de bord » ouvre la fenêtre principale.

## Menu de la pastille

Un clic droit sur la pastille (macOS et Windows) ouvre un menu : l'état, le constat, « Ouvrir le tableau de bord » et « Quitter ». L'état et le constat se mettent à jour quand la connexion change. Sous Linux, ce même menu remplace le popover : la barre système n'y transmet pas le clic.

Les entrées « Réglages » et « Tests de débit » s'ajouteront au menu avec leurs pages.

## Sans barre système

Certains bureaux n'affichent pas les icônes de la barre système (GNOME sans l'extension AppIndicator). L'app le détecte au lancement et passe en **mode fenêtre** : le tableau de bord s'ouvre au lancement, reste dans la barre des tâches et se réduit au lieu de se fermer ; on quitte par son bouton « Quitter ». Une note discrète explique pourquoi et mène à la page de l'extension ; « Masquer » la fait disparaître pour les lancements suivants.

L'ouverture automatique à l'ouverture de session arrivera avec le réglage « Lancer à l'ouverture de session ».

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
