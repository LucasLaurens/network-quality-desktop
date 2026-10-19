# Tauri 2 en tâche de fond sur 3 OS (ticket #9)

Date de recherche : 2026-10-19. Versions lues sur crates.io le même jour.

## Réponse courte

Tauri 2 couvre presque tout avec le cœur + 3 plugins officiels (positioner, autostart, notification). Deux trous : **détection veille/réveil** (aucun plugin officiel, il faut du code natif par OS ou un crate tiers) et **Linux/GNOME** (tray absent sans extension, événements de clic et position du tray non supportés). Point d'architecture décisif : la boucle de mesure doit vivre **côté Rust** (tâche tokio), jamais dans le webview, qui est bridé quand la fenêtre est cachée.

## Tableau capacité x OS

| Capacité | macOS | Windows | Linux |
|---|---|---|---|
| Icône de tray dynamique (`TrayIcon::set_icon`) | Oui. `set_icon_with_as_template` pour changer icône + mode template sans flash | Oui | Oui via libayatana/appindicator (icône écrite sur disque, `set_temp_dir_path`) ; rendu non vérifié |
| Dot coloré | Oui avec `icon_as_template(false)` (une icône template est monochrome) | Oui | Oui si le tray est affiché |
| Clic sur le tray (`on_tray_icon_event`) | Oui (Click, Enter, Move, Leave, avec `rect`) | Oui | **Non** : non émis, seul le menu contextuel marche (doc officielle) |
| Popover ancré (positioner `Tray*`) | Oui | Oui | **Non fiable** : le plugin utilise le `rect` des événements, absent sous Linux. Fallback : coin d'écran (`TopRight`/`BottomRight`) |
| Fermer sans quitter | Oui | Oui | Oui |
| Cacher l'icône du Dock | Oui (`set_activation_policy(Accessory)` / `set_dock_visibility(false)`) | n/a (`skip_taskbar`) | n/a (`skip_taskbar`) |
| Lancement au login | Oui : LaunchAgent (défaut) ou AppleScript (login item) | Oui : valeur de registre `Run` | Oui (crate `auto-launch`, mécanisme non détaillé ; AppImage géré) |
| Notifications natives | Oui ; en dev l'identité est `com.apple.Terminal` | Oui, **apps installées seulement** ; en dev affiche PowerShell | Oui (D-Bus, notify-rust) |
| Bridage du process en fond | App Nap | QoS Low si fenêtre minimisée/masquée, EcoQoS/efficiency mode | Pas d'équivalent standard |
| Bridage du webview caché | WebKit suspend par défaut ; option `backgroundThrottling` (macOS 14+) | WebView2 bridé ; option non supportée | WebKitGTK ; option non supportée |
| Veille/réveil | `NSWorkspace` willSleep/didWake | `WM_POWERBROADCAST` | logind D-Bus `PrepareForSleep` |

## Détails et sources

### Tray dynamique
- Feature Cargo `tray-icon`. `TrayIcon` expose `set_icon`, `set_icon_as_template`, `set_icon_with_as_template`, `set_title`, `set_tooltip`, `set_visible`, `rect` (docs.rs, tauri 2.12.4). Sur macOS, enchaîner `set_icon` puis `set_icon_as_template` provoque un flash visible : utiliser la variante atomique. Plusieurs méthodes sont annotées « Linux: Unsupported » (titre sans icône, `rect`).
- Dot de couleur : une icône RGBA par état (vert/orange/rouge) et `set_icon`. Une icône template macOS est teintée par le système, donc incompatible avec une couleur.
- Guide : https://v2.tauri.app/learn/system-tray/ ; API : https://docs.rs/tauri/latest/tauri/tray/struct.TrayIcon.html

### Popover
- `tauri-plugin-positioner` 2.4.0 (Rust 1.90+). Les positions `Tray*` exigent la feature `tray-icon` et `tauri_plugin_positioner::on_tray_event(...)` appelé depuis `on_tray_icon_event`. Tant qu'aucun événement n'a stocké le `rect`, déplacer vers une position `Tray*` échoue (code source). Le `rect` n'est lu que sur Click/Enter/Leave/Move ; Linux n'émet pas ces événements, donc `Tray*` y est inutilisable (déduction de la doc tray + du code, non testé).
- Docs : https://v2.tauri.app/plugin/positioner/ ; source : https://github.com/tauri-apps/plugins-workspace/tree/v2/plugins/positioner
- Le masquage du popover à la perte de focus (`WindowEvent::Focused(false)`) reste à coder.

