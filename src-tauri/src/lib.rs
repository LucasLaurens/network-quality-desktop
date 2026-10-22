//! Tauri shell: wires the adapters to the use cases and exposes typed commands to the front end.
#![forbid(unsafe_code)]

#[cfg(target_os = "macos")]
mod app_menu;
pub mod commands;
mod dashboard_display;
mod display;
mod engine_inbox;
mod history_export;
mod history_store;
mod network_display;
mod path_display;
mod settings_effects;
mod settings_form;
mod settings_store;
mod shell_mode;
mod shell_notifier;
mod shell_presenter;
mod technical_details;
mod text_catalog;
mod throughput_display;
mod throughput_lab;
mod traffic_store;
mod tray_icon;
mod tray_menu;
mod tray_painter;
mod windows;

use std::sync::Arc;
use std::time::Duration;

use application::{
    Clock, ColorSchemeReader, EngineMessage, EnginePorts, MeasurementEngine, NetworkChangeWatcher,
    PowerEventSource, list_default_targets, list_targets,
};
use display::ConnectionDisplay;
use domain::{ColorScheme, Verdict};
use engine_inbox::EngineInbox;
use infrastructure::{
    FallbackLatencyProbe, IcmpEchoSender, IcmpGatewayProbe, InterfaceChangeWatcher,
    NetdevContextReader, SystemClock, SystemColorSchemeReader, SystemPowerEventSource,
    SystemTrayHost, SystemWifiReader, TcpConnectionOpener, keep_measuring_in_background,
};
use shell_mode::{ShellMode, WindowModeTexts};
use shell_presenter::{
    ConnectionDisplayChanged, ConnectionStore, DisplayStore, ShellPresenter, ShellTrayPainter,
    create_icon_image, describe_verdict,
};
use tauri::tray::{MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent};
use tauri::{Manager, RunEvent};
use tauri_specta::{Builder, collect_commands, collect_events};
use text_catalog::TextCatalog;
use tokio::sync::mpsc;
use tokio::time::MissedTickBehavior;
use tray_menu::{MenuAction, TrayMenu, apply_menu_action};
use windows::{
    DashboardRequested, HAS_POPOVER, PopoverDismissal, apply_window_event, create_popover,
    show_dashboard, toggle_popover,
};

const TRAY_ID: &str = "connection-state";
const TICK_EVERY: Duration = Duration::from_secs(1);
/// A few pending ticks at most: if the engine falls behind, older ticks are not worth replaying.
const ENGINE_INBOX_CAPACITY: usize = 4;
const COLOR_SCHEME_EVERY: Duration = Duration::from_secs(1);

/// Single list of the commands and events shared with the front end, used by the runtime and the
/// bindings export.
#[must_use]
pub fn create_specta_builder() -> Builder<tauri::Wry> {
    Builder::<tauri::Wry>::new()
        .commands(collect_commands![
            commands::read_app_info,
            commands::read_interface_texts,
            commands::read_connection_display,
            commands::open_dashboard,
            commands::open_settings,
            commands::close_popover,
            commands::read_window_mode_texts,
            commands::open_tray_extension_page,
            commands::quit_app,
            commands::reveal_network_name,
            commands::read_history_notice,
            commands::read_dashboard_summary,
            commands::read_evolution,
            commands::read_settings,
            commands::save_settings,
            commands::read_throughput_panel,
            commands::run_throughput_test,
            commands::choose_export_folder,
            commands::export_history,
            commands::reveal_export_folder,
            commands::read_export_availability,
            commands::erase_history,
            commands::read_technical_details,
            commands::read_line_details
        ])
        .events(collect_events![
            ConnectionDisplayChanged,
            DashboardRequested
        ])
}

