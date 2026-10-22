# Glossaire

Le code est en anglais, l'interface et la documentation en français. Chaque concept a un seul nom de chaque côté.

| Français               | Code                             | Sens                                                                                                  |
| ---------------------- | -------------------------------- | ----------------------------------------------------------------------------------------------------- |
| Cible                  | `Target`                         | Adresse sondée (serveur Internet, passerelle)                                                         |
| Sonde                  | `Probe`                          | Une tentative de mesure vers une cible                                                                |
| Échantillon            | `Sample`                         | Résultat d'une sonde : temps aller-retour ou perdu, horodaté                                          |
| Tronçon                | `Segment`                        | `Local` (ce Mac vers la box) ou `Internet` (de bout en bout)                                          |
| Fenêtre                | `Window`                         | Série glissante d'échantillons                                                                        |
| Métriques              | `Metrics`                        | Latence p90, gigue, perte, débit                                                                      |
| Niveau                 | `Grade`                          | Solide, Instable ou Dégradée, pour une métrique                                                       |
| Verdict                | `Verdict`                        | Niveau global, plus Hors ligne et Mesure en cours, avec sa cause                                      |
| Cause                  | `Diagnosis`                      | Tronçon en cause et métrique en défaut                                                                |
| Test de débit          | `ThroughputTest`                 | Mesure ponctuelle des débits montant et descendant                                                    |
| Réseau                 | `NetworkKey`                     | Connexion identifiée (MAC de la passerelle + type), qui segmente l'historique                         |
| Contexte               | `ConnectionContext`              | Type de connexion, partage de connexion, VPN, connexion limitée, Wi-Fi                                |
| Identité réseau        | `NetworkIdentity`                | MAC de la passerelle + type de connexion                                                              |
| Support                | `Medium`                         | Ce qui porte une interface selon le système : Wi-Fi, Ethernet, cellulaire, tunnel                     |
| Type de connexion      | `ConnectionKind`                 | Ce que l'utilisateur reconnaît : Ethernet, Wi-Fi, modem cellulaire, partage de connexion              |
| Connexion limitée      | `Metering`                       | Ce que le système dit du coût des données : payante, réduite, libre, inconnue                         |
| Tronçon en cause       | `Fault`                          | Lien local, opérateur, réseau mobile, ou indéterminé                                                  |
| Non mesurable          | `Unmeasurable`                   | Ping refusé, passerelle muette, ou pas de passerelle                                                  |
| Lien Wi-Fi             | `WifiLink`                       | Nom (s'il est lisible), signal, bruit, débit, canal et bande                                          |
| Agrégat                | `Aggregate`                      | Résumé d'une minute ou d'une heure : histogramme de latence, gigue, sondes, temps par état            |
| Granularité            | `Granularity`                    | Minute ou heure                                                                                       |
| Histogramme de latence | `LatencyHistogram`               | 24 tranches additionnables, d'où les percentiles de toute période                                     |
| Dernière heure         | `SecondRing`                     | Une mesure par seconde sur une heure, en mémoire                                                      |
| Événement              | `HistoryEvent`                   | Changement de réseau, début ou fin de coupure, mise en veille, réveil                                 |
| Veille, réveil         | `PowerEvent`                     | Avis du système avant la mise en veille et après le réveil                                            |
| Trou de mesure         | `WAKE_GAP_SECONDS`               | Plus de 10 s d'heure murale entre deux ticks ou pendant un tick : traité comme un réveil              |
| Profil de test         | `TestProfile`                    | Test rapide ou complet                                                                                |
| Transfert              | `Transfer`                       | Un envoi ou un téléchargement chronométré, avec ses allers-retours en charge                          |
| Latence en charge      | `loaded_latency`                 | Aller-retour mesuré pendant un test de débit, affiché sans compter dans l'état                        |
| Plafond de débit       | `throughput_cap`                 | Le niveau d'un test lent, qui tient l'état 10 min ou jusqu'au changement de réseau                    |
| Dégradation durable    | `DegradationWatch`               | Un état affiché pire que le précédent connu, confirmé : le seul cas notifié                           |
| Réglages               | `Settings`                       | Ouverture de session, apparence, serveurs de mesure                                                   |
| Apparence              | `Appearance`                     | Système, Clair ou Sombre                                                                              |
| Serveurs de mesure     | `MeasurementServers`             | Cloudflare et Google, ou un serveur personnalisé seul                                                 |
| Mémoire vive           | `LiveHistory`                    | Dernière heure en mémoire et réseau courant, lus auprès du moteur                                     |
| Période                | `Period`                         | 1 h, 24 h, 7 j ou 30 j : l'étendue d'une courbe et la largeur de ses points                           |
| Évolution              | `Evolution`                      | Les points d'une période, ses changements de réseau et sa couverture                                  |
| Couverture             | `coverage`                       | Part d'une période réellement mesurée                                                                 |
| Coupure                | `Outage`                         | Du début à la fin d'un passage Hors ligne ; sans fin connue si l'app s'est arrêtée                    |
| Pire moment            | `worst_minute`                   | La minute du jour la plus pénible pour un appel                                                       |
| Bilan d'un réseau      | `NetworkSummary`                 | 30 jours d'un réseau : temps par état, latence p90, perte, actuel, le plus fiable                     |
| Trafic observé         | `TrafficMeter`, `TrafficRate`    | Ce que l'interface reçoit et envoie, moyenné sur 5 s, avec le pic de la dernière heure ; hors verdict |
| Compteurs d'octets     | `ByteCounters`                   | Totaux d'octets reçus et envoyés d'une interface, lus auprès du système                               |
| Ligne de la box        | `BoxLine`                        | Ce que la box dit de sa ligne en UPnP : type d'accès, débit synchronisé, trafic total                 |
| Type d'accès           | `WanAccessType`                  | Valeur UPnP `WANAccessType` : seul « DSL » prouve quelque chose                                       |
| Débit synchronisé      | `downstream_max`, `upstream_max` | Débit de la ligne annoncé par la box, jamais mesuré par l'app                                         |
| Accès                  | `AccessEstimate`                 | ADSL annoncé par la box, fibre ou ADSL estimés, ou inconnu                                            |
| Période d'export       | `ExportPeriod`                   | 24 dernières heures, 7 ou 30 derniers jours, ou tout l'historique                                     |
| Effacement             | `erase_history`                  | Suppression de tout l'historique, réglages gardés                                                     |
| Date civile            | `CivilDate`, `CivilDateTime`     | Année, mois et jour (et heure, minute, seconde) d'un moment compté depuis le 1er janvier 1970         |
