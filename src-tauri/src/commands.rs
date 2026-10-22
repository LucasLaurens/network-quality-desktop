//! Typed commands exposed to the front end. Tauri cannot export `pub` commands from the crate root.

use std::collections::BTreeMap;

use serde::Serialize;
use specta::Type;
use tauri::{AppHandle, Manager, State};
use tauri_plugin_dialog::DialogExt;

use application::{
    Clock, can_measure_against, detect_access, list_network_summaries, read_day_overview,
};
use domain::TestProfile;
use infrastructure::{
    LocationRequest, SystemClock, UpnpAccessDetector, request_location_permission,
};

use crate::create_latency_probe;
use crate::dashboard_display::{
    DashboardSummaryDisplay, EvolutionDisplay, PeriodChoice, compute_day_start,
};
use crate::display::ConnectionDisplay;
use crate::engine_inbox::EngineInbox;
use crate::history_export::{
    ExportAvailability, ExportDone, ExportFolder, ExportPeriodChoice, HistoryLocation, write_export,
};
use crate::history_store::{DashboardHistory, HistoryNotice};
use crate::settings_effects::apply_settings;
use crate::settings_form::{SettingsForm, format_save_confirmation};
use crate::settings_store::SettingsStore;
use crate::shell_mode::{APPINDICATOR_EXTENSION_URL, WindowModeTexts};
use crate::shell_presenter::ConnectionStore;
use crate::shell_presenter::DisplayStore;
use crate::technical_details::{LineDetails, TechnicalDetails};
use crate::text_catalog::TextCatalog;
use crate::throughput_display::{
    ProfileChoice, ThroughputOutcome, ThroughputPanel, ThroughputRow, compose_warning,
};
use crate::throughput_lab::{LabRefusal, ThroughputLab};
use crate::traffic_store::TrafficStore;
use crate::windows::{
    DashboardRequested, MAIN_WINDOW, POPOVER_WINDOW, report_failure, request_from_dashboard,
    show_dashboard,
};

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

/// Every word of the windows, read once when a window opens. O(c) for a catalogue of c characters.
#[tauri::command]
#[specta::specta]
#[must_use]
#[expect(
    clippy::needless_pass_by_value,
    reason = "Tauri injects command arguments by value"
)]
pub fn read_interface_texts(catalog: State<'_, TextCatalog>) -> BTreeMap<String, String> {
    catalog.list_interface_texts()
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

/// Opens the settings page of the dashboard and closes the popover.
#[tauri::command]
#[specta::specta]
#[expect(
    clippy::needless_pass_by_value,
    reason = "Tauri injects command arguments by value"
)]
pub fn open_settings(app: AppHandle) {
    close_popover(app.clone());
    request_from_dashboard(&app, DashboardRequested::Settings);
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
    let previous = store.read().servers;
    if !can_measure_against(previous, settings.servers, &create_latency_probe()).await {
        return Err(catalog.render("server-unreachable", None));
    }
    store.save(&settings).map_err(|error| {
        eprintln!("Saving the settings failed: {}", error.0);
        catalog.render("settings-not-saved", None)
    })?;
    apply_settings(&app, &settings);
    inbox.send_settings(settings).await;
    Ok(format_save_confirmation(
        SystemClock::start().read_utc_seconds(),
        i64::from(utc_offset_seconds),
        store.read_durability(),
        &catalog,
    ))
}

/// The tests shown at most, newest first.
const THROUGHPUT_ROWS: usize = 20;

/// The dashboard's throughput section: whether tests are allowed, what they may use, and the
/// last tests.
///
/// # Errors
///
/// The history could not be read.
#[tauri::command]
#[specta::specta]
pub async fn read_throughput_panel(
    utc_offset_seconds: i32,
    settings: State<'_, SettingsStore>,
    history: State<'_, DashboardHistory>,
    catalog: State<'_, TextCatalog>,
) -> Result<ThroughputPanel, String> {
    let now = SystemClock::start().read_utc_seconds();
    let mut records = history
        .read(|reader| reader.list_throughput_tests(0, now + 1))
        .map_err(|error| error.0)?;
    records.truncate(THROUGHPUT_ROWS);
    Ok(ThroughputPanel::compose(
        settings.read().allows_throughput_tests,
        &records,
        i64::from(utc_offset_seconds),
        &catalog,
    ))
}

