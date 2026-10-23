# Changelog

## [0.9.2](https://github.com/LucasLaurens/network-quality-desktop/compare/v0.9.1...v0.9.2) (2026-10-23)


### Bug Fixes

* **ci:** pass the first real CI run on every system ([#88](https://github.com/LucasLaurens/network-quality-desktop/issues/88)) ([feea82d](https://github.com/LucasLaurens/network-quality-desktop/commit/feea82d12672cd5e478e437d9f640c590899398d))

## 0.9.1 (2026-10-23)

### Bug Fixes

- **deps:** upgrade tokio past RUSTSEC-2026-0384 ([#82](https://github.com/LucasLaurens/network-quality-desktop/pull/82))
- **security:** fetch only the UPnP description the gateway announces, and build releases without a write token ([#83](https://github.com/LucasLaurens/network-quality-desktop/pull/83))
- **history:** tell the user when the dashboard cannot open the history ([#85](https://github.com/LucasLaurens/network-quality-desktop/pull/85))

### Code Refactoring

- apply the clean code audit across the codebase ([#85](https://github.com/LucasLaurens/network-quality-desktop/pull/85))

### Documentation

- show the app on the README and explain how to install and use it ([#81](https://github.com/LucasLaurens/network-quality-desktop/pull/81))

## 0.9.0 (2026-10-22)

### Features

- bootstrap Tauri, Vue and Rust workspace skeleton ([#41](https://github.com/LucasLaurens/network-quality-desktop/pull/41))
- show a live menu-bar dot from measured latency ([#42](https://github.com/LucasLaurens/network-quality-desktop/pull/42))
- grade jitter and loss, detect offline and warm up ([#43](https://github.com/LucasLaurens/network-quality-desktop/pull/43))
- open a popover with the state, its cause and the three metrics ([#44](https://github.com/LucasLaurens/network-quality-desktop/pull/44))
- add the native tray menu and a window mode without a tray ([#45](https://github.com/LucasLaurens/network-quality-desktop/pull/45))
- recognise the network and how the computer reaches it ([#46](https://github.com/LucasLaurens/network-quality-desktop/pull/46))
- know when data counts and measure again on a new network ([#47](https://github.com/LucasLaurens/network-quality-desktop/pull/47))
- say which segment of the path is at fault ([#48](https://github.com/LucasLaurens/network-quality-desktop/pull/48))
- read the Wi-Fi link and name a weak signal as the cause ([#49](https://github.com/LucasLaurens/network-quality-desktop/pull/49))
- keep a durable history of the measurements ([#50](https://github.com/LucasLaurens/network-quality-desktop/pull/50))
- show the day, the evolution and the networks in the dashboard ([#51](https://github.com/LucasLaurens/network-quality-desktop/pull/51))
- keep measuring in the background, through sleep and wake ([#52](https://github.com/LucasLaurens/network-quality-desktop/pull/52))
- add the settings page ([#53](https://github.com/LucasLaurens/network-quality-desktop/pull/53))
- notify when the connection gets durably worse ([#54](https://github.com/LucasLaurens/network-quality-desktop/pull/54))
- run throughput tests on a click ([#55](https://github.com/LucasLaurens/network-quality-desktop/pull/55))
- débit local, UPnP et données ([#56](https://github.com/LucasLaurens/network-quality-desktop/pull/56))
- garde-fous qualité encodés ([#57](https://github.com/LucasLaurens/network-quality-desktop/pull/57))
- releases automatisées ([#58](https://github.com/LucasLaurens/network-quality-desktop/pull/58))
- tests de débit et Réglages depuis la pastille ([#60](https://github.com/LucasLaurens/network-quality-desktop/pull/60))
- export dans un dossier choisi ([#68](https://github.com/LucasLaurens/network-quality-desktop/pull/68))
- pastille adaptée au thème sombre ([#69](https://github.com/LucasLaurens/network-quality-desktop/pull/69))
- chiffres honnêtes, durée mesurée et valeur inconnue ([#70](https://github.com/LucasLaurens/network-quality-desktop/pull/70))
- explain how to allow the ping, inside the app ([#71](https://github.com/LucasLaurens/network-quality-desktop/pull/71))
- try ICMP again after a network change, a wake or new servers ([#72](https://github.com/LucasLaurens/network-quality-desktop/pull/72))
- take every word of the windows from the Fluent catalogue ([#73](https://github.com/LucasLaurens/network-quality-desktop/pull/73))
- quit the whole app with Cmd+Q on macOS ([#79](https://github.com/LucasLaurens/network-quality-desktop/pull/79))

### Bug Fixes

- centre the window column and align the settings with it ([#77](https://github.com/LucasLaurens/network-quality-desktop/pull/77))

### Tests

- hold the remaining test decisions of the spec ([#74](https://github.com/LucasLaurens/network-quality-desktop/pull/74))
- cover the domain with its own unit tests ([#75](https://github.com/LucasLaurens/network-quality-desktop/pull/75))
