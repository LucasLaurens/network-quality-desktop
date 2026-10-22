//! Publishes each verdict to the shell: the menu-bar dot and the display object the popover renders.

use std::sync::{Arc, Mutex, PoisonError};

use application::{VerdictPresenter, VerdictView};
use domain::{ConnectionContext, Verdict};
use serde::Serialize;
use specta::Type;
use tauri::AppHandle;
use tauri::image::Image;
use tauri::tray::TrayIcon;
use tauri_specta::Event;

use crate::display::{ConnectionDisplay, DisplayState};
use crate::text_catalog::TextCatalog;
use crate::tray_icon::{ICON_SIZE, render_verdict_icon};
use crate::tray_menu::{TrayMenu, compose_menu_entries};

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

/// The connection last measured, for the commands that depend on it (a throughput test's
/// warning on a limited connection).
#[derive(Clone, Default)]
pub struct ConnectionStore(Arc<Mutex<Option<ConnectionContext>>>);

impl ConnectionStore {
    #[must_use]
    pub fn read(&self) -> Option<ConnectionContext> {
        *self.0.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn replace(&self, context: Option<ConnectionContext>) {
        *self.0.lock().unwrap_or_else(PoisonError::into_inner) = context;
    }
}

pub struct ShellPresenter {
    app: AppHandle,
    connection: ConnectionStore,
    tray: TrayIcon,
    menu: TrayMenu,
    catalog: TextCatalog,
    store: DisplayStore,
    shown: Mutex<Verdict>,
}

impl ShellPresenter {
    /// `tray` must already show `Verdict::Measuring`, as built by `create_icon_image`.
    #[must_use]
    pub fn new(
        app: AppHandle,
        tray: TrayIcon,
        menu: TrayMenu,
        catalog: TextCatalog,
        store: DisplayStore,
        connection: ConnectionStore,
    ) -> Self {
        Self {
            app,
            connection,
            tray,
            menu,
            catalog,
            store,
            shown: Mutex::new(Verdict::Measuring),
        }
    }

    fn update_tray(&self, verdict: Verdict, label: &str) {
        let mut shown = self.shown.lock().unwrap_or_else(PoisonError::into_inner);
        if *shown == verdict {
            return;
        }
        if let Err(error) = self.tray.set_icon(Some(create_icon_image(verdict))) {
            eprintln!("Updating the menu-bar icon failed: {error}");
            return;
        }
        // Linux ignores tooltips without failing; an error elsewhere is worth knowing about.
        if let Err(error) = self.tray.set_tooltip(Some(label)) {
            eprintln!("Updating the menu-bar tooltip failed: {error}");
        }
        *shown = verdict;
    }
}

impl VerdictPresenter for ShellPresenter {
    fn present(&self, view: &VerdictView) {
        self.connection.replace(view.context);
        let display = ConnectionDisplay::from_view(view, &self.catalog);
        self.update_tray(view.verdict, &display.state_label);
        if !self.store.replace(display.clone()) {
            return;
        }
        self.menu
            .update(&compose_menu_entries(&display, &self.catalog));
        if let Err(error) = ConnectionDisplayChanged(display).emit(&self.app) {
            eprintln!("Publishing the connection display failed: {error}");
        }
    }
}

#[must_use]
pub fn create_icon_image(verdict: Verdict) -> Image<'static> {
    Image::new_owned(
        render_verdict_icon(verdict, ICON_SIZE),
        ICON_SIZE,
        ICON_SIZE,
    )
}

/// The tray tooltip uses the same catalogue label as the popover.
#[must_use]
pub fn describe_verdict(verdict: Verdict, catalog: &TextCatalog) -> String {
    catalog.render(DisplayState::from(verdict).to_catalogue_key(), None)
}