### Fermer sans quitter
- Fermer la dernière fenêtre la détruit puis émet `RunEvent::ExitRequested { code: None, .. }` (vu dans `tauri-runtime-wry`) ; `api.prevent_exit()` l'annule (https://docs.rs/tauri/latest/tauri/struct.ExitRequestApi.html). `code: Some(_)` correspond à un `app.exit()` explicite : ne pas l'empêcher, pour garder un « Quitter » fonctionnel (doc `RunEvent`).
- Alternative plus propre : sur `WindowEvent::CloseRequested`, `prevent_close()` puis `window.hide()` ; la fenêtre n'est jamais détruite.
- macOS avec Dock visible : un clic Dock doit rouvrir la fenêtre (`RunEvent::Reopen`, non vérifié ici).

### Dock macOS
- `App::set_activation_policy(ActivationPolicy::Accessory)` (défaut Regular) et `set_dock_visibility(bool)`, aussi sur `AppHandle` (source `crates/tauri/src/app.rs`, branche dev). En Accessory : plus d'icône Dock ni de menu applicatif ; le focus des fenêtres est à tester.

### Autostart
- `tauri-plugin-autostart` 2.7.0 (Rust 1.90+). `MacosLauncher::LaunchAgent` (plist dans `~/Library/LaunchAgents`) ou `AppleScript` (login item via System Events). Permissions `autostart:allow-enable/disable/is-enabled`. Windows : valeur `Run` du registre.
- « Activé par défaut » : appeler `enable()` au premier lancement seulement, sinon on écrase le choix de l'utilisateur ; exposer un réglage.
- Passer un argument (ex. `--minimized`) pour démarrer sans fenêtre (`init` accepte une liste d'arguments).
- Docs : https://v2.tauri.app/plugin/autostart/ ; source : https://github.com/tauri-apps/plugins-workspace/tree/v2/plugins/autostart

### Notifications
- `tauri-plugin-notification` 2.7.0, basé sur `notify-rust` ; Linux via zbus. Windows : apps installées ; en dev l'identité PowerShell s'affiche (l'app id n'est posé que si l'exécutable n'est pas dans `target\debug|release`). macOS en dev : identité Terminal. Actions : surtout mobile ; sur desktop les clics ont des limites (Windows : seulement pendant que le toast est affiché).
- Docs : https://v2.tauri.app/plugin/notification/ ; source : https://github.com/tauri-apps/plugins-workspace/tree/v2/plugins/notification
- Utilisable depuis Rust (`app.notification().builder()...show()`), sans webview.

### Linux : tray
- Le crate `tray-icon` 0.26.1 (sous Tauri) utilise par défaut GTK3 + `libappindicator` ou `libayatana-appindicator` ; un backend `ksni` (StatusNotifierItem D-Bus) existe dans le crate, non vérifié s'il est exposé par `tauri`. Doc : https://github.com/tauri-apps/tray-icon
- GNOME Shell : l'extension « AppIndicator and KStatusNotifierItem Support » (https://extensions.gnome.org/extension/615/appindicator-support/) ajoute le support des icônes ; j'en déduis que GNOME standard n'affiche pas ces icônes (la page ne le dit pas explicitement, ni si Ubuntu l'embarque). Conséquence : prévoir un fallback sans tray et documenter la dépendance.
- `.deb` : dépendance `libayatana-appindicator3-1` (doc Tauri v1, à reconfirmer en v2). Wayland : positionnement libre de fenêtre non testé.

