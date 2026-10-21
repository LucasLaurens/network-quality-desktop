# Glossaire

Le code est en anglais, l'interface et la documentation en français. Chaque concept a un seul nom de chaque côté.

| Français               | Code                | Sens                                                                                       |
| ---------------------- | ------------------- | ------------------------------------------------------------------------------------------ |
| Cible                  | `Target`            | Adresse sondée (serveur Internet, passerelle)                                              |
| Sonde                  | `Probe`             | Une tentative de mesure vers une cible                                                     |
| Échantillon            | `Sample`            | Résultat d'une sonde : temps aller-retour ou perdu, horodaté                               |
| Tronçon                | `Segment`           | `Local` (ce Mac vers la box) ou `Internet` (de bout en bout)                               |
| Fenêtre                | `Window`            | Série glissante d'échantillons                                                             |
| Métriques              | `Metrics`           | Latence p90, gigue, perte, débit                                                           |
| Niveau                 | `Grade`             | Solide, Instable ou Dégradée, pour une métrique                                            |
| Verdict                | `Verdict`           | Niveau global, plus Hors ligne et Mesure en cours, avec sa cause                           |
| Cause                  | `Diagnosis`         | Tronçon en cause et métrique en défaut                                                     |
| Test de débit          | `ThroughputTest`    | Mesure ponctuelle des débits montant et descendant                                         |
| Réseau                 | `NetworkKey`        | Connexion identifiée (MAC de la passerelle + type), qui segmente l'historique              |
| Contexte               | `ConnectionContext` | Type de connexion, partage de connexion, VPN, connexion limitée, Wi-Fi                     |
| Identité réseau        | `NetworkIdentity`   | MAC de la passerelle + type de connexion                                                   |
| Support                | `Medium`            | Ce qui porte une interface selon le système : Wi-Fi, Ethernet, cellulaire, tunnel          |
| Type de connexion      | `ConnectionKind`    | Ce que l'utilisateur reconnaît : Ethernet, Wi-Fi, modem cellulaire, partage de connexion   |
| Connexion limitée      | `Metering`          | Ce que le système dit du coût des données : payante, réduite, libre, inconnue              |
| Tronçon en cause       | `Fault`             | Lien local, opérateur, réseau mobile, ou indéterminé                                       |
| Non mesurable          | `Unmeasurable`      | Ping refusé, passerelle muette, ou pas de passerelle                                       |
| Lien Wi-Fi             | `WifiLink`          | Nom (s'il est lisible), signal, bruit, débit, canal et bande                               |
| Agrégat                | `Aggregate`         | Résumé d'une minute ou d'une heure : histogramme de latence, gigue, sondes, temps par état |
| Granularité            | `Granularity`       | Minute ou heure                                                                            |
| Histogramme de latence | `LatencyHistogram`  | 24 tranches additionnables, d'où les percentiles de toute période                          |
| Dernière heure         | `SecondRing`        | Une mesure par seconde sur une heure, en mémoire                                           |
| Événement              | `HistoryEvent`      | Changement de réseau, début ou fin de coupure                                              |
