//! Tauri shell: wires the adapters to the use cases and exposes typed commands to the front end.
#![forbid(unsafe_code)]

pub mod commands;
mod tray_icon;
mod tray_presenter;

use std::time::Duration;

use application::{EngineMessage, MeasurementEngine, list_default_targets};
use domain::Verdict;
use infrastructure::{IcmpLatencyProbe, SystemClock};
use tauri::menu::{Menu, MenuItem};
use tauri::tray::TrayIconBuilder;
use tauri_specta::{Builder, collect_commands};
use tokio::sync::mpsc;
use tokio::time::MissedTickBehavior;
use tray_presenter::{TrayPresenter, create_icon_image, describe_verdict};

const TRAY_ID: &str = "connection-state";
const QUIT_ID: &str = "quit";
const TICK_EVERY: Duration = Duration::from_secs(1);
/// A few pending ticks at most: if the engine falls behind, older ticks are not worth replaying.
const ENGINE_INBOX_CAPACITY: usize = 4;

/// Single list of the commands exposed to the front end, shared by the runtime and the bindings export.
#[must_use]
pub fn create_specta_builder() -> Builder<tauri::Wry> {
    Builder::<tauri::Wry>::new().commands(collect_commands![commands::read_app_info])
}

/// # Panics
///
/// Panics when the Tauri runtime cannot start, which leaves nothing to recover.
pub fn run() {
    let builder = create_specta_builder();
    tauri::Builder::default()
        .invoke_handler(builder.invoke_handler())
        .setup(|app| {
            start_measurement(app.handle())?;
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("the Tauri runtime failed to start");
}

/// Creates the menu-bar dot and starts the measurement actor with its 1 Hz clock, all on the Rust side.
fn start_measurement(app: &tauri::AppHandle) -> tauri::Result<()> {
    // Linux tray backends may hide an icon that has no menu; the full native menu comes later.
    let quit = MenuItem::with_id(app, QUIT_ID, "Quitter", true, None::<&str>)?;
    let tray = TrayIconBuilder::with_id(TRAY_ID)
        .icon(create_icon_image(Verdict::Measuring))
        .icon_as_template(false)
        .tooltip(describe_verdict(Verdict::Measuring))
        .menu(&Menu::with_items(app, &[&quit])?)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| {
            if event.id() == QUIT_ID {
                app.exit(0);
            }
        })
        .build(app)?;
    let (sender, inbox) = mpsc::channel(ENGINE_INBOX_CAPACITY);
    let engine = MeasurementEngine::new(
        IcmpLatencyProbe,
        SystemClock::start(),
        TrayPresenter::new(tray),
        list_default_targets(),
    );
    tauri::async_runtime::spawn(engine.run(inbox));
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
