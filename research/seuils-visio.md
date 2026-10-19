# Seuils de qualité pour la visio (ticket #2)

## Réponse courte

Aucun éditeur ne publie de seuils « vert / jaune / rouge ». Les chiffres convergent toutefois sur un ordre de grandeur utilisable. Proposition de seuils par segment, sur **fenêtre glissante de 15 s, percentile 90 sur l'historique** :

| Métrique (vers Internet) | Solide | Instable | Dégradée |
|---|---|---|---|
| Latence RTT | < 100 ms | 100 à 300 ms | > 300 ms |
| Gigue (RFC 3550) | < 30 ms | 30 à 50 ms | > 50 ms |
| Perte de paquets | < 1 % | 1 à 5 % | > 5 % |
| Débit descendant / montant | >= 2,5 / 1,5 Mbit/s | 0,6 à 2,5 / 0,6 à 1,5 Mbit/s | < 0,6 / 0,6 Mbit/s |

Segment LAN (ordinateur vers passerelle) : mêmes classes de gigue et de perte, latence nominale < 10 ms ; au-delà de 30 ms de RTT ou 1 % de perte sur ce segment, le Wi-Fi est en cause. Le pire des deux segments fixe le verdict global (principe Microsoft : la qualité de bout en bout est celle du segment le plus mauvais).

Statut : les bornes de latence/gigue/perte sont **sourcées mais arbitrées ici** (les sources donnent « optimal » et « mauvais », pas trois classes). Les seuils de débit viennent des tableaux Teams/Zoom/Meet. Voir incertitudes.

## Sources et valeurs

| Source | Valeurs publiées | Lien |
|---|---|---|
| ITU-T G.114 (05/2003) | Délai unidirectionnel : <= 150 ms acceptable pour la plupart des usages, > 400 ms à éviter en planification. Le tampon de gigue compte dans le délai (annexe A.3). Valeurs 150/400 recoupées via des résumés secondaires ; la page ITU ne les affiche pas et le PDF n'a pas pu être extrait. | https://www.itu.int/rec/T-REC-G.114 |
| ITU-T G.1010 | Non vérifié (aucun texte primaire obtenu). Un ordre de grandeur 3GPP cité par des tiers (perte de trames <= 3 % pour la voix conversationnelle) reste secondaire. | https://www.itu.int/rec/T-REC-G.1010 |
| Microsoft Teams / Skype Entreprise, client vers bord du réseau Microsoft (page archivée 2017) | Optimal / Mauvais : RTT < 60 ms / > 500 ms ; perte max < 5 % / > 25 % ; perte moyenne < 0,5 % / > 10 % ; gigue < 3 ms / > 30 ms. Comparaison faite avec le **90e percentile** d'échantillons pris toutes les 10 min. Des relais citent aussi « RTT < 100 ms, perte < 1 %, gigue < 30 ms sur toute fenêtre de 15 s » : source secondaire seulement. | https://learn.microsoft.com/en-us/previous-versions/skypeforbusiness/optimizing-your-network/media-quality-and-network-connectivity-performance |
| Microsoft Teams, bande passante (mise à jour 2026) | Audio 10 à 76 kbit/s. Vidéo 1:1 : min 150, recommandé 1 500, max 4 000 kbit/s. Réunion : 150/200, 2 500/4 000, 4 000/4 000 (montant/descendant). Partage d'écran 1:1 : 200, 1 500, 4 000. L'audio est prioritaire quand la bande passante manque. | https://learn.microsoft.com/en-us/microsoftteams/prepare-network |
| Zoom, bande passante | 1:1 : 600 kbit/s (qualité haute), 1,2 Mbit/s (720p), 3,8 up / 3,0 down (1080p). Groupe : 1,0 up / 0,6 down, 2,6 / 1,8, 3,8 / 3,0. Audio VoIP 60 à 80 kbit/s. Aucune valeur de latence, gigue ou perte pour Meetings. | https://support.zoom.com/hc/en/article?id=zm_kb&sysparm_article=KB0060748 |
| Zoom, latence/gigue/perte | Non publiés pour Meetings. Valeurs répandues (latence <= 150 ms, gigue <= 40 ms, perte <= 2 %) venant de Zoom Phone ou de tiers. | https://community.zoom.com/t5/Zoom-Meetings/Zoom-Network-Monitoring/m-p/69544 |
| Google Meet | Vidéo : ~1 Mbit/s up, 1,3 Mbit/s down par participant ; 720p jusqu'à 1,7 Mbit/s, 1080p jusqu'à 3,6 Mbit/s ; audio seul ~12 à 100 kbit/s. Aucun seuil de latence, gigue ou perte. | https://knowledge.workspace.google.com/admin/meet/prepare-your-network-for-meet-meetings-and-live-streams |
| Apple « Responsiveness » (RPM), brouillon IETF | Mauvais < 300 RPM (~200 ms), moyen 300 à 1000 RPM (60 à 200 ms), bon 1000 à 6000 RPM (10 à 60 ms), excellent > 6000. Mesure sous charge. Fenêtre : intervalles de 5 s, moyenne mobile sur 4 intervalles, moyenne tronquée à 95 %, test d'environ 10 s. | https://www.ietf.org/archive/id/draft-ietf-ippm-responsiveness-09.html |
| Apple `networkQuality` (Low / Medium / High) | Libellés décrits qualitativement (Low : FaceTime peu fiable si un autre appareil charge le réseau). Seuils numériques Low/Medium/High **non publiés** ; un exemple isolé « Low (188 RPM) » existe. | https://tidbits.com/2022/04/22/use-apples-networkquality-tool-to-test-internet-responsiveness/ |

