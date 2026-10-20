# Verdict

## Seuils

Les seuils des métriques vivent dans un seul objet du domaine, `Thresholds::for_video_calls()`. Aujourd'hui, seule la latence est notée : Solide sous 100 ms, Instable de 100 à 300 ms inclus, Dégradée au-delà. La taille de la fenêtre, le percentile et les délais d'hystérésis sont des constantes privées de leur module (`latency_window.rs`, `hysteresis.rs`), qui sont leur seul endroit.

## Fenêtre et percentile

`LatencyWindow` garde les 15 dernières secondes ; une seconde sans réponse ne compte pas comme échantillon. Le p90 utilise le rang le plus proche : sur 15 échantillons, c'est le 14e plus petit. Il faut donc au moins deux secondes lentes dans la fenêtre pour que le p90 devienne lent.

## Hystérésis

`Hysteresis` est une machine à états pure :

- le premier niveau observé s'affiche tout de suite ;
- un niveau plus mauvais que celui affiché doit tenir 10 s de suite ;
- un niveau meilleur doit tenir 30 s de suite ;
- revenir au niveau affiché, ou changer de sens, remet le compteur à zéro ;
- c'est le sens qui doit tenir, pas le niveau : Solide, puis Instable, puis Dégradée affiche Dégradée 10 s après la première dégradation.

Quand toute la fenêtre est restée sans réponse (15 s), l'hystérésis repart de zéro et le verdict redevient « Mesure en cours » ; le premier niveau mesuré ensuite s'affiche tout de suite. Pendant les 15 premières secondes d'une coupure, le p90 porte encore sur les dernières réponses : l'état Hors ligne arrive avec le ticket « Verdict complet ».