/// # Panics
///
/// Panics when the Tauri runtime cannot start, which leaves nothing to recover.
pub fn run() {
    let builder = create_specta_builder();
    let app = tauri::Builder::default()
        // First: a second launch shows the running app's dashboard, and never opens the history
        // file a second time.
        .plugin(tauri_plugin_single_instance::init(|app, _, _| {
            show_dashboard(app);
        }))
        .plugin(tauri_plugin_autostart::Builder::new().build())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_positioner::init());
    #[cfg(target_os = "macos")]
    let app = app.menu(app_menu::build_app_menu);
    app.invoke_handler(builder.invoke_handler())
        .setup(move |app| {
            builder.mount_events(app);
            // A menu-bar app: no Dock icon, whose click would find the dashboard hidden.
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);
            if HAS_POPOVER {
                create_popover(app.handle())?;
            }
            start_measurement(app.handle())?;
            Ok(())
        })
        .on_window_event(apply_window_event)
        .build(tauri::generate_context!())
        .expect("the Tauri runtime failed to start")
        .run(|app, event| {
            if matches!(event, RunEvent::Exit)
                && let Some(inbox) = app.try_state::<EngineInbox>()
            {
                inbox.flush();
            }
        });
}

/// Picks the tray or window mode, builds the dot and its menu, and starts the measurement actor
/// with its 1 Hz clock, all on the Rust side.
fn start_measurement(app: &tauri::AppHandle) -> tauri::Result<()> {
    let catalog = TextCatalog::load();
    // The dashboard commands word their figures with their own copy.
    app.manage(TextCatalog::load());
    let initial = ConnectionDisplay::create_initial(&catalog);
    // Before any window shows: the look is set, and the settings choose the servers.
    let stores = history_store::open_stores(app, &catalog, SystemClock::start().read_utc_seconds());
    let settings = stores.settings.read();
    settings_effects::apply_settings(app, &settings);
    app.manage(stores.notice);
    app.manage(stores.dashboard);
    app.manage(stores.settings);
    app.manage(stores.location);
    app.manage(history_export::ExportFolder::default());
    let mode = ShellMode::detect(&SystemTrayHost);
    app.manage(mode);
    app.manage(WindowModeTexts::compose(mode, &catalog));
    app.manage(PopoverDismissal::default());
    let store = DisplayStore::new(initial.clone());
    app.manage(store.clone());
    let scheme = SystemColorSchemeReader.read_color_scheme();
    let menu = Arc::new(TrayMenu::build(app, &initial, &settings, &catalog)?);
    app.manage(Arc::clone(&menu));
    // The icon is built in window mode too: a tray that appears later still gets its dot.
    let tray = build_tray(app, &menu, scheme, &catalog)?;
    let painter = Arc::new(ShellTrayPainter::new(
        tray,
        SystemColorSchemeReader,
        Verdict::Measuring,
        scheme,
        describe_verdict(Verdict::Measuring, &catalog),
    ));
    follow_color_scheme(Arc::clone(&painter));
    if mode == ShellMode::Window {
        show_dashboard(app);
    }
    // The default route is the one to the Internet, whichever server the user measures against.
    let route_reference = list_default_targets()
        .first()
        .expect("the default targets are listed")
        .get_address();
    let connection_store = ConnectionStore::default();
    app.manage(connection_store.clone());
    app.manage(traffic_store::TrafficStore::start(
        NetdevContextReader::new(route_reference),
    ));
    app.manage(throughput_lab::ThroughputLab::create());
    let (sender, inbox) = mpsc::channel(ENGINE_INBOX_CAPACITY);
    let (urgent_sender, urgent) = mpsc::channel(engine_inbox::URGENT_CAPACITY);
    let engine_inbox = EngineInbox {
        regular: sender.clone(),
        urgent: urgent_sender,
    };
    app.manage(engine_inbox.clone());
    let mut engine = MeasurementEngine::new(
        EnginePorts {
            probe: create_latency_probe(),
            clock: SystemClock::start(),
            presenter: ShellPresenter::new(
                app.clone(),
                menu,
                catalog,
                store,
                connection_store,
                painter,
            ),
            context_reader: NetdevContextReader::new(route_reference),
            gateway_probe: IcmpGatewayProbe::new(IcmpEchoSender),
            wifi_reader: SystemWifiReader,
            history_recorder: stores.recorder,
            notifier: shell_notifier::ShellNotifier::new(app.clone(), TextCatalog::load()),
        },
        list_targets(settings.servers),
    );
    engine.apply_settings(settings);
    let change_sender = sender.clone();
    InterfaceChangeWatcher.watch(move || {
        // Dropped when the inbox is full: the periodic reading still catches the change within 10 s.
        let _ = change_sender.try_send(EngineMessage::NetworkChanged);
    });
    SystemPowerEventSource.watch(move |event| engine_inbox.relay_power_event(event));
    keep_measuring_in_background();
    tauri::async_runtime::spawn(engine.run(inbox, urgent));
    tauri::async_runtime::spawn(send_ticks(sender));
    Ok(())
}

