//! Publishes each verdict to the shell: the menu-bar dot and the display object the popover renders.

use std::sync::{Arc, Mutex, PoisonError};

use application::{VerdictPresenter, VerdictView};
use std::time::Duration;

use application::ColorSchemeReader;
use domain::{ColorScheme, ConnectionContext, Verdict, WifiLink};
use infrastructure::SystemColorSchemeReader;
use serde::Serialize;
use specta::Type;
use tauri::AppHandle;
use tauri::image::Image;
use tauri::tray::TrayIcon;
use tauri_specta::Event;

use crate::display::{ConnectionDisplay, DisplayState};
use crate::text_catalog::TextCatalog;
use crate::tray_icon::{ICON_SIZE, render_verdict_icon};
use crate::tray_menu::TrayMenu;

/// Sent to every window whenever what the popover shows changes.
#[derive(Debug, Clone, Serialize, Type, Event)]
pub struct ConnectionDisplayChanged(pub ConnectionDisplay);

/// The latest display, read by windows that open after it was emitted.
#[derive(Clone)]
pub struct DisplayStore(Arc<Mutex<ConnectionDisplay>>);

impl DisplayStore {
    #[must_use]
    pub fn new(initial: ConnectionDisplay) -> Self {
        Self(Arc::new(Mutex::new(initial)))
    }

    #[must_use]
    pub fn read(&self) -> ConnectionDisplay {
        self.0
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }

    /// Stores the display and says whether it differs from the previous one.
    fn replace(&self, display: ConnectionDisplay) -> bool {
        let mut current = self.0.lock().unwrap_or_else(PoisonError::into_inner);
        let changed = *current != display;
        *current = display;
        changed
    }
}

/// What the last verdict said of the connection, beside the verdict itself.
#[derive(Debug, Clone, Default)]
pub struct ConnectionReading {
    /// `None` without a default route.
    pub context: Option<ConnectionContext>,
    pub wifi: Option<WifiLink>,
    /// The latency at rest, as shown.
    pub latency: Option<Duration>,
}

/// The connection last measured, for the commands that depend on it (a throughput test's
/// warning on a limited connection, the technical details).
#[derive(Clone, Default)]
pub struct ConnectionStore(Arc<Mutex<ConnectionReading>>);

impl ConnectionStore {
    #[must_use]
    pub fn read(&self) -> Option<ConnectionContext> {
        self.0
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .context
    }

    #[must_use]
    pub fn read_all(&self) -> ConnectionReading {
        self.0
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }

    fn replace(&self, view: &VerdictView) {
        *self.0.lock().unwrap_or_else(PoisonError::into_inner) = ConnectionReading {
            context: view.context,
            wifi: view.wifi.clone(),
            latency: view.latency.map(|reading| reading.value),
        };
    }
}

pub struct ShellPresenter {
    app: AppHandle,
    connection: ConnectionStore,
    tray: TrayIcon,
    menu: Arc<TrayMenu>,
    catalog: TextCatalog,
    store: DisplayStore,
    color_scheme: SystemColorSchemeReader,
    /// What the icon shows: the verdict, in the colours of the system's scheme.
    shown: Mutex<(Verdict, ColorScheme)>,
}

impl ShellPresenter {
    /// `tray` must already show `Verdict::Measuring`, as built by `create_icon_image`, in the
    /// colours of `scheme`.
    #[must_use]
    pub fn new(
        app: AppHandle,
        tray: TrayIcon,
        menu: Arc<TrayMenu>,
        catalog: TextCatalog,
        store: DisplayStore,
        connection: ConnectionStore,
        scheme: ColorScheme,
    ) -> Self {
        Self {
            app,
            connection,
            tray,
            menu,
            catalog,
            store,
            color_scheme: SystemColorSchemeReader,
            shown: Mutex::new((Verdict::Measuring, scheme)),
        }
    }

    /// Redrawn when the verdict or the system's scheme changes: the scheme is read each second,
    /// so a switch to dark shows within a second, whatever the state. O(1) when nothing changed.
    fn update_tray(&self, verdict: Verdict, label: &str) {
        let wanted = (verdict, self.color_scheme.read_color_scheme());
        let mut shown = self.shown.lock().unwrap_or_else(PoisonError::into_inner);
        if *shown == wanted {
            return;
        }
        if let Err(error) = self
            .tray
            .set_icon(Some(create_icon_image(verdict, wanted.1)))
        {
            eprintln!("Updating the menu-bar icon failed: {error}");
            return;
        }
        // Linux ignores tooltips without failing; an error elsewhere is worth knowing about.
        if let Err(error) = self.tray.set_tooltip(Some(label)) {
            eprintln!("Updating the menu-bar tooltip failed: {error}");
        }
        *shown = wanted;
    }
}

impl VerdictPresenter for ShellPresenter {
    fn present(&self, view: &VerdictView) {
        self.connection.replace(view);
        let display = ConnectionDisplay::from_view(view, &self.catalog);
        self.update_tray(view.verdict, &display.state_label);
        if !self.store.replace(display.clone()) {
            return;
        }
        self.menu.show_display(&display, &self.catalog);
        if let Err(error) = ConnectionDisplayChanged(display).emit(&self.app) {
            eprintln!("Publishing the connection display failed: {error}");
        }
    }
}

#[must_use]
pub fn create_icon_image(verdict: Verdict, scheme: ColorScheme) -> Image<'static> {
    Image::new_owned(
        render_verdict_icon(verdict, scheme, ICON_SIZE),
        ICON_SIZE,
        ICON_SIZE,
    )
}

/// The tray tooltip uses the same catalogue label as the popover.
#[must_use]
pub fn describe_verdict(verdict: Verdict, catalog: &TextCatalog) -> String {
    catalog.render(DisplayState::from(verdict).to_catalogue_key(), None)
}
