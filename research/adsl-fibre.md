# Distinguer ADSL et fibre derrière la box (ticket #8)

## Réponse courte

1. **UPnP IGD seul n'est pas un détecteur fiable ADSL/fibre.** Le service standard `WANCommonInterfaceConfig:1` expose bien `GetCommonLinkProperties` (sorties `NewWANAccessType`, `NewLayer1UpstreamMaxBitRate`, `NewLayer1DownstreamMaxBitRate`), mais : (a) l'UPnP est un réglage de la box, pas toujours actif ; (b) aucune source primaire trouvée ne confirme ce que renvoient Freebox, Livebox, Bbox ou SFR Box ; (c) le standard n'a pas de valeur « fibre » : une box fibre voit son lien WAN comme `Ethernet` (l'ONT est un convertisseur), et AVM doit recourir à une extension propriétaire `X_AVM-DE_Fiber`.
2. **Crate Rust : `igd-next` ne convient pas** (API limitée aux redirections de ports, adresse externe, pare-feu IPv6). Utiliser **`rupnp`** (3.0.0, appel d'action SOAP générique) ou un client SOAP maison sur la description UPnP.
3. **Heuristique recommandée** : combiner débit montant soutenu + RTT vers le premier saut du FAI, afficher « estimé » seulement si les deux signaux concordent, sinon « inconnu ». Les seuils ci-dessous sont des propositions non validées sur le terrain.

## Spécification UPnP

- Service `urn:schemas-upnp-org:service:WANCommonInterfaceConfig:1` : variables `WANAccessType`, `Layer1UpstreamMaxBitRate`, `Layer1DownstreamMaxBitRate`, `PhysicalLinkStatus`. Spec : https://upnp.org/specs/gw/UPnP-gw-WANCommonInterfaceConfig-v1-Service.pdf (le PDF n'a pas pu être extrait en texte ici ; les valeurs ci-dessous viennent de sources secondaires).
- Valeurs standard de `WANAccessType` (de mémoire, **faiblement sourcé**) : `DSL`, `POTS`, `Cable`, `Ethernet`, `Unknown`. Preuve d'extensions propriétaires : la doc AVM liste `DSL, Ethernet, X_AVM-DE_Fiber, X_AVM-DE_UMTS, X_AVM-DE_Cable, X_AVM-DE_LTE` : https://fritz.com/fileadmin/user_upload/Global/Service/Schnittstellen/wancommonifconfigSCPD.pdf
- Un équipement Allied Telesis documente `WANAccessType Ethernet` : https://www.allied-telesis.co.jp/support/list/router/ar415s/docs/SHOW_UPNP_INTERFACE.html
- `Layer1*MaxBitRate` est le débit de synchronisation du lien physique côté box (ou une valeur configurée), pas un débit mesuré. Avec miniupnpd, ces valeurs peuvent être fixées par option de config (`-B`) : https://man.archlinux.org/man/miniupnpd.8.en.raw (le lien entre cette option et la valeur renvoyée n'est pas confirmé).
- Client de référence : miniupnpc expose `UPNP_GetLinkLayerMaxBitRates` (appelle `GetCommonLinkProperties`) : https://weghos.duckdns.org/godot/Godot/thirdparty/miniupnpc/include/upnpcommands.h.html

## Comportement par box (France)

| Box | UPnP IGD par défaut | `GetCommonLinkProperties` | Source / fiabilité |
|---|---|---|---|
| Freebox | Non établi. L'API Freebox permet d'activer/désactiver `upnpigd` (`enabled`, version 1 ou 2) ; l'exemple de la doc montre `false`, ce n'est pas une preuve du réglage d'usine. Un vieil article Clubic parle d'activation manuelle. | Non documenté. La doc ne mentionne que les redirections NAT. | https://dev.freebox.fr/sdk/os/igd ; https://www.clubic.com/actualite-266150-freebox-adsl-upnp-routage-de-ports-facilite.html. Faible. |
| Bbox | Actif par défaut selon un guide communautaire. | Non trouvé. | https://www.bbox-mag.fr/box/reglages-bbox-ne-pas-modifier/45008198 (site non officiel). Faible. |
| Livebox | Non trouvé. | Non trouvé. | Aucune source. |
| SFR Box | Non trouvé. | Non trouvé. | Aucune source. |

**Conclusion honnête** : aucune donnée vérifiable sur ce que ces quatre box renvoient pour `WANAccessType`. Il faut le mesurer sur du matériel réel (ou via retours d'utilisateurs) avant de s'y fier. Hypothèse de travail : sur une box fibre, `Ethernet` ou `Unknown` ; sur une box ADSL, `DSL` si le service est exposé et honnête. Seul `DSL` est un signal positif fort ; `Ethernet` ne permet pas de conclure à la fibre (câble et box en cascade renvoient aussi `Ethernet`).

## Crates Rust

- **`igd-next` 0.18.0** (fork de `rust-igd`, dépôt https://github.com/dariusc93/rust-igd) : méthodes publiques de `Gateway` = `add_port`, `add_any_port`, `remove_port`, `get_external_ip`, `get_generic_port_mapping_entry`, `get_firewall_status`, pinholes IPv6, etc. Ni `GetCommonLinkProperties`, ni `WANAccessType`, ni appel SOAP générique visibles sur https://docs.rs/igd-next/latest/igd_next/aio/struct.Gateway.html. Non utilisable pour ce besoin.
- **`rupnp` 3.0.0** (crates.io, mis à jour avril 2025) : découverte SSDP, lecture des descriptions de services (SCPD), `service.action(url, "NomAction", args_xml)` renvoyant une map de sorties. Doc : https://docs.rs/rupnp. Permet d'appeler `GetCommonLinkProperties` en chargeant le service `WANCommonInterfaceConfig:1`. Réserve : exemples d'API différents selon versions, à relire avant usage ; aucun appel réel compilé ici.
- Alternative : requête SOAP écrite à la main (HTTP POST + XML) sur l'URL de contrôle trouvée via SSDP/description du device.

## Heuristique sur les mesures (proposition, non validée)

Signaux et pourquoi ils discriminent (connaissance générale des technologies, pas de source primaire citée) :

- **Débit montant soutenu** : l'ADSL/ADSL2+ plafonne autour de 1 Mb/s (jusqu'à ~3 Mb/s en Annex M) ; la fibre FTTH offre en général plusieurs dizaines à centaines de Mb/s. Meilleur signal, mais bridé par le serveur de test et le plan tarifaire.
- **Symétrie débit montant/descendant** : ratio proche de 1 = fibre ; ratio < 0,1 = ADSL, câble ou VDSL (peu discriminant seul).
- **RTT vers le premier saut après la box (ou la passerelle FAI)** : l'ADSL ajoute typiquement 10 à 40 ms (entrelacement, DSLAM), la fibre quelques ms. Sensible à la charge, au Wi-Fi local et au serveur choisi.

Règle proposée :

| Condition | Résultat | Libellé |
|---|---|---|
| UPnP renvoie `DSL` | ADSL/VDSL | « ADSL » (source UPnP, affichée sans « estimé ») |
| Montant soutenu ≥ ~20 Mb/s et RTT premier saut ≤ ~8 ms | fibre (ou câble) | « fibre (estimé) » |
| Montant ≤ ~2 Mb/s et RTT premier saut ≥ ~10 ms | ADSL | « ADSL (estimé) » |
| Autre cas, ou un seul signal disponible, ou mesure bruitée | indécis | « inconnu » |

Niveau de confiance : « estimé » = deux signaux concordants sur plusieurs mesures consécutives (ex. médiane sur 5 tests). Pas de pourcentage affiché (faux sentiment de précision). Limites : fibre et câble DOCSIS se confondent ; VDSL2 (montant 5 à 50 Mb/s) tombe dans l'indécis ou se confond avec la fibre ; une connexion saturée fausse le RTT. Les seuils sont à étalonner sur quelques lignes réelles avant release.

## Risques

- UPnP désactivé ou service WAN absent : repli sur l'heuristique, prévoir ce cas comme normal.
- UPnP exposé expose aussi une surface de sécurité (redirections de ports) : l'app ne doit faire que de la lecture (jamais `AddPortMapping`).
- Valeurs `Layer1*MaxBitRate` parfois fixes/fausses (valeur configurée) : ne pas les afficher comme mesure.
- Box en cascade (double NAT) ou répéteur : l'IGD interrogé n'est pas forcément la box FAI.
- Découverte SSDP multicast : permissions macOS (réseau local) et pare-feu Windows/Linux à tester.

## Recommandation

1. Implémenter la détection avec `rupnp` : lecture seule de `GetCommonLinkProperties`, timeout court.
2. Ne traiter que `DSL` comme preuve directe ; `Ethernet`/`Unknown` ne concluent pas.
3. Appliquer l'heuristique de mesure en repli, avec « estimé » ou « inconnu » comme ci-dessus.
4. Ticket de suivi : relever les réponses réelles de Freebox, Livebox, Bbox, SFR Box (retours communauté ou matériel) pour remplir le tableau et étalonner les seuils.
