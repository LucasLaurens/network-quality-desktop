# Export

Le contenu des trois fichiers CSV et leur import dans un tableur.

## Exporter

Dans [Réglages](reglages.md#données), section « Données » : choisir la période, puis **Exporter**. La période se compte à partir du moment de l'export : 24 dernières heures, 7 derniers jours, 30 derniers jours (par défaut) ou Tout.

L'app crée un nouveau dossier dans Téléchargements, nommé à l'heure de l'ordinateur (`qualite-reseau-AAAA-MM-JJ-HHMMSS`, par exemple `qualite-reseau-2026-10-21-140512`), y écrit les trois fichiers et ouvre le dossier dans le gestionnaire de fichiers. La page le confirme : « Export enregistré dans Téléchargements : qualite-reseau-2026-10-21-140512. ». Si l'export échoue, aucun dossier n'est laissé : « L'export n'a pas pu être écrit. »

L'export lit l'historique ligne par ligne pendant que la mesure continue : un historique de plusieurs années s'exporte sans charger la mémoire. Un historique gardé en mémoire pour la session (voir [Historique](historique.md#si-le-fichier-est-abîmé)) ne s'exporte pas : « L'historique n'est pas enregistré sur le disque pendant cette session : il n'y a rien à exporter. »

## Format

- CSV selon la RFC 4180 : virgule comme séparateur, fin de ligne CRLF, une ligne d'en-tête. Un champ n'est entre guillemets que s'il contient une virgule, un guillemet ou un retour à la ligne (un nom de Wi-Fi, par exemple), ses guillemets doublés.
- Encodage UTF-8, sans BOM.
- Dates en UTC, au format ISO 8601 : `2026-10-21T12:05:00Z`.
- Nombres décimaux avec un point : `12.6`.
- Un champ vide est une valeur non mesurée.
- Le réseau est nommé par le nom de son Wi-Fi, vide quand il n'est pas connu (Ethernet, nom non lisible). Un nom qui commence par `=`, `+`, `-`, `@`, une tabulation ou un retour chariot est précédé d'une apostrophe, pour qu'un tableur ne l'exécute pas comme une formule.
- L'adresse MAC de la box n'est jamais exportée, ni la répartition détaillée des latences.

## `mesures.csv`

Une ligne par réseau et par minute ou par heure. Les minutes sont gardées 30 jours : avant, ce sont les heures, y compris l'heure dont la purge a déjà retiré les premières minutes, qui sort entière. Une seconde n'apparaît jamais deux fois. Une période qui commence au milieu d'une heure exportée en heure (30 derniers jours, par exemple) reprend cette heure entière, donc quelques minutes de plus.

| Colonne               | Contenu                                                           |
| --------------------- | ----------------------------------------------------------------- |
| `debut_utc`           | début de la minute ou de l'heure                                  |
| `granularite`         | `minute` ou `heure`                                               |
| `reseau`              | nom du Wi-Fi                                                      |
| `type_connexion`      | `ethernet`, `wifi`, `cellulaire`, `partage_de_connexion`, `autre` |
| `latence_p90_ms`      | latence p90, en millisecondes au dixième                          |
| `gigue_moyenne_ms`    | gigue moyenne, en millisecondes au dixième                        |
| `perte_pct`           | perte en pourcentage, deux décimales                              |
| `sondes_envoyees`     | sondes envoyées                                                   |
| `sondes_perdues`      | sondes perdues                                                    |
| `secondes_solide`     | secondes passées en Solide                                        |
| `secondes_instable`   | secondes passées en Instable                                      |
| `secondes_degrade`    | secondes passées en Dégradée                                      |
| `secondes_hors_ligne` | secondes passées Hors ligne                                       |

Les secondes de « Mesure en cours » ne comptent dans aucun état : la somme des quatre dernières colonnes peut rester sous 60 ou 3 600.

## `evenements.csv`

| Colonne          | Contenu                                                                    |
| ---------------- | -------------------------------------------------------------------------- |
| `horodatage_utc` | moment de l'événement, à la seconde                                        |
| `reseau`         | nom du Wi-Fi                                                               |
| `type_connexion` | comme dans `mesures.csv`                                                   |
| `evenement`      | `changement_de_reseau`, `debut_coupure`, `fin_coupure`, `veille`, `reveil` |

## `tests-de-debit.csv`

Les [tests de débit](tests-de-debit.md) terminés.

| Colonne                | Contenu                             |
| ---------------------- | ----------------------------------- |
| `horodatage_utc`       | fin du test                         |
| `reseau`               | nom du Wi-Fi                        |
| `type_connexion`       | comme dans `mesures.csv`            |
| `profil`               | `rapide` ou `complet`               |
| `reception_kbps`       | débit en réception, en kbit/s       |
| `envoi_kbps`           | débit en envoi, en kbit/s           |
| `latence_en_charge_ms` | latence en charge, en millisecondes |
| `octets`               | données consommées                  |
| `duree_ms`             | durée du test, en millisecondes     |

## Ouvrir dans un tableur

Un tableur réglé en français attend en général le point-virgule et la virgule décimale : ouvrir le fichier par l'import de texte plutôt que par un double clic, puis choisir l'encodage UTF-8, la virgule comme séparateur et le point comme séparateur décimal.

- Excel : Données, À partir d'un fichier texte/CSV ; origine du fichier « 65001 : Unicode (UTF-8) », délimiteur « Virgule ».
- LibreOffice Calc : à l'ouverture, jeu de caractères « Unicode (UTF-8) », séparateur « Virgule », langue « Anglais (USA) » pour lire les points décimaux.

Les dates sont en UTC : ajouter le décalage du fuseau pour retrouver l'heure locale.
