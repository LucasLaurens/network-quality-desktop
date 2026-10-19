# Test de débit : quel service / protocole ? (ticket #4)

## Réponse courte

- **Test à la demande (tous OS)** : endpoints Cloudflare `speed.cloudflare.com/__down` et `/__up` appelés directement depuis Rust (`reqwest`), avec une séquence de tailles réduite (mode « léger »). Pas de client Rust officiel, mais le protocole est trivial (GET/POST HTTP) et le moteur de référence (`@cloudflare/speedtest`, MIT) documente l'algorithme. Réserve : endpoints non présentés comme une API publique stable.
- **Test périodique** : à éviter en M-Lab (quota 4 tests/jour pour un client automatisé). Utiliser le mode léger Cloudflare (quelques Mo), désactivé sur hotspot / connexion limitée.
- **ndt7 (M-Lab)** : meilleur choix « propre » côté licence/gouvernance, seul à avoir un client Rust (`ndt7-client`), mais volume élevé (~10 s par sens) et quota automatique très bas. Candidat de repli pour le test manuel.
- **`networkQuality` (Apple)** : bonus macOS optionnel (RPM, JSON), pas une base multi-plateforme.
- **Ookla** : à exclure (licence personnelle non commerciale, binaire fermé, non redistribuable).

## Tableau comparatif

| Critère | Cloudflare (`__down`/`__up`) | M-Lab ndt7 | Apple `networkQuality` | Ookla Speedtest CLI |
|---|---|---|---|---|
| Plateformes | toutes (HTTP) | toutes (WebSocket TLS) | macOS 12+ seulement | binaire par OS |
| Client Rust | aucun officiel ; HTTP simple à écrire. Crates tierces récentes et peu utilisées | `ndt7-client` 0.3.1 (Apache-2.0, ~210 téléchargements, jeune mais actif) | aucun ; on lance le binaire système (`networkQuality -c` donne du JSON) | aucun ; binaire fermé |
| Durée | adaptative : on arrête de monter en taille dès qu'un palier dépasse ~1 s ; typ. 10-30 s | ~10 s par sens (max ~13 s), sens séquentiels | ~15 s (mesuré ici : 16,8 s avec `-M 15`) ; `-M` borne la durée | ~15-30 s |
| Volume de données | jusqu'à ~1,27 Go théoriques sur lien très rapide avec la séquence par défaut ; ~10-20 Mo en mode léger | proportionnel au débit : ~10 s x débit x 2 sens (ex. 100 Mbit/s = ~250 Mo) | proportionnel au débit (flux parallèles saturants), non borné en octets | proportionnel au débit |
| Test léger possible | oui : on choisit les tailles (100 ko, 1 Mo) | partiel : le client peut fermer plus tôt, la spec ne fixe pas de plafond d'octets | partiel : `-M` (durée), `-d`/`-u` pour un seul sens | non |
| Précision | bonne ; percentile 90 sur les requêtes, latence en charge | bonne (un seul flux TCP, métriques TCP côté serveur) ; peut sous-estimer sur liens très rapides | bonne, multi-flux, mesure aussi la réactivité (RPM) | référence du marché |
| Licence / CGU | voir ci-dessous | AUP stricte mais favorable à l'open source | outil système, serveurs Apple | restrictive |
| Serveurs | CDN anycast Cloudflare | flotte M-Lab via `locate.measurementlab.net` | serveurs Apple par défaut ; serveur open source possible (`-C`) | réseau Ookla |

## Conditions d'utilisation

