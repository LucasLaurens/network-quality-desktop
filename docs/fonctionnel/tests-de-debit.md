# Tests de débit

Les tests Rapide et Complet, lancés uniquement sur clic, et les données qu'ils consomment.

## Activer les tests

Les tests sont désactivés par défaut : la section « Débit » du tableau de bord dit alors « Tests de débit désactivés : aucune donnée n'est consommée. » et mène à Réglages (« Autoriser les tests de débit », voir [Réglages](reglages.md)). Une fois autorisés, deux boutons apparaissent : **Test rapide** et **Test complet**. Aucun test ne part jamais seul, ni au lancement, ni au changement de réseau, ni à intervalle régulier.

## Les deux tests

| Test    | Durée                 | Données        | Ce qu'il fait                                                                                                         |
| ------- | --------------------- | -------------- | --------------------------------------------------------------------------------------------------------------------- |
| Rapide  | environ 3 s           | environ 5 Mo   | un petit transfert puis des transferts de 1 Mo en réception, de 0,5 Mo en envoi                                       |
| Complet | jusqu'à 15 s par sens | jusqu'à 160 Mo | des transferts de plus en plus gros (100 ko, 1 Mo, 10 Mo, 25 Mo), arrêtés dès qu'un transfert dure plus d'une seconde |

La réception passe avant l'envoi, jamais les deux en même temps. Le débit d'un sens est le 90e percentile de ses transferts de 1 Mo ou plus. Pendant les transferts, une petite requête toutes les 250 ms mesure la **latence en charge** : elle montre si la ligne gonfle sa file d'attente quand elle est pleine, ce qui coupe une visio malgré un bon débit. Elle s'affiche dans le tableau, sans compter dans l'état.

Sur une connexion limitée ou un partage de connexion, le test demande d'abord confirmation, avec le volume qu'il peut consommer (« Connexion limitée : ce test consomme jusqu'à 5,2 Mo. »).

Pendant un test, son bouton affiche « Test en cours… » et l'autre est désactivé : un seul test à la fois. Si le serveur ne répond pas : « Test indisponible : le serveur de test ne répond pas. Réessayer dans quelques minutes. »

## Effet sur l'état

Un test trop lent pour une visio tient l'état à son niveau pendant 10 minutes, ou jusqu'au changement de réseau :

| Sens      | Solide       | Instable       | Dégradée     |
| --------- | ------------ | -------------- | ------------ |
| Réception | ≥ 2,5 Mbit/s | entre les deux | < 0,6 Mbit/s |
| Envoi     | ≥ 1,5 Mbit/s | entre les deux | < 0,6 Mbit/s |

Le pire des deux sens compte. L'état affiché devient le plus mauvais entre la mesure en continu et le test, avec le constat « Le dernier test de débit est trop lent pour la visio. ». Un test assez rapide ne change rien.

## Historique

Chaque test est gardé sur l'ordinateur, sous son réseau : quand, lequel, réception, envoi, latence en charge et données consommées. Le tableau de bord affiche les 20 derniers, le plus récent en premier. Les débits sont gardés au kbit/s près, les durées à la milliseconde.

Le serveur de test est celui de Cloudflare (`speed.cloudflare.com`), gratuit et sans compte ; voir [Confidentialité](confidentialite.md).
