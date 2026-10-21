//! Typed commands exposed to the front end. Tauri cannot export `pub` commands from the crate root.

use serde::Serialize;
use specta::Type;
use tauri::{AppHandle, Manager, State};

use application::{Clock, LatencyProbe, list_network_summaries, read_day_overview};
use domain::{MeasurementServers, ProbeOutcome, Target};
use infrastructure::{
    FallbackLatencyProbe, IcmpEchoSender, LocationRequest, SystemClock, TcpConnectionOpener,
    request_location_permission,
};

use crate::dashboard_display::{
    DashboardSummaryDisplay, EvolutionDisplay, PeriodChoice, compute_day_start,
};
use crate::display::ConnectionDisplay;
use crate::engine_inbox::EngineInbox;
use crate::history_store::{DashboardHistory, HistoryNotice};
use crate::settings_effects::apply_settings;
use crate::settings_form::{SettingsForm, confirm_saved};
use crate::settings_store::SettingsStore;
use crate::shell_mode::{APPINDICATOR_EXTENSION_URL, WindowModeTexts};
use crate::shell_presenter::DisplayStore;
use crate::text_catalog::TextCatalog;
use crate::windows::{POPOVER_WINDOW, report_failure, show_dashboard};

/// Identity of the running build, shown in the window footer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Type)]
pub struct AppInfo {
    pub name: String,
    pub version: String,
}

#[tauri::command]
#[specta::specta]
#[must_use]
pub fn read_app_info() -> AppInfo {
    AppInfo {
        name: env!("CARGO_PKG_NAME").to_owned(),
        version: env!("CARGO_PKG_VERSION").to_owned(),
    }
}

/// The latest display, for a window that opens between two events.
#[tauri::command]
#[specta::specta]
#[must_use]
#[expect(
    clippy::needless_pass_by_value,
    reason = "Tauri injects command arguments by value"
)]
pub fn read_connection_display(store: State<'_, DisplayStore>) -> ConnectionDisplay {
    store.read()
}

/// Brings the dashboard window forward and closes the popover.
#[tauri::command]
#[specta::specta]
#[expect(
    clippy::needless_pass_by_value,
    reason = "Tauri injects command arguments by value"
)]
pub fn open_dashboard(app: AppHandle) {
    close_popover(app.clone());
    show_dashboard(&app);
}

/// Closes the popover from the keyboard (Escape).
#[tauri::command]
#[specta::specta]
#[expect(
    clippy::needless_pass_by_value,
    reason = "Tauri injects command arguments by value"
)]
pub fn close_popover(app: AppHandle) {
    if let Some(popover) = app.get_webview_window(POPOVER_WINDOW) {
        report_failure("Hiding the popover", popover.hide());
    }
}

/// Where the location permission is given once the system stopped asking.
const LOCATION_SETTINGS_URL: Option<&str> = if cfg!(target_os = "macos") {
    Some("x-apple.systempreferences:com.apple.preference.security?Privacy_LocationServices")
} else if cfg!(target_os = "windows") {
    Some("ms-settings:privacy-location")
} else {
    None
};

/// The note shown in window mode; `None` when the tray works.
#[tauri::command]
#[specta::specta]
#[must_use]
#[expect(
    clippy::needless_pass_by_value,
    reason = "Tauri injects command arguments by value"
)]
pub fn read_window_mode_texts(hint: State<'_, Option<WindowModeTexts>>) -> Option<WindowModeTexts> {
    hint.inner().clone()
}

/// Opens the extension page in the default browser.
#[tauri::command]
#[specta::specta]
pub fn open_tray_extension_page() {
    if let Err(error) = tauri_plugin_opener::open_url(APPINDICATOR_EXTENSION_URL, None::<&str>) {
        eprintln!("Opening the extension page failed: {error}");
    }
}

/// Quits from the dashboard, the only way out in window mode.
#[tauri::command]
#[specta::specta]
#[expect(
    clippy::needless_pass_by_value,
    reason = "Tauri injects command arguments by value"
)]
pub fn quit_app(app: AppHandle) {
    app.exit(0);
}

/// Asks for the location permission that reveals the Wi-Fi name, or opens the system settings
/// where it can be given once the question was answered.
#[tauri::command]
#[specta::specta]
#[expect(
    clippy::needless_pass_by_value,
    reason = "Tauri injects command arguments by value"
)]
pub fn reveal_network_name(app: AppHandle) {
    // CoreLocation answers through the main run loop.
    let asked = app.run_on_main_thread(|| {
        if request_location_permission() == LocationRequest::NeedsSettings
            && let Some(url) = LOCATION_SETTINGS_URL
            && let Err(error) = tauri_plugin_opener::open_url(url, None::<&str>)
        {
            eprintln!("Opening the location settings failed: {error}");
        }
    });
    report_failure("Asking for the location permission", asked);
}

