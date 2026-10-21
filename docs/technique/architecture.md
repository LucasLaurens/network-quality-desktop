# Architecture

## Découpage hexagonal

Le cœur est en Rust, organisé en workspace Cargo de quatre crates. Le sens des dépendances est garanti par le compilateur : une crate ne peut pas importer une crate de la couche au-dessus.

| Crate                       | Rôle                                                                                              | Dépend de                                             |
| --------------------------- | ------------------------------------------------------------------------------------------------- | ----------------------------------------------------- |
| `domain`                    | Règles pures de la mesure (verdict, seuils, hystérésis). Aucune entrée-sortie. `unsafe` interdit. | rien                                                  |
| `application`               | Cas d'usage et ports (traits) que l'infrastructure implémente. `unsafe` interdit.                 | `domain`                                              |
| `infrastructure`            | Adaptateurs par OS, stockage, clients HTTP. Seul endroit où l'`unsafe` est permis (FFI native).   | `domain`, `application`                               |
| `app` (dossier `src-tauri`) | Shell Tauri : assemble les adaptateurs et expose des commandes typées au front.                   | les trois autres, au fur et à mesure qu'elles servent |

État actuel :

- `domain` : niveaux et seuils, diagnostic (`Diagnosis`, `Metric`), fenêtre de latence (p90, gigue), fenêtre de perte, détection Hors ligne, issue d'une sonde, médiane, hystérésis, verdict.
- `application` : moteur de mesure (acteur sur un canal tokio, seule dépendance d'exécution de cette crate), ports `LatencyProbe`, `Clock`, `VerdictPresenter`. Les faux en mémoire ne sont compilés que pour les tests (feature `test-support`).
- `infrastructure` : sonde à repli (écho ICMP, connexion TCP 443) et horloge système.
- `app` : icône de barre système (avec un menu minimal « Quitter »), horloge à 1 Hz, `ShellPresenter` qui met à jour l'icône et publie l'objet d'affichage, fenêtre popover ancrée à l'icône (plugin positioner), commandes `read_app_info`, `read_connection_display`, `open_dashboard`, `close_popover`, événement `ConnectionDisplayChanged`.

## Front

Vue 3 + TypeScript. Le front ne contient aucune règle métier : il affiche ce que Rust lui envoie.

- Les composants ne parlent jamais directement à Tauri. Ils passent par le port `NetworkQualityClient`, injecté par `provide`/`inject`.
- En production, `TauriNetworkQualityClient` appelle les commandes Rust.
- En test, `InMemoryNetworkQualityClient` (dans `tests/fakes`) renvoie des données préparées : aucun mock de module.

## Types partagés

Les types et les signatures des commandes sont générés depuis Rust par `tauri-specta` dans `src/bindings.ts`. Ce fichier ne se modifie jamais à la main : le test Rust `exports_up_to_date_typescript_bindings` le régénère, et la CI échoue s'il diffère de la version commitée (`pnpm bindings:check`).

## Objet d'affichage et textes

- Le moteur produit un `VerdictView` (verdict, `Diagnosis`, trois `MetricReading` avec leurs bornes). `ConnectionDisplay::from_view` (crate `app`) le transforme en objet prêt à afficher : libellé d'état, constat, action, détail chiffré, et pour chaque mesure son nom, sa méthode, sa valeur formatée, sa jauge (positions en fraction) et son indication « Proche du seuil ».
- Tous ces textes viennent du catalogue Fluent `src-tauri/locales/fr.ftl`, chargé par `TextCatalog`. Les nombres sont formatés à la française (virgule décimale, espace insécable avant l'unité).
- Le front reçoit cet objet par la commande `read_connection_display` (à l'ouverture) et l'événement `ConnectionDisplayChanged` (à chaque changement), et l'affiche tel quel.
- La fenêtre popover charge la même page avec `?view=popover`. Elle s'ouvre au clic gauche sur l'icône, se place sous l'icône (macOS) ou au-dessus (Windows), et se masque quand elle perd le focus ou sur Échap. Un clic sur l'icône qui arrive moins de 300 ms après ce masquage est le même geste : il ne rouvre pas le popover. Fermer le tableau de bord le masque seulement, pour que le popover puisse le rouvrir.
