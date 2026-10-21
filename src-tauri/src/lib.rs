//! Tauri shell: wires the adapters to the use cases and exposes typed commands to the front end.
#![forbid(unsafe_code)]

#[cfg(target_os = "macos")]
mod app_menu;
pub mod commands;
mod dashboard_display;
mod display;
mod engine_inbox;
mod history_store;
mod network_display;
mod path_display;
mod shell_mode;
mod shell_presenter;
mod text_catalog;
mod tray_icon;
mod tray_menu;
mod windows;

use std::time::Duration;

use application::{
    Clock, EngineMessage, EnginePorts, MeasurementEngine, NetworkChangeWatcher, PowerEventSource,
    list_default_targets,
};
use display::ConnectionDisplay;
use domain::Verdict;
use engine_inbox::EngineInbox;
use infrastructure::{
    FallbackLatencyProbe, IcmpEchoSender, IcmpGatewayProbe, InterfaceChangeWatcher,
    NetdevContextReader, SystemClock, SystemPowerEventSource, SystemTrayHost, SystemWifiReader,
    TcpConnectionOpener, keep_measuring_in_background,
};
use shell_mode::{ShellMode, WindowModeTexts};
use shell_presenter::{
    ConnectionDisplayChanged, DisplayStore, ShellPresenter, create_icon_image, describe_verdict,
};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{Manager, RunEvent};
use tauri_specta::{Builder, collect_commands, collect_events};
use text_catalog::TextCatalog;
use tokio::sync::mpsc;
use tokio::time::MissedTickBehavior;
use tray_menu::{MenuAction, TrayMenu, compose_menu_entries};
use windows::{
    HAS_POPOVER, PopoverDismissal, apply_window_event, create_popover, show_dashboard,
    toggle_popover,
};

const TRAY_ID: &str = "connection-state";
const TICK_EVERY: Duration = Duration::from_secs(1);
/// A few pending ticks at most: if the engine falls behind, older ticks are not worth replaying.
const ENGINE_INBOX_CAPACITY: usize = 4;

/// Single list of the commands and events shared with the front end, used by the runtime and the
/// bindings export.
#[must_use]
pub fn create_specta_builder() -> Builder<tauri::Wry> {
    Builder::<tauri::Wry>::new()
        .commands(collect_commands![
            commands::read_app_info,
            commands::read_connection_display,
            commands::open_dashboard,
            commands::close_popover,
            commands::read_window_mode_texts,
            commands::open_tray_extension_page,
            commands::quit_app,
            commands::reveal_network_name,
            commands::read_history_notice,
            commands::read_dashboard_summary,
            commands::read_evolution
        ])
        .events(collect_events![ConnectionDisplayChanged])
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
    let mode = ShellMode::detect(&SystemTrayHost);
    app.manage(mode);
    app.manage(WindowModeTexts::compose(mode, &catalog));
    app.manage(PopoverDismissal::default());
    let store = DisplayStore::new(initial.clone());
    app.manage(store.clone());
    let menu = TrayMenu::build(app, &compose_menu_entries(&initial, &catalog))?;
    // The icon is built in window mode too: a tray that appears later still gets its dot.
    let tray = TrayIconBuilder::with_id(TRAY_ID)
        .icon(create_icon_image(Verdict::Measuring))
        .icon_as_template(false)
        .tooltip(describe_verdict(Verdict::Measuring, &catalog))
        .menu(menu.read_menu())
        .show_menu_on_left_click(!HAS_POPOVER)
        .on_menu_event(
            |app, event| match MenuAction::from_id(event.id().as_ref()) {
                Some(MenuAction::OpenDashboard) => show_dashboard(app),
                Some(MenuAction::Quit) => app.exit(0),
                None => {}
            },
        )
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
        .build(app)?;
    if mode == ShellMode::Window {
        show_dashboard(app);
    }
    let targets = list_default_targets();
    let route_reference = targets
        .first()
        .expect("the default targets are listed")
        .address();
    let (history_recorder, history_notice, dashboard_history) =
        history_store::open_history(app, &catalog, SystemClock::start().read_utc_seconds());
    app.manage(history_notice);
    app.manage(dashboard_history);
    let (sender, inbox) = mpsc::channel(ENGINE_INBOX_CAPACITY);
    let (urgent_sender, urgent) = mpsc::channel(engine_inbox::URGENT_CAPACITY);
    let engine_inbox = EngineInbox {
        regular: sender.clone(),
        urgent: urgent_sender,
    };
    app.manage(engine_inbox.clone());
    let engine = MeasurementEngine::new(
        EnginePorts {
            probe: FallbackLatencyProbe::new(IcmpEchoSender, TcpConnectionOpener),
            clock: SystemClock::start(),
            presenter: ShellPresenter::new(app.clone(), tray, menu, catalog, store),
            context_reader: NetdevContextReader::new(route_reference),
            gateway_probe: IcmpGatewayProbe::new(IcmpEchoSender),
            wifi_reader: SystemWifiReader,
            history_recorder,
        },
        targets,
    );
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

#[cfg(test)]
mod tests {
    use specta_typescript::Typescript;

    // Rewrites the front-end bindings; `pnpm bindings:check` then fails if the committed file differs.
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
