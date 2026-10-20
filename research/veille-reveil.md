# Détecter la veille et le réveil sur 3 OS depuis Rust (#19)

Ticket : https://github.com/LucasLaurens/network-quality-desktop/issues/19 (carte : #1)
Recherche faite le 2026-10-20. Les points non confirmés par une source primaire sont marqués **[non vérifié]**.

## Réponse courte

- Les 3 OS offrent une notification « avant veille » et une « après réveil » utilisable sans fenêtre et sans bloquer l'event loop Tauri, à condition de l'héberger sur **un thread dédié** qui parle à l'acteur tokio par un canal.
- Aucune crate multiplateforme mature ne couvre le besoin. Seule `powerwatch` 0.1.0 (publiée 2026-09-04, 18 téléchargements, un seul mainteneur) s'en approche : trop récente, on ne la prend pas. Les plugins Tauri existants (`tauri-plugin-nosleep`) *empêchent* la veille, ils ne la signalent pas.
- Budget de temps avant veille : **~2 s** Windows, **~5 s** Linux (délai logind), **30 s** macOS (IOKit uniquement). Le flush d'une minute d'agrégat dans une transaction SQLite (quelques ms) tient dans les trois budgets. On l'écrit pourtant de façon à ne rien perdre si le délai est dépassé (voir Risques).
- Recommandation : un port `PowerEventSource` (séparé de `NetworkChangeWatcher`, voir plus bas), 3 adaptateurs dans `infrastructure/` (`#[cfg(target_os)]`), plus un `InMemoryPowerEventSource` pour les tests.

## Tableau par OS