/// ICMP echo, then a TCP connection where ICMP is refused: the probe the engine measures with,
/// and the one a new own server must answer.
fn create_latency_probe() -> FallbackLatencyProbe<IcmpEchoSender, TcpConnectionOpener> {
    FallbackLatencyProbe::new(IcmpEchoSender, TcpConnectionOpener)
}

async fn send_ticks(sender: mpsc::Sender<EngineMessage>) {
    let mut interval = tokio::time::interval(TICK_EVERY);
    interval.set_missed_tick_behavior(MissedTickBehavior::Delay);
    loop {
        interval.tick().await;
        if sender.send(EngineMessage::Tick).await.is_err() {
            break;
        }
    }
}

/// The menu-bar icon showing « Mesure en cours », with its menu and its clicks. O(1).
fn build_tray(
    app: &tauri::AppHandle,
    menu: &TrayMenu,
    scheme: ColorScheme,
    catalog: &TextCatalog,
) -> tauri::Result<TrayIcon> {
    TrayIconBuilder::with_id(TRAY_ID)
        .icon(create_icon_image(Verdict::Measuring, scheme))
        .icon_as_template(false)
        .tooltip(describe_verdict(Verdict::Measuring, catalog))
        .menu(menu.read_menu())
        .show_menu_on_left_click(!HAS_POPOVER)
        .on_menu_event(|app, event| {
            if let Some(action) = MenuAction::from_id(event.id().as_ref()) {
                apply_menu_action(app, action);
            }
        })
        .on_tray_icon_event(|tray, event| {
            tauri_plugin_positioner::on_tray_event(tray.app_handle(), &event);
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
                && HAS_POPOVER
            {
                toggle_popover(tray.app_handle());
            }
        })
        .build(app)
}

/// Redraws the dot when the system's scheme changes, apart from the displays: none comes during a
/// throughput test. One reading a second, on a thread of its own.
fn follow_color_scheme(painter: Arc<ShellTrayPainter>) {
    let spawned = std::thread::Builder::new()
        .name("color-scheme".to_owned())
        .spawn(move || {
            loop {
                std::thread::sleep(COLOR_SCHEME_EVERY);
                painter.follow_scheme();
            }
        });
    if let Err(error) = spawned {
        eprintln!("Following the system's colour scheme failed: {error}");
    }
}

#[cfg(test)]
mod tests {
    use specta_typescript::Typescript;

    // Rewrites the front-end bindings; `pnpm bindings:check` then fails if the committed file differs.
    // Rewrites the texts the front-end fakes hand out, so they are the real catalogue's; `pnpm
    // bindings:check` then fails if the committed file differs.
    #[test]
    fn exports_the_interface_texts_for_the_fakes() {
        // Arrange
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../tests/fakes/interface-texts.json"
        );
        let texts = crate::text_catalog::TextCatalog::load().list_interface_texts();

        // Act
        let json = serde_json::to_string_pretty(&texts).expect("texts as JSON");
        let result = std::fs::write(path, json + "\n");

        // Assert
        assert!(result.is_ok(), "texts export failed: {result:?}");
    }

    #[test]
    fn exports_typescript_bindings() {
        // Arrange
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../src/bindings.ts");

        // Act
        let result = super::create_specta_builder().export(Typescript::default(), path);

        // Assert
        assert!(result.is_ok(), "bindings export failed: {result:?}");
    }
}