**Cloudflare.** Aucun texte spécifique « speed test » trouvé. Les conditions des sites/services en ligne (https://www.cloudflare.com/website-terms/) interdisent de dépasser ou contourner les limites d'API et de surcharger les services, sans interdire globalement l'accès automatisé hors usage IA/scraping. Les endpoints `/__down` et `/__up` sont les valeurs par défaut du module officiel MIT https://github.com/cloudflare/speedtest, mais ne sont pas documentés comme API publique supportée ; des projets tiers les qualifient eux-mêmes de non documentés (risque de rupture). Le README précise que le moteur officiel envoie les résultats à Cloudflare pour des statistiques agrégées (un client maison n'en envoie pas). Risques : limitation de débit ou changement sans préavis ; mitigation par fréquence faible et repli vers ndt7.

**M-Lab ndt7.** AUP : https://www.measurementlab.net/aup/ (mise à jour juillet 2025). Points pertinents :
- clients logiciels sous licence approuvée OSI : compatible avec une app open source ;
- plafonds : 40 tests/jour en usage interactif, **4 tests/jour pour un client automatisé**, horaires à randomiser, consentement utilisateur requis pour les tests automatiques répétés ;
- les résultats (dont l'IP du client) sont publiés dans le domaine public : politique de confidentialité et mention RGPD exigées ;
- mesures actives uniquement ; usage commercial possible avec contribution ; pas de rémunération des testeurs.

La spec ndt7 (https://github.com/m-lab/ndt-server/blob/main/spec/ndt7-protocol.md, v0.11.0) recommande pour les clients non interactifs un intervalle exponentiel de moyenne 6 h (bornes 36 min à 15 h) et un backoff sur HTTP 204 (pas de capacité).

**Apple `networkQuality`.** Page de man locale : outil du système (macOS 12+), « utilise des données de votre forfait ». Serveurs par défaut d'Apple ; pas de CGU d'usage tiers trouvées. Un serveur open source existe (https://github.com/network-quality/server) et le protocole est en draft IETF (responsiveness), utilisable via `-C`. Lancer le binaire local depuis l'app est un simple appel de processus, sans redistribution.

**Ookla.** Les pages officielles (speedtest.net) n'ont pas pu être consultées depuis cet environnement (accès refusé). D'après des sources secondaires (miroirs de la licence Gentoo, fil OpenWrt, wrappers npm), le CLI est réservé à un usage personnel non commercial en ligne de commande sur un ordinateur personnel, avec interdiction de le rendre accessible à des tiers ou de l'installer sur routeur. **Non vérifié à la source** ; dans tous les cas, un binaire fermé ne s'embarque pas dans une app open source multi-plateforme. Exclu.

## Estimation de volume (Cloudflare, séquence par défaut)

Somme des octets de la config par défaut du README `@cloudflare/speedtest` : download ~0,97 Go + upload ~0,30 Go si tous les paliers sont exécutés. En pratique le moteur cesse de monter en taille quand un palier dépasse 1 s (`bandwidthFinishRequestDuration`), donc le volume réel dépend du débit. Mode léger proposé (à valider au ticket de décision) : paliers 100 ko et 1 Mo seulement (ex. 10 x 100 ko + 8 x 1 Mo en download, idem en upload), soit ~10-20 Mo. Précision réduite sur les liens > 100 Mbit/s : ce mode sert à détecter une dégradation, pas à mesurer le débit maximal.

## Recommandation pour le ticket de décision (#13)

1. Fournisseur principal : Cloudflare via HTTP direct (`reqwest`), deux profils : « à la demande » (paliers jusqu'à 25-100 Mo, durée plafonnée) et « périodique léger » (paliers <= 1 Mo).
2. Repli / alternative : ndt7 via `ndt7-client` pour le test manuel uniquement, jamais en périodique (quota 4/jour), avec mention RGPD dans l'UI.
3. Optionnel macOS : `networkQuality -c -M <n>` pour afficher le RPM ; ne pas en dépendre.
4. Ookla : exclu.
5. Risques à acter : endpoints Cloudflare non contractuels ; métriques non comparables entre fournisseurs. La désactivation hotspot / connexion limitée est hors périmètre de ce ticket.

## Limites de cette recherche

- Pages Ookla inaccessibles ; licence Ookla non confirmée à la source.
- Volumes Cloudflare/ndt7 calculés depuis la config/la spec, pas mesurés. `networkQuality` exécuté une fois : 16,8 s au total, ~42 Mbit/s down et ~12,7 Mbit/s up sur le lien de la machine, volume non relevé.
- Maturité de `ndt7-client` (crate récente, peu téléchargée) à auditer avant adoption.
