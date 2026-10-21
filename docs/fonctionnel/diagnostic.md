# Diagnostic

Le « pourquoi » s'affiche sous l'état, dans le popover. Il se compose de trois éléments :

1. **Constat** : ce qui se passe, en une phrase courte.
2. **Action** : quoi faire, seulement quand une action existe.
3. **Détail chiffré** : la mesure en cause et son seuil, sur une ligne à part, pour que la phrase reste stable pendant que les chiffres bougent.

## Causes affichées aujourd'hui

| Situation                        | Constat                                           | Action                                        | Détail                              |
| -------------------------------- | ------------------------------------------------- | --------------------------------------------- | ----------------------------------- |
| Tout va bien                     | Bonne connexion pour la visio.                    | aucune                                        | aucun                               |
| Une mesure approche de son seuil | Bonne connexion, proche de la limite.             | aucune                                        | ex. « Latence 85 ms, seuil 100 ms » |
| Une mesure dépasse son seuil     | La connexion ne tient pas les seuils de la visio. | aucune                                        | ex. « Gigue 41 ms, seuil 30 ms »    |
| Premières mesures                | Premières mesures en cours.                       | aucune                                        | progression, ex. « 6 sur 10 »       |
| Aucun serveur joignable          | Aucun réseau connecté.                            | Se connecter à un Wi-Fi ou brancher un câble. | aucun                               |

Sous chaque mesure proche de son seuil, le popover indique « Proche du seuil ».

## Ce qui arrive ensuite

Le ticket « Diagnostic par tronçon » désignera le tronçon en cause (Wi-Fi, opérateur, réseau mobile, box sans Internet) et remplacera le constat générique « La connexion ne tient pas les seuils de la visio. ».

## Ton et typographie

Ton impersonnel (actions à l'infinitif), constat de 80 caractères au plus, action de 60 au plus, virgule décimale, espace insécable avant les unités, « % » et « : », ni point d'exclamation, ni tiret long, ni emoji. Un test vérifie ces règles sur tout le catalogue.