| | macOS | Windows | Linux |
|---|---|---|---|
| Avant veille | IOKit `kIOMessageSystemWillSleep` (+ `kIOMessageCanSystemSleep` pour la veille d'inactivité). NSWorkspace `NSWorkspaceWillSleepNotification` (informatif) | `PBT_APMSUSPEND` | logind `PrepareForSleep(true)` |
| Après réveil | IOKit `kIOMessageSystemHasPoweredOn` (réveil terminé) ; `WillPowerOn` = début. NSWorkspace `NSWorkspaceDidWakeNotification` | `PBT_APMRESUMEAUTOMATIC` (toujours envoyé), `PBT_APMRESUMESUSPEND` (en plus si action utilisateur) | `PrepareForSleep(false)` |
| Peut retarder la veille | IOKit oui (`IOAllowPowerChange`). NSWorkspace non | Non (notification seulement, pas d'acquittement) | Oui, via un verrou inhibiteur `delay` |
| Délai accordé | 30 s pour acquitter, sinon la veille est repoussée de 30 s (WillSleep) ou démarre après 30 s (CanSystemSleep) | environ 2 s, passé ce délai le système « peut interrompre » l'appli | 5 s par défaut (`InhibitDelayMaxUSec` = 5000000 observé, configurable dans `logind.conf`) |
| API sans fenêtre | IOKit : oui. NSWorkspace : oui mais sur la run loop principale | `PowerRegisterSuspendResumeNotification` ou `RegisterSuspendResumeNotification` avec `DEVICE_NOTIFY_CALLBACK` : oui. `WM_POWERBROADCAST` : exige une fenêtre | D-Bus, aucune fenêtre |
| Modèle de thread | callback sur la run loop du thread qui a ajouté la source | callback fourni par le système : thread exact **[non vérifié]** | flux de signaux async sur le runtime tokio |
| Crates | `objc2-app-kit` 0.3.2 (feature `NSWorkspace`), `objc2-io-kit` 0.3.2, `block2` 0.6.2, `objc2` 0.6.5 | `windows` 0.62.2 (ou `windows-sys` 0.61.2) | `zbus` 6.0.0 (feature `tokio`), ou `logind-zbus` 5.3.2 |
| Version mini de l'OS | non précisée | Windows 8 (API callback), Windows XP (`WM_POWERBROADCAST`) | systemd/logind |

## Détails par plateforme

### macOS

Source : Apple QA1340, https://developer.apple.com/library/archive/qa/qa1340/_index.html (page de 2014, toujours la référence).

- NSWorkspace (observer sur le centre de notifications de `NSWorkspace`, pas sur le centre par défaut sinon rien n'arrive) : simple, mais **purement informatif**, il ne permet pas de retarder la veille. Les statics sont dans `objc2-app-kit` derrière la feature `NSWorkspace` (https://docs.rs/objc2-app-kit/latest/objc2_app_kit/static.NSWorkspaceWillSleepNotification.html). Le bloc d'observation passe par `block2`. Aucune garantie que le flush ait fini avant la coupure : **à ne pas utiliser pour le flush**, seulement en complément pour le réveil.
- IOKit `IORegisterForSystemPower` : seul chemin qui permet d'acquitter. Il faut ajouter la source de run loop de l'`IONotificationPort` à une run loop et la faire tourner ; Apple précise de ne pas appeler `CFRunLoopRun()` sur le thread principal d'une appli Cocoa. Donc : **thread dédié** avec sa propre run loop (alternative par dispatch queue : **[non vérifié côté Rust]**). Dans le callback `WillSleep`, on envoie l'ordre de flush à l'acteur, on attend l'acquittement (avec timeout court), puis `IOAllowPowerChange`. `IOCancelPowerChange` sur `WillSleep` est sans effet : la machine dort quand même.
- Bindings Rust : `objc2-io-kit` 0.3.2 expose IOKit ; la couverture exacte de `IORegisterForSystemPower` et `IOAllowPowerChange` dans cette crate est **[non vérifié]**. Repli : FFI `extern "C"` minimal (3 fonctions) ou `io-kit-sys` 0.5.0.
- Pas de privilège ni d'entitlement mentionné par QA1340. Sandbox App Store **[non vérifié]** (hors périmètre : appli open source).

### Windows

Sources : https://learn.microsoft.com/en-us/windows/win32/power/wm-powerbroadcast , https://learn.microsoft.com/en-us/windows/win32/power/pbt-apmsuspend , https://learn.microsoft.com/en-us/windows/win32/api/powerbase/nf-powerbase-powerregistersuspendresumenotification , https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-registersuspendresumenotification

- `WM_POWERBROADCAST` : arrive dans la WindowProc d'une fenêtre. Tauri a une fenêtre mais son event loop (tao) ne l'expose pas comme événement public : s'y brancher demande un sous-classement de fenêtre, fragile et couplé à tao **[non vérifié : absence d'événement power dans tao]**. Écarté.
- `PowerRegisterSuspendResumeNotification(DEVICE_NOTIFY_CALLBACK, ...)` (powerbase.h, Windows 8+, desktop seulement) : enregistre un callback ; `Type` vaut `PBT_APMSUSPEND`, `PBT_APMRESUMESUSPEND` ou `PBT_APMRESUMEAUTOMATIC`. Aucune fenêtre. `RegisterSuspendResumeNotification` (user32) est le jumeau qui accepte aussi un `HWND`. Les deux sont équivalentes en mode callback. Dans la crate `windows`, la fonction se trouve a priori sous `Win32::System::Power` avec la feature `Win32_System_Power` **[non vérifié]**.
- Délai : « environ deux secondes » pour traiter `PBT_APMSUSPEND`, après quoi le système peut interrompre l'appli. Pas d'acquittement : le callback doit rendre la main vite. Il ne fait que poster un message dans un canal et attendre l'acquittement de l'acteur au plus ~1 s.
- `PBT_APMRESUMEAUTOMATIC` est envoyé à chaque reprise ; `PBT_APMRESUMESUSPEND` seulement en plus en cas d'action utilisateur. Écouter uniquement `RESUMEAUTOMATIC` pour ne pas dupliquer.
- Modern Standby (S0 low power idle) : la doc Microsoft décrit un état où les applis de bureau sont quiescées puis reprennent à l'allumage de l'écran (https://learn.microsoft.com/en-us/windows-hardware/design/device-experiences/modern-standby). Que `PBT_APMSUSPEND` y soit toujours délivré avec le même délai est **[non vérifié]** : à tester sur une machine Modern Standby réelle. Prévoir de toute façon le repli « trou de mesure » (voir Risques).

### Linux

Sources : https://systemd.io/INHIBITOR_LOCKS/ , https://www.freedesktop.org/software/systemd/man/latest/systemd-inhibit.html

- Au démarrage, appeler `org.freedesktop.login1.Manager.Inhibit("sleep", who, why, "delay")` sur le bus **système** ; la méthode renvoie un descripteur de fichier. Tant qu'il est ouvert, logind retarde la veille (`delay` : temporaire, jusqu'à libération ou expiration du délai).
- S'abonner au signal `PrepareForSleep(b)` : `true` = la veille démarre, il faut finir vite puis **fermer le fd** ; `false` = cycle terminé (réveil), et on **reprend un nouveau verrou** pour le cycle suivant. Écouter `true` sans verrou delay n'est pas fiable.
- Délai : 5 s observé par défaut (`InhibitDelayMaxUSec` = 5000000, systemd.io), réglable dans `logind.conf` ; la page ne l'écrit pas comme valeur par défaut explicite, d'où une prudence légère.
- `zbus` 6.0.0 : feature `tokio` pour tourner sur le runtime existant sans thread supplémentaire (https://docs.rs/zbus/latest/zbus/). `#[proxy]` génère un proxy typé ; le fd retourné se manipule via `zbus::OwnedFd` (usage exact **[non vérifié]**, pas d'exemple dans la page). Alternative : `logind-zbus` 5.3.2 (proxys logind prêts, dernière publication mai 2025, compatibilité avec zbus 6 **[non vérifié]**) ; `zbus_systemd` 0.26200.0 (sept. 2026) couvre aussi logind.
- Pas de logind (Alpine, conteneurs, distributions sans systemd) : aucune notification. Le watcher se dégrade en no-op avec un log, sans planter. elogind **[non vérifié]** (expose a priori la même interface D-Bus).
- Ne dépend ni de GNOME, ni de KDE, ni de Wayland/X11.

## Crates existantes multiplateformes

| Crate | Verdict |
|---|---|
| `powerwatch` 0.1.0 (MIT, 2026-09-04, 18 téléchargements, dépôt bradbury/powermonitor) | « événements d'alimentation et de session sans event loop GUI », API `PowerWatch::start` + canal `flume`. Les variantes de `PowerEvent`, les plateformes supportées et la capacité de **retarder** la veille ne sont **pas confirmés** par la doc. Trop jeune, un seul mainteneur : à ne pas adopter. À relire si elle mûrit |
| `tauri-plugin-nosleep` 0.1.0 | Fait l'inverse (empêche la veille). Hors sujet |
| Plugins Tauri officiels | Pas de plugin veille/réveil trouvé dans les résultats crates.io **[non vérifié : recherche limitée]** |
| Autres recherches crates.io (« suspend resume », « sleep wake », « tauri plugin power ») | Pas d'autre résultat pertinent |

Conclusion : écrire les 3 adaptateurs soi-même (quelques dizaines de lignes chacun). Le coût est inférieur à celui d'auditer et suivre une crate de 18 téléchargements, et le port isole le choix.

## Intégration avec Tauri et l'acteur de mesure

Principe : jamais de logique métier dans le callback OS. Le callback poste un `PowerEvent` dans un canal, rien d'autre.

1. Chaque adaptateur vit sur son propre thread ou sa tâche (macOS IOKit : run loop dédiée ; Windows : callback fourni par le système ; Linux : tâche tokio zbus).
2. Le callback envoie `PowerEvent::WillSleep { ack }` ou `PowerEvent::DidWake` dans un canal vers l'acteur.
3. Sur `WillSleep`, l'acteur ferme la minute courante, demande au writer SQLite de l'écrire en une transaction, puis déclenche `ack`. L'adaptateur attend `ack` avec un timeout (macOS ~5 s par prudence, Windows ~1 s, Linux ~3 s) puis libère : `IOAllowPowerChange` / retour du callback / fermeture du fd.
4. Sur `DidWake`, l'acteur passe le verdict à « Mesure en cours », rejette les échantillons en vol (ils traversent la veille et sont faux) et relance une mesure immédiate.

Rien de tout cela ne touche l'event loop Tauri. Ne jamais appeler le front depuis le callback OS ; passer par l'acteur, qui possède déjà ce chemin.

### Le port

```rust
// application/ports/power_event_source.rs  (esquisse, pas de code applicatif dans ce ticket)
pub enum PowerEvent { WillSleep(SleepAck), DidWake }
pub trait PowerEventSource { fn subscribe(&self) -> Receiver<PowerEvent>; }
```

- Port **séparé** de `NetworkChangeWatcher` : les changements de réseau et la veille ont des sources, cycles de vie et cadences différents (responsabilité unique). L'acteur reçoit deux flux dans son `select!`. Si le ticket sur `NetworkChangeWatcher` retient un enum d'événements unique, `PowerEvent` peut y devenir une variante sans changer les adaptateurs.
- Adaptateurs dans `infrastructure/` : `MacosPowerEventSource` (IOKit, complété par NSWorkspace si besoin), `WindowsPowerEventSource`, `LogindPowerEventSource`, plus un `NullPowerEventSource` (Linux sans logind).
- Test : `InMemoryPowerEventSource` concret, sans bibliothèque de mock, avec `simulate_will_sleep()` qui renvoie un `SleepAck` observable et `simulate_wake()`. Les tests de l'acteur vérifient : minute écrite avant l'acquittement, verdict « Mesure en cours » après `DidWake`, mesure relancée, échantillons en vol rejetés.
- Les adaptateurs OS ne se testent qu'à la main ou en CI par OS (une veille réelle n'est pas automatisable de façon fiable **[non vérifié]**). On les garde minces pour que cela suffise.

## Risques

1. **Délai dépassé** (surtout Windows, ~2 s) : le flush peut être interrompu. Mitigation : le writer écrit déjà chaque minute close ; l'acteur peut en plus persister l'agrégat partiel de la minute en cours à intervalle court, pour borner la perte même sans notification.
2. **Notification manquée** (Modern Standby non confirmé, Linux sans logind, hibernation, coupure de courant) : prévoir une détection indépendante du trou de mesure : si l'écart entre deux ticks dépasse un seuil (plusieurs fois l'intervalle), traiter comme un réveil. Ce filet couvre aussi les cas non vérifiés ci-dessus.
3. **Course au réveil** : le réseau n'est souvent pas encore remonté. Passer à « Mesure en cours », puis laisser `NetworkChangeWatcher` ou le premier échec de mesure décider, plutôt que conclure « hors ligne ».
4. **Linux** : reprendre le verrou `delay` après chaque `PrepareForSleep(false)`, sinon la deuxième veille n'est plus retardée. Un fd qui fuit retarde la veille jusqu'à expiration à chaque fois.
5. **macOS** : ne pas bloquer la run loop IOKit en attendant l'acteur sans timeout ; ne pas lancer `CFRunLoopRun` sur le thread principal.
6. **Unsafe/FFI** : les 3 adaptateurs concentrent de l'`unsafe`. Un module par OS, seul à contenir le FFI, un commentaire SAFETY par bloc.

## Recommandation

1. Port `PowerEventSource` séparé, 3 adaptateurs + `Null` + `InMemory`.
2. macOS : IOKit `IORegisterForSystemPower` sur thread dédié (vrai « avant veille » avec acquittement) ; NSWorkspace seulement en complément. Windows : `PowerRegisterSuspendResumeNotification` en mode callback. Linux : `zbus` 6 (feature `tokio`) + verrou `delay` repris à chaque cycle.
3. Ne pas adopter `powerwatch` (trop récent) ; ne pas se coupler à tao pour `WM_POWERBROADCAST`.
4. Ajouter le filet « trou de mesure » indépendant des notifications.
5. À lever en implémentation (non résolu ici) : couverture IOKit de `objc2-io-kit`, comportement de Modern Standby, compatibilité `logind-zbus` / zbus 6.

## Sources

- Apple QA1340 : https://developer.apple.com/library/archive/qa/qa1340/_index.html
- objc2-app-kit : https://docs.rs/objc2-app-kit/latest/objc2_app_kit/static.NSWorkspaceWillSleepNotification.html
- WM_POWERBROADCAST : https://learn.microsoft.com/en-us/windows/win32/power/wm-powerbroadcast
- PBT_APMSUSPEND : https://learn.microsoft.com/en-us/windows/win32/power/pbt-apmsuspend
- PowerRegisterSuspendResumeNotification : https://learn.microsoft.com/en-us/windows/win32/api/powerbase/nf-powerbase-powerregistersuspendresumenotification
- RegisterSuspendResumeNotification : https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-registersuspendresumenotification
- Modern Standby : https://learn.microsoft.com/en-us/windows-hardware/design/device-experiences/modern-standby
- Verrous inhibiteurs systemd : https://systemd.io/INHIBITOR_LOCKS/
- systemd-inhibit : https://www.freedesktop.org/software/systemd/man/latest/systemd-inhibit.html
- zbus : https://docs.rs/zbus/latest/zbus/
- powerwatch : https://docs.rs/powerwatch/0.1.0/powerwatch/ , https://crates.io/crates/powerwatch
- Versions relevées via l'API crates.io le 2026-10-20 : tauri 2.12.4, zbus 6.0.0, objc2 0.6.5, objc2-app-kit 0.3.2, objc2-io-kit 0.3.2, block2 0.6.2, windows 0.62.2, windows-sys 0.61.2, logind-zbus 5.3.2, io-kit-sys 0.5.0.