/// Runs the test the user clicked. Where data counts, the first click only returns the warning:
/// the test runs once the user confirms, the connection read again at that moment. The engine
/// stops measuring while the line is full, then keeps the result.
///
/// # Errors
///
/// The sentence shown under the buttons: tests not allowed, one already running, or a server
/// that did not answer.
#[tauri::command]
#[specta::specta]
#[expect(
    clippy::too_many_arguments,
    reason = "Tauri injects each state as an argument"
)]
pub async fn run_throughput_test(
    profile: ProfileChoice,
    is_confirmed: bool,
    utc_offset_seconds: i32,
    settings: State<'_, SettingsStore>,
    lab: State<'_, ThroughputLab>,
    inbox: State<'_, EngineInbox>,
    connection: State<'_, ConnectionStore>,
    catalog: State<'_, TextCatalog>,
) -> Result<ThroughputOutcome, String> {
    let profile = TestProfile::from(profile);
    if !settings.read().allows_throughput_tests {
        return Err(catalog.render("throughput-disabled", None));
    }
    if let Some(warning) = compose_warning(connection.read().as_ref(), profile, &catalog)
        && !is_confirmed
    {
        return Ok(ThroughputOutcome::NeedsConfirmation { warning });
    }
    let refuse = |refusal| {
        let key = match refusal {
            LabRefusal::Busy => "throughput-busy",
            LabRefusal::Unavailable => "throughput-unreachable",
        };
        catalog.render(key, None)
    };
    let test = lab.begin().map_err(refuse)?;
    inbox.send_throughput_started().await;
    let result = test.run(profile).await;
    drop(test);
    inbox.send_throughput_ended(result.ok()).await;
    let result = result.map_err(refuse)?;
    Ok(ThroughputOutcome::Done {
        row: ThroughputRow::compose(
            SystemClock::start().read_utc_seconds(),
            &result,
            i64::from(utc_offset_seconds),
            &catalog,
        ),
    })
}

/// The catalogue key naming the file manager that shows an exported folder.
const REVEAL_EXPORT_KEY: &str = if cfg!(target_os = "macos") {
    "export-reveal-finder"
} else if cfg!(target_os = "windows") {
    "export-reveal-explorer"
} else {
    "export-reveal-folder"
};

/// Asks for the export folder in the system's dialog, attached to the dashboard; `false` when the
/// dialog was closed without a choice.
///
/// # Errors
///
/// The sentence shown under the button when the dialog could not open.
#[tauri::command]
#[specta::specta]
pub async fn choose_export_folder(
    app: AppHandle,
    export_folder: State<'_, ExportFolder>,
    catalog: State<'_, TextCatalog>,
) -> Result<bool, String> {
    let (chosen, picked) = tokio::sync::oneshot::channel();
    let mut dialog = app.dialog().file();
    if let Some(dashboard) = app.get_webview_window(MAIN_WINDOW) {
        dialog = dialog.set_parent(&dashboard);
    }
    dialog.pick_folder(move |folder| {
        let _ = chosen.send(folder);
    });
    // A dropped sender means the dialog never opened: an error, not a choice.
    let picked = picked.await.map_err(|_| {
        eprintln!("The folder dialog could not open");
        catalog.render("export-failed", None)
    })?;
    let Some(folder) = picked else {
        return Ok(false);
    };
    let folder = folder.into_path().map_err(|error| {
        eprintln!("The chosen folder is not a path: {error}");
        catalog.render("export-failed", None)
    })?;
    export_folder.remember(folder);
    Ok(true)
}

/// Writes the history of `period` as three CSV files into the chosen folder, on a thread of its
/// own.
///
/// # Errors
///
/// The sentence shown under the button: a history kept in memory, no folder chosen, or a failed
/// export.
#[tauri::command]
#[specta::specta]
pub async fn export_history(
    period: ExportPeriodChoice,
    location: State<'_, HistoryLocation>,
    export_folder: State<'_, ExportFolder>,
    catalog: State<'_, TextCatalog>,
) -> Result<ExportDone, String> {
    let HistoryLocation::OnDisk(history) = location.inner() else {
        return Err(catalog.render("export-unavailable", None));
    };
    let failed = || catalog.render("export-failed", None);
    let folder = export_folder.read().ok_or_else(failed)?;
    let history = history.clone();
    let now = SystemClock::start().read_utc_seconds();
    let (answer, answered) = tokio::sync::oneshot::channel();
    std::thread::Builder::new()
        .name("export".to_owned())
        .spawn(move || {
            let _ = answer.send(write_export(&history, &folder, period.into(), now));
        })
        .map_err(|error| {
            eprintln!("Starting the export failed: {error}");
            failed()
        })?;
    answered
        .await
        .map_err(|error| {
            eprintln!("The export thread stopped: {error}");
            failed()
        })?
        .map_err(|error| {
            eprintln!("The export failed: {error:?}");
            failed()
        })?;
    Ok(ExportDone {
        message: catalog.render("export-done", None),
        reveal_label: catalog.render(REVEAL_EXPORT_KEY, None),
    })
}