## Fenêtre de mesure

- Teams : fenêtre de 15 s pour gigue/perte (source secondaire) ; seuils vérifiés sur le 90e percentile de séries longues (source primaire).
- RPM : 5 s par intervalle, stabilité sur 4 intervalles, moyenne tronquée à 95 %.
- Reco : échantillonner à ~1 Hz, évaluer sur 15 s glissantes, afficher le verdict sur les 30 à 60 s précédentes avec hystérésis (passage au pire immédiat, retour au mieux après plusieurs fenêtres saines) pour éviter le clignotement de la pastille.

## Incertitudes

1. Les valeurs Teams les plus précises sont mesurées vers le bord Microsoft, avec une gigue < 3 ms « optimale » : très strict pour du Wi-Fi domestique ; 30 ms est leur seuil « mauvais », pas « bon ».
2. Zoom et Meet ne publient aucun seuil de qualité de paquets pour Meetings ; les 150/40/2 % circulent mais viennent de Zoom Phone ou de tiers.
3. G.1010 non lu, G.114 lu en second main (150/400 ms stables dans les sources secondaires).
4. Apple n'expose pas ses coupures Low/Medium/High ; les classes du brouillon IETF sont une analogie, pas une règle.
5. RPM mesure la latence sous charge (bufferbloat), pas la latence au repos : à mesurer séparément si l'appli veut le détecter.
6. Le débit ne se mesure pas passivement sans consommer de bande passante : décision à prendre au ticket de décision (test actif rare, ou verdict sans débit).
7. L'échelle Solide/Instable/Dégradée et les paliers 300 ms / 5 % / 50 ms sont des choix de conception sans source directe.

## Recommandation pour le ticket de décision (#11)

- Retenir le tableau ci-dessus comme valeurs par défaut, dans une configuration unique (pas de constantes dispersées).
- Verdict = pire métrique du pire segment, évalué sur 15 s glissantes en p90.
- Rendre le débit optionnel : s'il n'est pas mesuré, le verdict repose sur latence, gigue et perte seules.
- Prévoir un étalonnage sur des mesures réelles (Wi-Fi domestique) avant de figer les paliers.
