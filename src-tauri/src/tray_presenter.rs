//! Shows each verdict as the menu-bar dot; only redraws when the verdict changes.

use std::sync::{Mutex, PoisonError};

use application::{VerdictPresenter, VerdictView};
use domain::{Grade, Verdict};
use tauri::image::Image;
use tauri::tray::TrayIcon;

use crate::tray_icon::{ICON_SIZE, render_verdict_icon};

pub struct TrayPresenter {
    tray: TrayIcon,
    shown: Mutex<Verdict>,
}

impl TrayPresenter {
    #[must_use]
    pub fn new(tray: TrayIcon) -> Self {
        Self {
            tray,
            shown: Mutex::new(Verdict::Measuring),
        }
    }
}

impl VerdictPresenter for TrayPresenter {
    fn present(&self, view: &VerdictView) {
        let mut shown = self.shown.lock().unwrap_or_else(PoisonError::into_inner);
        if *shown == view.verdict {
            return;
        }
        if let Err(error) = self.tray.set_icon(Some(create_icon_image(view.verdict))) {
            eprintln!("Updating the menu-bar icon failed: {error}");
            return;
        }
        // Linux ignores tooltips without failing; an error elsewhere is worth knowing about.
        if let Err(error) = self.tray.set_tooltip(Some(describe_verdict(view.verdict))) {
            eprintln!("Updating the menu-bar tooltip failed: {error}");
        }
        *shown = view.verdict;
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

/// Provisional labels until the Fluent catalogue arrives (ticket « Popover et textes »).
#[must_use]
pub const fn describe_verdict(verdict: Verdict) -> &'static str {
    match verdict {
        Verdict::Measuring => "Mesure en cours",
        Verdict::Graded(Grade::Solid) => "Solide",
        Verdict::Graded(Grade::Unstable) => "Instable",
        Verdict::Graded(Grade::Degraded) => "Dégradée",
    }
}
