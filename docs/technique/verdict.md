# Verdict

## Seuils

Tous les seuils vivent dans un seul objet du domaine, `Thresholds::for_video_calls()`. Aujourd'hui, seule la latence est notée : Solide sous 100 ms, Instable de 100 à 300 ms inclus, Dégradée au-delà.

## Fenêtre et percentile

`LatencyWindow` garde les 15 dernières secondes ; une seconde sans réponse ne compte pas comme échantillon. Le p90 utilise le rang le plus proche : sur 15 échantillons, c'est le 14e plus petit. Il faut donc au moins deux secondes lentes dans la fenêtre pour que le p90 devienne lent.

## Hystérésis

`Hysteresis` est une machine à états pure :

- le premier niveau observé s'affiche tout de suite ;
- un niveau plus mauvais que celui affiché doit tenir 10 s de suite ;
- un niveau meilleur doit tenir 30 s de suite ;
- revenir au niveau affiché, ou changer de sens, remet le compteur à zéro.

Tant qu'aucune latence n'est connue, le verdict est « Mesure en cours ».
