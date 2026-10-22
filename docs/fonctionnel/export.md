# Export

Le contenu des trois fichiers CSV et leur import dans un tableur.

## Exporter

Dans [Réglages](reglages.md#données), section « Données » : choisir la période, puis **Exporter…**, et choisir le dossier dans la fenêtre du système. La période se compte à partir du moment de l'export : 24 dernières heures, 7 derniers jours, 30 derniers jours (par défaut) ou Tout.

L'app écrit les trois fichiers dans ce dossier, en remplaçant des fichiers du même nom. Pendant l'écriture, le bouton affiche « Export en cours… » et ne se relance pas. À la fin, la page dit « 3 fichiers enregistrés. » et propose **Afficher le dossier**, qui l'ouvre dans le gestionnaire de fichiers. Fermer la fenêtre sans choisir de dossier n'exporte rien et ne dit rien. Si l'export échoue, ses fichiers sont retirés : « L'export n'a pas pu être écrit. »

Tant que l'historique est vide (premier lancement, ou juste après un effacement), le bouton est désactivé et la page dit « Rien à exporter pour l'instant. ».

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

Les en-têtes sont en anglais, comme les valeurs codées (type de connexion, événement, profil), pour qu'un script les lise sans traduction.

| Colonne            | Contenu                                                  |
| ------------------ | -------------------------------------------------------- |
| `start_utc`        | début de la minute ou de l'heure                         |
| `granularity`      | `minute` ou `hour`                                       |
| `network`          | nom du Wi-Fi                                             |
| `connection_type`  | `ethernet`, `wifi`, `cellular`, `phone_hotspot`, `other` |
| `latency_p50_ms`   | latence médiane, en millisecondes au dixième             |
| `latency_p90_ms`   | latence p90, en millisecondes au dixième                 |
| `jitter_mean_ms`   | gigue moyenne, en millisecondes au dixième               |
| `loss_pct`         | perte en pourcentage, deux décimales                     |
| `probes_sent`      | sondes envoyées                                          |
| `probes_lost`      | sondes perdues                                           |
| `seconds_solid`    | secondes passées en Solide                               |
| `seconds_unstable` | secondes passées en Instable                             |
| `seconds_degraded` | secondes passées en Dégradée                             |
| `seconds_offline`  | secondes passées Hors ligne                              |

Les secondes de « Mesure en cours » ne comptent dans aucun état : la somme des quatre dernières colonnes peut rester sous 60 ou 3 600.

## `evenements.csv`

| Colonne           | Contenu                                                                |
| ----------------- | ---------------------------------------------------------------------- |
| `time_utc`        | moment de l'événement, à la seconde                                    |
| `network`         | nom du Wi-Fi                                                           |
| `connection_type` | comme dans `mesures.csv`                                               |
| `event`           | `network_changed`, `offline_started`, `offline_ended`, `sleep`, `wake` |

## `tests-debit.csv`

Les [tests de débit](tests-de-debit.md) terminés.

| Colonne             | Contenu                              |
| ------------------- | ------------------------------------ |
| `time_utc`          | fin du test                          |
| `network`           | nom du Wi-Fi                         |
| `connection_type`   | comme dans `mesures.csv`             |
| `profile`           | `quick` (rapide) ou `full` (complet) |
| `download_kbps`     | débit en réception, en kbit/s        |
| `upload_kbps`       | débit en envoi, en kbit/s            |
| `loaded_latency_ms` | latence en charge, en millisecondes  |
| `bytes`             | données consommées                   |
| `duration_ms`       | durée du test, en millisecondes      |

## Ouvrir dans un tableur

Un tableur réglé en français attend en général le point-virgule et la virgule décimale : ouvrir le fichier par l'import de texte plutôt que par un double clic, puis choisir l'encodage UTF-8, la virgule comme séparateur et le point comme séparateur décimal.

- Excel : Données, À partir d'un fichier texte/CSV ; origine du fichier « 65001 : Unicode (UTF-8) », délimiteur « Virgule ».
- LibreOffice Calc : à l'ouverture, jeu de caractères « Unicode (UTF-8) », séparateur « Virgule », langue « Anglais (USA) » pour lire les points décimaux.

Les dates sont en UTC : ajouter le décalage du fuseau pour retrouver l'heure locale.