/// Why the history is not what it was, if something happened to it at launch.
#[tauri::command]
#[specta::specta]
#[must_use]
#[expect(
    clippy::needless_pass_by_value,
    reason = "Tauri injects command arguments by value"
)]
pub fn read_history_notice(notice: State<'_, HistoryNotice>) -> Option<String> {
    notice.0.clone()
}

/// The day so far and the networks of the last 30 days. `utc_offset_seconds` is the user's time
/// zone, ahead of UTC: the day starts at local midnight.
///
/// # Errors
///
/// The history could not be read.
#[tauri::command]
#[specta::specta]
pub async fn read_dashboard_summary(
    utc_offset_seconds: i32,
    inbox: State<'_, EngineInbox>,
    history: State<'_, DashboardHistory>,
    catalog: State<'_, TextCatalog>,
) -> Result<DashboardSummaryDisplay, String> {
    let live = inbox.read_live().await;
    let now = SystemClock::start().read_utc_seconds();
    let offset = i64::from(utc_offset_seconds);
    history
        .read(|reader| {
            let overview = read_day_overview(reader, &live, compute_day_start(now, offset), now)?;
            let summaries = list_network_summaries(reader, &live, now)?;
            Ok(DashboardSummaryDisplay::compose(
                &overview, &summaries, now, offset, &catalog,
            ))
        })
        .map_err(|error: application::RepositoryError| error.0)
}

/// The chart of `period`, with every metric: switching metric needs no new reading.
///
/// # Errors
///
/// The history could not be read.
#[tauri::command]
#[specta::specta]
pub async fn read_evolution(
    period: PeriodChoice,
    utc_offset_seconds: i32,
    inbox: State<'_, EngineInbox>,
    history: State<'_, DashboardHistory>,
    catalog: State<'_, TextCatalog>,
) -> Result<EvolutionDisplay, String> {
    let live = inbox.read_live().await;
    let now = SystemClock::start().read_utc_seconds();
    let period = period.into();
    history
        .read(|reader| application::read_evolution(reader, &live, period, now))
        .map(|evolution| {
            EvolutionDisplay::compose(&evolution, period, i64::from(utc_offset_seconds), &catalog)
        })
        .map_err(|error| error.0)
}

/// The settings as last saved, for the Réglages page.
#[tauri::command]
#[specta::specta]
#[must_use]
#[expect(
    clippy::needless_pass_by_value,
    reason = "Tauri injects command arguments by value"
)]
pub fn read_settings(store: State<'_, SettingsStore>) -> SettingsForm {
    SettingsForm::from_settings(&store.read())
}

/// Saves every setting, applies it, and says when it was saved. A new own server must answer a
/// ping first.
///
/// # Errors
///
/// The sentence shown under the form: an invalid address, a server that does not answer, or a
/// storage failure.
#[tauri::command]
#[specta::specta]
pub async fn save_settings(
    form: SettingsForm,
    utc_offset_seconds: i32,
    app: AppHandle,
    store: State<'_, SettingsStore>,
    inbox: State<'_, EngineInbox>,
    catalog: State<'_, TextCatalog>,
) -> Result<String, String> {
    let settings = form.to_settings(&catalog)?;
    if let MeasurementServers::Custom(target) = settings.servers
        && store.read().servers != settings.servers
        && !answers_ping(target).await
    {
        return Err(catalog.render("server-unreachable", None));
    }
    store.save(&settings).map_err(|error| {
        eprintln!("Saving the settings failed: {}", error.0);
        catalog.render("settings-not-saved", None)
    })?;
    apply_settings(&app, &settings);
    inbox.send_settings(settings).await;
    Ok(confirm_saved(
        SystemClock::start().read_utc_seconds(),
        i64::from(utc_offset_seconds),
        &catalog,
    ))
}

/// One probe, as the engine sends them: an answer or a refused connection both prove the server
/// is there. At most one probe timeout.
async fn answers_ping(target: Target) -> bool {
    FallbackLatencyProbe::new(IcmpEchoSender, TcpConnectionOpener)
        .send_probe(target)
        .await
        != ProbeOutcome::Unreachable
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn read_app_info_reports_the_package_name_and_version() {
        // Act
        let info = read_app_info();

        // Assert
        assert_eq!(info.name, "app");
        assert_eq!(info.version, env!("CARGO_PKG_VERSION"));
    }
}
