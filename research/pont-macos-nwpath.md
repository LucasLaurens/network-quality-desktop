# Pont Rust vers NWPath sur macOS (#17)

## Réponse courte

Recommandation : **FFI direct vers l'API C `nw_path_monitor_*`** (option 1), avec les crates `block2` 0.6.2 et `dispatch2` 0.3.1, dans un seul module `unsafe` de la crate `infrastructure`, derrière les ports `ConnectionContextReader` et `NetworkChangeWatcher`. Aucun toolchain Swift n'est requis : seul le link `-framework Network` est ajouté. Le spike compile et tourne sur ce Mac (voir plus bas). `SCDynamicStore` reste utile comme repli ou complément pour détecter les changements de configuration, mais **ne sait pas fournir `isExpensive` / `isConstrained`**.

## Contexte vérifié

- `objc2-network` est bien une coquille vide : version `0.0.0`, « Reserved crate name » (`cargo info objc2-network`). Le dépôt `madsmtm/objc2` contient un dossier `framework-crates/objc2-network`, sans bindings publiés. Network.framework expose une API C avec des blocks, pas des classes Objective-C : la génération `objc2` ne la couvre pas.
- Les headers du SDK local (`Network.framework/Headers/path_monitor.h`, `path.h`) confirment : `nw_path_monitor_create` (macOS 10.14), `nw_path_monitor_set_queue`, `nw_path_monitor_set_update_handler` (block `void (^)(nw_path_t)`), `nw_path_monitor_start`, `nw_path_monitor_cancel`, `nw_path_get_status`, `nw_path_is_expensive` (10.14), `nw_path_is_constrained` (10.15), et `nw_path_is_ultra_constrained` (macOS 26 seulement ; disponibilité à tester avant usage, **[non vérifié à l'exécution]**).
- Versions crates.io au 2026-10-20 : `block2` 0.6.2, `dispatch2` 0.3.1, `swift-rs` 1.0.9, `swift-bridge` 0.1.59, `system-configuration` 0.8.0, `objc2-system-configuration` 0.3.2.
- Activité : `swift-rs` poussé le 2026-10-10, `system-configuration-rs` (Mullvad) le 2026-08-11 : tous deux maintenus.

## Comparatif

| Critère | 1. FFI C direct (`block2` + `dispatch2`) | 2. Pont Swift (`swift-rs` / `swift-bridge`) | 3. SCDynamicStore + SCNetworkReachability |
|---|---|---|---|
| `isExpensive` / `isConstrained` | Oui (spike) | Oui (`NWPathMonitor`) | **Non** (pas d'équivalent macOS ; `IsWWAN` est iOS) **[non vérifié en détail]** |
| Changements de chemin | Oui, push via handler | Oui | Oui (notifications par clé de store), plus bas niveau |
| Toolchain build | Rust + SDK macOS (link framework) | Swift + Xcode CLT, package SPM `.static`, `swift-rs` en dépendance et build-dependency | Rust + SDK |
| Coût CI | Runner macOS standard | Idem + temps de build Swift, link du runtime Swift à gérer avec Tauri | Runner macOS standard |
| Autres OS | Module sous `#[cfg(target_os = "macos")]`, sans effet ailleurs | Idem, `build.rs` Swift à conditionner | Idem |
| Sûreté des callbacks | `unsafe` à encapsuler soi-même (1 module) | Même frontière `unsafe` (Swift -> C -> Rust) en plus | Callback déjà enveloppé par la crate (API safe) |
| Surface `unsafe` | ~10 déclarations `extern "C"`, 1 block | Plus petite côté Rust, mais code Swift (2e langage) à maintenir | Faible |
| Testabilité | Port + `InMemory...` ; seul l'adaptateur macOS est non testable hors macOS | Idem | Idem |
| Dépendance tierce | Aucune (API C publique d'Apple, stable depuis 10.14) | Outillage `swift-rs`/Swift | Crate Mullvad |

## Spike (jeté, non commité)

Projet minimal (`block2` 0.6.2, `dispatch2` 0.3.1, `build.rs` avec `cargo:rustc-link-lib=framework=Network`), Swift 6.4 / Xcode local, macOS arm64, cargo 1.98.1 : compile en 3,5 s, s'exécute, et affiche dès l'enregistrement du handler :

```
status=1 expensive=false constrained=false
```

`status=1` correspond à `nw_path_status_satisfied`. Le handler est appelé immédiatement avec le chemin courant, puis à chaque changement (seul le premier appel a été observé : pas de bascule réseau provoquée pendant le test). Non observé : une bascule réelle sur hotspot iPhone (donc `expensive=true`) **[non vérifié]** ; le risque porte sur le cycle de vie, pas sur la lecture du booléen.

## Points de vigilance de l'option 1

1. **Cycle de vie** : garder le `RcBlock` vivant tant que le monitor tourne ; après `nw_path_monitor_cancel`, libérer avec `nw_release`.
2. **Le `nw_path_t` n'est fiable que pendant l'appel du handler** (sinon `nw_retain`) : copier aussitôt les champs dans une struct Rust (`PathSnapshot { status, is_expensive, is_constrained }`) et ne jamais laisser sortir le pointeur.
3. **Thread** : le handler tourne sur la file GCD fournie (`dispatch2::DispatchQueue` série), ni sur un thread Tokio ni sur le thread principal. Le closure doit être `Send + 'static` ; transmettre par canal (`tokio::sync::watch` ou `mpsc`) sans verrou long. Ne jamais paniquer dans le handler (unwind à travers un frame C = UB) : `catch_unwind`.
4. **Types** : `nw_path_monitor_t` et la file sont des objets OS ; déclarés en `*mut c_void` dans le spike, à envelopper dans un newtype avec `Drop` en production.
5. **Sandbox** : `com.apple.security.network.client` probablement nécessaire si l'app est sandboxée **[non vérifié]**.
6. **Version minimale** : `is_constrained` exige macOS 10.15 ; vérifier le deployment target de Tauri 2 **[non vérifié]**.

## Mapping vers les ports

- `NetworkChangeWatcher` : l'adaptateur macOS démarre le monitor, pousse chaque `PathSnapshot` dans un `watch::Sender` et expose le flux de changements. Un `InMemoryNetworkChangeWatcher` piloté par le test couvre le reste, sans mock.
- `ConnectionContextReader` : lit le dernier `PathSnapshot` (cache alimenté par le watcher) plutôt que créer un monitor à chaque lecture. Linux/Windows : autres adaptateurs, hors périmètre.
- Échec du monitor : l'adaptateur renvoie un contexte « inconnu » explicite (type du domaine), pas `null`.
- Un test d'intégration macOS-only (`#[cfg(target_os = "macos")]`) vérifie qu'un premier snapshot arrive en quelques secondes.

## Pourquoi pas les deux autres

- **Swift** : un second langage et une chaîne de build (SPM `.static`, runtime Swift) pour appeler ~6 fonctions C déjà exposées. À reconsidérer seulement si d'autres API Swift-only deviennent nécessaires.
- **SCDynamicStore / Reachability** : bon déclencheur de re-mesure (interface ou route modifiée), insuffisant pour la métrique coûteux/économie de données. Repli possible pour `NetworkChangeWatcher` uniquement, pas pour `ConnectionContextReader`.

## Sources

- Headers locaux : `$(xcrun --show-sdk-path)/System/Library/Frameworks/Network.framework/Headers/{path_monitor,path}.h`
- https://developer.apple.com/documentation/network/nw_path_monitor_t
- https://developer.apple.com/documentation/network/nw_path_is_expensive(_:)
- https://crates.io/crates/objc2-network (version 0.0.0, réservée)
- https://crates.io/crates/block2 , https://crates.io/crates/dispatch2
- https://github.com/Brendonovich/swift-rs (prérequis SPM / build)
- https://github.com/mullvad/system-configuration-rs
