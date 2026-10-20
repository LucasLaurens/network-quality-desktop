# États et verdict

La pastille de la barre des menus montre l'état de la connexion. Chaque état a une forme et une couleur, pour rester lisible sans distinguer les couleurs.

| État            | Forme                 | Quand                           |
| --------------- | --------------------- | ------------------------------- |
| Solide          | disque plein vert     | latence p90 sous 100 ms         |
| Instable        | demi-disque ambre     | latence p90 entre 100 et 300 ms |
| Dégradée        | triangle rouge        | latence p90 au-delà de 300 ms   |
| Mesure en cours | anneau pointillé bleu | aucune mesure encore reçue      |

Survoler la pastille affiche le nom de l'état (sauf sous Linux).

## Comment l'état est calculé aujourd'hui

- Chaque seconde, l'app envoie un petit ping à deux serveurs publics (Cloudflare 1.1.1.1 et Google 8.8.8.8) et retient la médiane des réponses.
- La latence retenue est le **p90 des 15 dernières secondes** : 90 % des mesures sont en dessous. Un pic isolé ne change donc pas l'état.
- La pastille reste stable : une **dégradation** n'apparaît qu'après **10 secondes** de suite, une **amélioration** qu'après **30 secondes**.

## Ce qui arrive ensuite

La gigue, la perte, l'état Hors ligne et la règle complète de Mesure en cours arrivent avec le ticket « Verdict complet ».
