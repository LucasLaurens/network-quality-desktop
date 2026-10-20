# Produit

## Register

product

## Utilisateurs

Une personne qui travaille sur son ordinateur (MacBook Air au départ, puis Linux et Windows) et se connecte selon les jours à une box en Wi-Fi, en Ethernet ou au partage de connexion de son téléphone. Elle ouvre l'app dans deux situations :

- **Quand ça rame** : elle veut comprendre tout de suite si le problème vient de son Wi-Fi, de sa box ou de l'opérateur, et quoi faire.
- **Pour comparer ses connexions** : de temps en temps, elle regarde dans le temps quelle connexion tient le mieux, surtout pour la visio.

Le reste du temps, l'app reste une pastille dans la barre des menus que l'on ne regarde pas.

## But du produit

Afficher en temps réel l'état de la connexion Internet (Solide, Instable, Dégradée, Hors ligne, Mesure en cours) à partir de mesures réelles (latence, gigue, perte, débit), expliquer le « pourquoi » en une phrase, et garder un historique par réseau. La réussite : en moins de cinq secondes, on sait si la connexion tient et, sinon, où est le problème.

## Personnalité

Calme, précis, discret. Un instrument de mesure : il se fait oublier quand tout va bien et parle clairement quand quelque chose casse. Ton factuel, phrases courtes, jamais alarmiste, jamais de jargon sans explication.

Références d'esprit : réglages et menus natifs de macOS (intégration au système, sobriété), iStat Menus (graphiques compacts et lisibles), Linear et Raycast (soin typographique, densité aérée), Apple Santé et Météo (une information centrale très claire, graphiques simples, explications en phrases).

## Anti-références

- Speedtest commercial (Ookla) : compteurs de vitesse géants, néons, effet de jeu vidéo.
- Dashboard SaaS générique : grilles de cartes identiques, gros chiffres sur dégradé, fond bleu nuit.
- Console d'admin réseau : tableaux denses, jargon, des dizaines de métriques à l'écran.
- Appli ludique : mascottes, emojis, couleurs vives partout, gamification.

## Principes de design

1. **L'état d'abord, les chiffres ensuite.** Le verdict et sa cause se lisent avant toute valeur ; les métriques expliquent, elles ne remplacent pas.
2. **La couleur est réservée à l'état.** Rien de décoratif ne prend une teinte : quand quelque chose est coloré, c'est qu'il signale un état.
3. **Dire où est le problème.** Chaque dégradation désigne un tronçon (Wi-Fi, box, opérateur) et une action possible.
4. **Discret quand tout va bien.** Pas d'animation, pas de notification, pas de bruit tant que la connexion est Solide.
5. **Honnête sur ses limites.** Une estimation est marquée « estimé », une donnée absente est dite absente, jamais inventée.

## Accessibilité et inclusion

- WCAG AA pour les contrastes, dans les thèmes clair et sombre (l'app suit le thème de macOS).
- Un état n'est jamais porté par la couleur seule : chaque état a aussi une forme et un mot, pastille comprise.
- Le réglage « réduire les animations » est respecté.
- Navigation complète au clavier, focus toujours visible.
