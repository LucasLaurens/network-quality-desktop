# Design

Système visuel de l'app, tiré de la maquette validée (ticket « Maquetter pastille, popover et dashboard »). Les valeurs exactes vivent dans un seul fichier, `src/styles/tokens.css` ; ce document décrit leur rôle et leurs règles d'usage.

## Thème

- Suit le thème clair ou sombre du système par défaut. Chaque token a une valeur claire et une valeur sombre.
- Couleurs en OKLCH. Neutres légèrement teintés vers une pierre chaude ; jamais de noir ou de blanc purs.

## Couleur

Stratégie retenue : la couleur est **réservée à l'état**. Rien de décoratif ne prend une teinte.

| Token                                                             | Rôle                                                                  |
| ----------------------------------------------------------------- | --------------------------------------------------------------------- |
| `--bg`, `--surface`, `--sunken`                                   | Fond de fenêtre, surfaces, zones en creux (contrôles segmentés)       |
| `--ink`, `--muted`                                                | Texte principal, texte secondaire (contraste AA dans les deux thèmes) |
| `--line`, `--zone`                                                | Filets et séparateurs, plages neutres des jauges                      |
| `--focus`                                                         | Anneau de focus clavier                                               |
| `--solid`, `--unstable`, `--degraded`, `--offline`, `--measuring` | Les cinq états, et rien d'autre                                       |

Un état n'est jamais porté par la couleur seule : chaque état a aussi une forme (disque plein, demi-disque, triangle, anneau barré, anneau pointillé) et un mot.

## Typographie

- Police système (SF Pro sur macOS, Segoe UI sur Windows, `system-ui` ailleurs). Aucune police téléchargée.
- Échelle : `--t-xs` 11, `--t-sm` 12, `--t-md` 13 (corps), `--t-lg` 16, `--t-xl` 19, `--t-2xl` 28.
- Chiffres alignés (`font-variant-numeric: tabular-nums`) pour toutes les valeurs mesurées.

## Espace et formes

- Espacements sur une grille de 4 : `--s1` 4 à `--s7` 48.
- Un rayon (`--radius` 8 px) et un rayon imbriqué (`--radius-nested` 6 px).
- Une seule ombre (`--shadow`), réservée aux surfaces flottantes (popover, fenêtre).
- Ni dégradé, ni effet de verre, ni emoji, ni grille de cartes identiques.

## Mouvement

Le mouvement signale un changement d'état, rien d'autre : moins de 200 ms, courbe en sortie douce, et aucune animation quand le système demande de réduire les animations.
