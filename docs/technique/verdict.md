# Verdict

## Seuils

Les seuils des trois métriques vivent dans un seul objet du domaine, `Thresholds::for_video_calls()` :

| Métrique      | Instable à partir de | Dégradée au-delà de |
| ------------- | -------------------- | ------------------- |
| Latence (p90) | 100 ms               | 300 ms              |
| Gigue         | 30 ms                | 50 ms               |
| Perte         | 1 %                  | 5 %                 |

`GradeBounds<T>` est générique sur la valeur mesurée (`Duration`, `LossRate`). « Proche du seuil » : la valeur atteint 80 % de la borne qui la ferait passer au niveau suivant. La taille des fenêtres, le percentile, les délais d'hystérésis, le délai Hors ligne et le nombre d'échantillons de démarrage sont des constantes privées de leur module, qui sont leur seul endroit.

## Calcul à chaque seconde

1. Chaque cible donne une issue (`ProbeOutcome`) : réponse avec son temps, perdue mais joignable en TCP, ou injoignable.
2. La médiane des réponses entre dans `LatencyWindow` (15 s) ; chaque cible a aussi sa propre fenêtre pour la gigue ; envoyées et perdues entrent dans `LossWindow` (60 s, totaux tenus à jour). La perte n'a de valeur qu'à partir de 60 sondes (30 s avec deux cibles) : avant, une seule perte pèserait plusieurs pour cent.
3. `OfflineDetector` compte les secondes sans aucune cible joignable : à 5, le verdict est Hors ligne, la vue ne porte plus aucune mesure, et le moteur repart de zéro (fenêtres, hystérésis, compteur de démarrage).
4. Tant que moins de 10 secondes ont reçu une réponse depuis le (re)démarrage, le verdict est « Mesure en cours ».
5. Sinon, le pire niveau des trois métriques passe par l'hystérésis.

## Fenêtre et percentile

Le p90 utilise le rang le plus proche : sur 15 échantillons, c'est le 14e plus petit. Il faut donc au moins deux secondes lentes dans la fenêtre pour que le p90 devienne lent. La gigue est l'écart moyen entre deux secondes successives ayant chacune une réponse, calculé **par cible** puis moyenné : comparer la médiane de deux cibles inventerait de la gigue dès qu'une cible perd un paquet. C'est la grandeur qu'estime la gigue de la RFC 3550, sans son lissage exponentiel.

## Hystérésis

`Hysteresis` est une machine à états pure :

- le premier niveau observé s'affiche tout de suite ;
- un niveau plus mauvais que celui affiché doit tenir 10 s de suite ; un meilleur, 30 s ;
- revenir au niveau affiché, ou changer de sens, remet le compteur à zéro ;
- c'est le sens qui doit tenir, pas le niveau : Solide, puis Instable, puis Dégradée affiche Dégradée 10 s après la première dégradation.
