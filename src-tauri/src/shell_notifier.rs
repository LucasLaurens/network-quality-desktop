//! The system notification of a durable degradation: the state as its title, the finding as its
//! body, and neither an action nor a figure.

use std::sync::Arc;

use application::{Notifier, VerdictView};
use tauri::AppHandle;
use tauri_plugin_notification::NotificationExt;

use crate::display::ConnectionDisplay;
use crate::text_catalog::TextCatalog;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Notice {
    pub title: String,
    pub body: String,
}

impl Notice {
    /// The same words as the dot and the popover. O(1).
    #[must_use]
    pub fn compose(view: &VerdictView, catalog: &TextCatalog) -> Self {
        let display = ConnectionDisplay::from_view(view, catalog);
        Self {
            title: display.state_label,
            body: display.cause,
        }
    }
}

pub struct ShellNotifier {
    app: AppHandle,
    catalog: Arc<TextCatalog>,
}

impl ShellNotifier {
    #[must_use]
    pub const fn new(app: AppHandle, catalog: Arc<TextCatalog>) -> Self {
        Self { app, catalog }
    }
}

impl Notifier for ShellNotifier {
    fn notify_degradation(&self, view: &VerdictView) {
        let notice = Notice::compose(view, &self.catalog);
        let shown = self
            .app
            .notification()
            .builder()
            .title(notice.title)
            .body(notice.body)
            .show();
        if let Err(error) = shown {
            eprintln!("Showing the degradation notice failed: {error}");
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use application::MetricReading;
    use domain::{
        Diagnosis, Fault, Grade, GradeBounds, LocalSegment, Metric, Unmeasurable, Verdict,
    };

    use super::*;

    #[test]
    fn titles_the_state_and_tells_the_finding_without_figures() {
        // Arrange
        let catalog = TextCatalog::load();
        let view = VerdictView {
            verdict: Verdict::Graded(Grade::Unstable),
            diagnosis: Diagnosis::OutOfThreshold {
                metric: Metric::Latency,
                fault: Fault::Operator,
            },
            latency: Some(MetricReading {
                value: Duration::from_millis(180),
                grade: Grade::Unstable,
                is_near_next_threshold: false,
                bounds: GradeBounds::new(Duration::from_millis(100), Duration::from_millis(300)),
            }),
            jitter: None,
            loss: None,
            context: None,
            local: LocalSegment::Unmeasurable(Unmeasurable::NoGateway),
            wifi: None,
        };

        // Act
        let notice = Notice::compose(&view, &catalog);

        // Assert
        assert_eq!(notice.title, "Instable");
        assert!(notice.body.starts_with("L'opérateur est en cause"));
        assert!(
            !notice
                .body
                .chars()
                .any(|character| character.is_ascii_digit())
        );
    }
}