/// Shows the export folder in the system's file manager.
#[tauri::command]
#[specta::specta]
#[expect(
    clippy::needless_pass_by_value,
    reason = "Tauri injects command arguments by value"
)]
pub fn reveal_export_folder(export_folder: State<'_, ExportFolder>) {
    let Some(folder) = export_folder.read() else {
        return;
    };
    if let Err(error) = tauri_plugin_opener::reveal_item_in_dir(&folder) {
        eprintln!("Showing the export failed: {error}");
    }
}

/// Whether there is anything to export, and why not: a history kept in memory, or nothing kept
/// yet.
#[tauri::command]
#[specta::specta]
#[must_use]
#[expect(
    clippy::needless_pass_by_value,
    reason = "Tauri injects command arguments by value"
)]
pub fn read_export_availability(
    location: State<'_, HistoryLocation>,
    history: State<'_, DashboardHistory>,
    catalog: State<'_, TextCatalog>,
) -> ExportAvailability {
    let unavailable = |key: &str| ExportAvailability::Unavailable {
        reason: catalog.render(key, None),
    };
    if matches!(location.inner(), HistoryLocation::InMemory) {
        return unavailable("export-unavailable");
    }
    // Unreadable here, the export may still read it through its own connection: let it try.
    let is_empty = history
        .read(|reader| reader.is_history_empty())
        .unwrap_or_else(|error| {
            eprintln!("Reading whether the history is empty failed: {}", error.0);
            false
        });
    if is_empty {
        unavailable("export-nothing")
    } else {
        ExportAvailability::Ready
    }
}

/// Erases every measure, event and throughput test; the settings stay.
///
/// # Errors
///
/// The sentence shown under the button when the erase did not finish.
#[tauri::command]
#[specta::specta]
pub async fn erase_history(
    inbox: State<'_, EngineInbox>,
    catalog: State<'_, TextCatalog>,
) -> Result<String, String> {
    let inbox = inbox.inner().clone();
    let is_erased = tauri::async_runtime::spawn_blocking(move || inbox.erase_history())
        .await
        .unwrap_or(false);
    if is_erased {
        Ok(catalog.render("history-erased", None))
    } else {
        Err(catalog.render("history-not-erased", None))
    }
}

/// The connection, its Wi-Fi and the traffic seen: quick, read again while the details are open.
#[tauri::command]
#[specta::specta]
#[must_use]
#[expect(
    clippy::needless_pass_by_value,
    reason = "Tauri injects command arguments by value"
)]
pub fn read_technical_details(
    connection: State<'_, ConnectionStore>,
    traffic: State<'_, TrafficStore>,
    catalog: State<'_, TextCatalog>,
) -> TechnicalDetails {
    TechnicalDetails::compose(&connection.read_all(), traffic.read(), &catalog)
}

/// What carries the line: the box asked over `UPnP` (a few seconds at most), then the estimate
/// from this network's throughput tests and the latency at rest.
///
/// # Errors
///
/// The sentence shown in the panel when the history could not be read.
#[tauri::command]
#[specta::specta]
pub async fn read_line_details(
    connection: State<'_, ConnectionStore>,
    inbox: State<'_, EngineInbox>,
    history: State<'_, DashboardHistory>,
    catalog: State<'_, TextCatalog>,
) -> Result<LineDetails, String> {
    let network = inbox.read_live().await.network;
    let now = SystemClock::start().read_utc_seconds();
    let tests = history
        .read(|reader| reader.list_throughput_tests(0, now + 1))
        .map_err(|error| {
            eprintln!("Reading the throughput tests failed: {}", error.0);
            catalog.render("details-line-failed", None)
        })?;
    let reading = connection.read_all();
    let detector =
        UpnpAccessDetector::new(reading.context.and_then(|context| context.gateway_address));
    let line = detect_access(&detector, &tests, network.as_ref(), reading.latency).await;
    Ok(LineDetails::compose(&line, &catalog))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_the_file_manager_of_each_system() {
        // Arrange
        let catalog = TextCatalog::load();

        // Act & Assert
        for key in [
            "export-reveal-finder",
            "export-reveal-explorer",
            "export-reveal-folder",
        ] {
            assert!(!catalog.render(key, None).is_empty(), "{key}");
        }
        assert!(!catalog.render(REVEAL_EXPORT_KEY, None).is_empty());
    }

    #[test]
    fn read_app_info_reports_the_package_name_and_version() {
        // Act
        let info = read_app_info();

        // Assert
        assert_eq!(info.name, "app");
        assert_eq!(info.version, env!("CARGO_PKG_VERSION"));
    }
}