### Bridage en arrière-plan
- **macOS App Nap** (https://developer.apple.com/library/archive/documentation/Performance/Conceptual/power_efficiency_guidelines_osx/AppNap.html) : candidat quand l'app n'est pas au premier plan, n'a pas mis à jour de fenêtre visible, n'est pas audible et n'a pas d'assertion NSProcessInfo/IOKit. Effets : priorité réduite, timers étalés, I/O bridé. Une app de menu bar sans fenêtre visible est donc candidate (déduction). Parade : `NSProcessInfo.beginActivity` conservé pour la durée de vie (via `objc2`), pas de plugin officiel ; des retours tiers signalent une efficacité variable : à mesurer.
- **Windows** (https://learn.microsoft.com/en-us/windows/win32/procthread/quality-of-service) : fenêtre minimisée ou totalement masquée = QoS Low (cœurs efficients sur batterie) ; EcoQoS posé explicitement via `SetProcessInformation(PROCESS_POWER_THROTTLING_EXECUTION_SPEED)`. Une app peut s'étiqueter High QoS pour éviter le bridage. Le comportement exact d'un process Tauri en tray sans fenêtre n'est pas documenté : à mesurer.
- **Webview caché** : config `backgroundThrottling` (`disabled`/`suspend`/`throttle`), supportée seulement iOS 17+ et macOS 14+ ; Linux/Windows « Unsupported », contournement suggéré : WebLock en attente (source `tauri-utils/src/config.rs`, renvoie à https://github.com/tauri-apps/tauri/issues/5250). Inutile de s'y battre si les mesures sont en Rust.
- **Linux** : pas d'équivalent App Nap ; seule la veille système arrête les timers.

### Boucle de mesure en Rust
- Tâche `tauri::async_runtime::spawn` (tokio) lancée dans `setup`, avec `tokio::time::interval` ; elle émet l'état (`app.emit`), met à jour le tray et envoie les notifications directement. Le webview ne fait qu'afficher. Les timers JS d'une fenêtre cachée ne sont pas fiables.
- macOS : le tray vit sur le thread principal ; appeler ses méthodes depuis une tâche async peut nécessiter `run_on_main_thread` (à vérifier au prototype).

### Veille / réveil
- Pas d'événement Tauri officiel (`RunEvent::Resumed` = reprise de la boucle d'événements ; je n'ai pas trouvé de doc le liant à la veille système). Options :
  - macOS : `NSWorkspaceWillSleepNotification` / `DidWakeNotification` sur `NSWorkspace.sharedWorkspace.notificationCenter` (pas le centre par défaut).
  - Windows : `WM_POWERBROADCAST` ; `PBT_APMSUSPEND`, `PBT_APMRESUMEAUTOMATIC` (envoyé à chaque reprise), `PBT_APMRESUMESUSPEND` (en plus si reprise par action utilisateur). Ne distingue pas les états de basse conso. Source : https://learn.microsoft.com/en-us/windows/win32/power/wm-powerbroadcast
  - Linux : signal D-Bus `org.freedesktop.login1.Manager.PrepareForSleep(bool)` (true avant, false après), verrou inhibiteur `delay` possible. Source : https://www.freedesktop.org/software/systemd/man/latest/org.freedesktop.login1.html
- Crates tiers (jeunes, à évaluer) : `tauri-plugin-power-monitor` 0.1.0 (sept. 2026, 25 téléchargements, veille/réveil + batterie) ; `naptime` 0.1.0 (macOS seulement d'après sa description). Aucun n'est mature.
- Option portable à zéro dépendance : détecter la reprise par **saut d'horloge** (écart entre ticks >> intervalle), jeter la mesure en cours et marquer un trou. Les API natives ne servent que si l'on veut s'arrêter *avant* la veille.

## Plugins officiels : récapitulatif

| Besoin | Plugin | Version | Note |
|---|---|---|---|
| Tray | `tauri` feature `tray-icon` | 2.12.4 | cœur |
| Popover | `tauri-plugin-positioner` (feature `tray-icon`) | 2.4.0 | pas de `Tray*` sous Linux |
| Autostart | `tauri-plugin-autostart` | 2.7.0 | |
| Notifications | `tauri-plugin-notification` | 2.7.0 | |
| Instance unique | `tauri-plugin-single-instance` | 2.5.4 | non demandé, utile pour un résident |
| Mémoriser fenêtre | `tauri-plugin-window-state` | 2.5.0 | optionnel |

## Limites et pièges connus

1. Linux : pas de clic ni de `rect` sur le tray ; GNOME sans extension : pas de tray.
2. Windows : notifications en dev sous identité PowerShell ; tester sur build installé.
3. macOS : App Nap + fenêtre cachée = risque de ticks espacés ; prototyper `beginActivity`.
4. `backgroundThrottling` inutilisable hors macOS 14+/iOS : ne pas compter dessus.
5. Pas d'API veille/réveil officielle.
6. Positioner échoue tant qu'aucun événement tray n'a été reçu.
7. Notifications desktop : actions limitées, clics non garantis.

## Recommandation

- Stack : `tauri` (tray-icon) + positioner + autostart + notification + single-instance. Boucle de mesure en tâche tokio ; webview = affichage seul.
- Fermeture : `prevent_close` + `hide()`, plus `ExitRequested` (code `None`) filtré pour garder « Quitter ». macOS : `Accessory` ; Windows/Linux : `skip_taskbar`.
- Veille/réveil v1 : saut d'horloge (portable) ; natif par OS en amélioration ultérieure.
- macOS : prototyper `beginActivity` et mesurer la gigue de la boucle, fenêtre cachée, sur 1 h.
- Linux : « best effort » : documenter l'extension AppIndicator pour GNOME, prévoir un mode fenêtre sans tray et un fallback de position du popover.
- Non vérifié ici (à tester sur machine) : rendu du tray Linux, focus en policy Accessory, mécanisme d'autostart Linux, `RunEvent::Reopen`, comportement EcoQoS d'un process sans fenêtre.
