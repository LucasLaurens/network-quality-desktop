//! Turns each verdict into the ready-to-show object the front end renders: states, texts, gauges.

use std::time::Duration;

use application::{MetricReading, VerdictView};
use domain::{ConnectionContext, ConnectionKind, Diagnosis, Grade, LossRate, Metric, Verdict};
use fluent_bundle::FluentArgs;
use serde::Serialize;
use specta::Type;

use crate::text_catalog::TextCatalog;

/// Right end of each gauge: values beyond it pin the marker to the end.
const LATENCY_GAUGE_MAX: Duration = Duration::from_millis(400);
const JITTER_GAUGE_MAX: Duration = Duration::from_millis(70);
const LOSS_GAUGE_MAX: LossRate = LossRate::from_percent(8.0);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum DisplayState {
    Solid,
    Unstable,
    Degraded,
    Offline,
    Measuring,
}

impl From<Verdict> for DisplayState {
    fn from(verdict: Verdict) -> Self {
        match verdict {
            Verdict::Graded(grade) => grade.into(),
            Verdict::Offline => Self::Offline,
            Verdict::Measuring => Self::Measuring,
        }
    }
}

impl From<Grade> for DisplayState {
    fn from(grade: Grade) -> Self {
        match grade {
            Grade::Solid => Self::Solid,
            Grade::Unstable => Self::Unstable,
            Grade::Degraded => Self::Degraded,
        }
    }
}

impl DisplayState {
    #[must_use]
    pub const fn to_catalogue_key(self) -> &'static str {
        match self {
            Self::Solid => "state-solid",
            Self::Unstable => "state-unstable",
            Self::Degraded => "state-degraded",
            Self::Offline => "state-offline",
            Self::Measuring => "state-measuring",
        }
    }
}

/// Positions on a gauge, as fractions of its length (0 = left end, 1 = right end), and the
/// thresholds in words for screen readers.
#[derive(Debug, Clone, PartialEq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Gauge {
    pub position: f64,
    pub unstable_from: f64,
    pub degraded_above: f64,
    pub label: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct MetricDisplay {
    pub name: String,
    pub method: String,
    /// `None` while the metric cannot be computed yet.
    pub value: Option<String>,
    pub gauge: Option<Gauge>,
    /// Shown under a metric close to its next threshold.
    pub hint: Option<String>,
}

/// The network in words: what to call it, and how the computer reaches it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct NetworkDisplay {
    pub label: String,
    pub connection: String,
}

impl NetworkDisplay {
    /// The Wi-Fi name will replace the generic label once it can be read. O(1).
    #[must_use]
    pub fn from_context(context: &ConnectionContext, catalog: &TextCatalog) -> Self {
        let (label_key, connection_key) = match (context.kind, context.is_vpn) {
            (ConnectionKind::Ethernet, _) => ("network-ethernet", "connection-ethernet"),
            (ConnectionKind::WiFi, _) => ("network-wifi", "connection-wifi"),
            (ConnectionKind::Cellular, _) => ("network-cellular", "connection-cellular"),
            (ConnectionKind::PhoneHotspot, _) => ("network-hotspot", "connection-hotspot"),
            (ConnectionKind::Other, true) => ("network-vpn", "connection-vpn"),
            (ConnectionKind::Other, false) => ("network-other", "connection-other"),
        };
        let mut connection = catalog.render(connection_key, None);
        if context.is_vpn && context.kind != ConnectionKind::Other {
            let mut args = FluentArgs::new();
            args.set("connection", connection);
            connection = catalog.render("connection-through-vpn", Some(&args));
        }
        Self {
            label: catalog.render(label_key, None),
            connection,
        }
    }
}

/// Everything the popover shows, already worded and laid out: the front end adds no rule.
#[derive(Debug, Clone, PartialEq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ConnectionDisplay {
    pub state: DisplayState,
    pub state_label: String,
    pub cause: String,
    pub action: Option<String>,
    pub detail: Option<String>,
    pub metrics: Vec<MetricDisplay>,
    pub open_dashboard_label: String,
    /// `None` without a default route.
    pub network: Option<NetworkDisplay>,
}

impl ConnectionDisplay {
    /// What the popover shows before the first second was measured.
    #[must_use]
    pub fn create_initial(catalog: &TextCatalog) -> Self {
        let view = VerdictView {
            verdict: Verdict::Measuring,
            diagnosis: Diagnosis::Measuring {
                collected: 0,
                needed: application::WARM_UP_SAMPLES,
            },
            latency: None,
            jitter: None,
            loss: None,
            context: None,
        };
        Self::from_view(&view, catalog)
    }

    /// O(1): three metrics and a handful of short messages.
    #[must_use]
    pub fn from_view(view: &VerdictView, catalog: &TextCatalog) -> Self {
        let state = DisplayState::from(view.verdict);
        let (cause_key, action_key) = match view.diagnosis {
            Diagnosis::Healthy => ("cause-healthy", None),
            Diagnosis::NearThreshold(_) => ("cause-near-threshold", None),
            Diagnosis::OutOfThreshold(_) => ("cause-out-of-threshold", None),
            Diagnosis::Recovering => ("cause-recovering", None),
            Diagnosis::Measuring { .. } => ("cause-measuring", None),
            Diagnosis::NoServerReachable => ("cause-no-server", Some("action-check-connection")),
        };
        Self {
            state,
            state_label: catalog.render(state.to_catalogue_key(), None),
            cause: catalog.render(cause_key, None),
            action: action_key.map(|key| catalog.render(key, None)),
            detail: compose_detail(view, catalog),
            metrics: vec![
                compose_metric(
                    Metric::Latency,
                    view.latency.as_ref(),
                    LATENCY_GAUGE_MAX,
                    catalog,
                ),
                compose_metric(
                    Metric::Jitter,
                    view.jitter.as_ref(),
                    JITTER_GAUGE_MAX,
                    catalog,
                ),
                compose_metric(Metric::Loss, view.loss.as_ref(), LOSS_GAUGE_MAX, catalog),
            ],
            open_dashboard_label: catalog.render("action-open-dashboard", None),
            network: view
                .context
                .map(|context| NetworkDisplay::from_context(&context, catalog)),
        }
    }
}

/// A value that can be printed with its unit and placed on a gauge.
trait Displayable: domain::Scalable {
    fn format(self, catalog: &TextCatalog) -> String;
    /// Where the value sits between zero and `max`, capped at 1.
    fn compute_fraction(self, max: Self) -> f64;
}

impl Displayable for Duration {
    fn format(self, catalog: &TextCatalog) -> String {
        let mut args = FluentArgs::new();
        args.set("value", self.as_millis().to_string());
        catalog.render("unit-milliseconds", Some(&args))
    }

    fn compute_fraction(self, max: Self) -> f64 {
        self.div_duration_f64(max).min(1.0)
    }
}

impl Displayable for LossRate {
    fn format(self, catalog: &TextCatalog) -> String {
        let rounded = format!("{:.1}", self.to_percent());
        let trimmed = rounded
            .strip_suffix(".0")
            .unwrap_or(&rounded)
            .replace('.', ",");
        let mut args = FluentArgs::new();
        args.set("value", trimmed);
        catalog.render("unit-percent", Some(&args))
    }

    fn compute_fraction(self, max: Self) -> f64 {
        (self.to_percent() / max.to_percent()).min(1.0)
    }
}

const fn to_name_key(metric: Metric) -> &'static str {
    match metric {
        Metric::Latency => "metric-latency",
        Metric::Jitter => "metric-jitter",
        Metric::Loss => "metric-loss",
    }
}

const fn to_method_key(metric: Metric) -> &'static str {
    match metric {
        Metric::Latency => "method-latency",
        Metric::Jitter => "method-jitter",
        Metric::Loss => "method-loss",
    }
}

/// O(1).
fn compose_metric<T: Displayable>(
    metric: Metric,
    reading: Option<&MetricReading<T>>,
    gauge_max: T,
    catalog: &TextCatalog,
) -> MetricDisplay {
    MetricDisplay {
        name: catalog.render(to_name_key(metric), None),
        method: catalog.render(to_method_key(metric), None),
        value: reading.map(|reading| reading.value.format(catalog)),
        gauge: reading.map(|reading| compose_gauge(reading, gauge_max, catalog)),
        hint: reading
            .filter(|reading| reading.is_near_next_threshold)
            .map(|_| catalog.render("hint-near-threshold", None)),
    }
}

/// O(1).
fn compose_gauge<T: Displayable>(
    reading: &MetricReading<T>,
    gauge_max: T,
    catalog: &TextCatalog,
) -> Gauge {
    let unstable_from = reading.bounds.read_unstable_from();
    let degraded_above = reading.bounds.read_degraded_above();
    let mut args = FluentArgs::new();
    args.set("unstable", unstable_from.format(catalog));
    args.set("degraded", degraded_above.format(catalog));
    Gauge {
        position: reading.value.compute_fraction(gauge_max),
        unstable_from: unstable_from.compute_fraction(gauge_max),
        degraded_above: degraded_above.compute_fraction(gauge_max),
        label: catalog.render("gauge-label", Some(&args)),
    }
}

/// O(1).
fn compose_detail(view: &VerdictView, catalog: &TextCatalog) -> Option<String> {
    let mut args = FluentArgs::new();
    match view.diagnosis {
        // Past the warm-up, a metric can still lack samples: "10 sur 10" would only confuse.
        Diagnosis::Measuring { collected, needed } if collected < needed => {
            args.set("collected", collected.to_string());
            args.set("needed", needed.to_string());
            Some(catalog.render("detail-measuring", Some(&args)))
        }
        Diagnosis::NearThreshold(metric) | Diagnosis::OutOfThreshold(metric) => {
            let (value, threshold) = match metric {
                Metric::Latency => describe_reading(view.latency.as_ref(), catalog)?,
                Metric::Jitter => describe_reading(view.jitter.as_ref(), catalog)?,
                Metric::Loss => describe_reading(view.loss.as_ref(), catalog)?,
            };
            args.set("metric", catalog.render(to_name_key(metric), None));
            args.set("value", value);
            args.set("threshold", threshold);
            Some(catalog.render("detail-metric", Some(&args)))
        }
        Diagnosis::Healthy
        | Diagnosis::Recovering
        | Diagnosis::Measuring { .. }
        | Diagnosis::NoServerReachable => None,
    }
}

/// The formatted value and the threshold it is compared with.
fn describe_reading<T: Displayable>(
    reading: Option<&MetricReading<T>>,
    catalog: &TextCatalog,
) -> Option<(String, String)> {
    let reading = reading?;
    let threshold = reading.bounds.find_reference_threshold(reading.value);
    Some((reading.value.format(catalog), threshold.format(catalog)))
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use application::{MetricReading, VerdictView};
    use domain::{Diagnosis, Grade, GradeBounds, LossRate, Metric, Scalable, Thresholds, Verdict};

    use super::*;

    const NBSP: char = '\u{a0}';

    fn create_reading<T: Scalable>(value: T, bounds: GradeBounds<T>) -> MetricReading<T> {
        MetricReading {
            value,
            grade: bounds.compute_grade(value),
            is_near_next_threshold: bounds.is_near_next_threshold(value),
            bounds,
        }
    }

    fn create_view(verdict: Verdict, diagnosis: Diagnosis, latency: Duration) -> VerdictView {
        let thresholds = Thresholds::for_video_calls();
        VerdictView {
            verdict,
            diagnosis,
            latency: Some(create_reading(latency, thresholds.latency)),
            jitter: Some(create_reading(Duration::from_millis(4), thresholds.jitter)),
            loss: Some(create_reading(LossRate::from_percent(2.5), thresholds.loss)),
            context: None,
        }
    }

    #[test]
    fn shows_a_healthy_solid_connection_without_action_or_detail() {
        // Arrange
        let view = create_view(
            Verdict::Graded(Grade::Solid),
            Diagnosis::Healthy,
            Duration::from_millis(20),
        );

        // Act
        let display = ConnectionDisplay::from_view(&view, &TextCatalog::load());

        // Assert
        assert_eq!(display.state, DisplayState::Solid);
        assert_eq!(display.state_label, "Solide");
        assert_eq!(display.cause, "Bonne connexion pour la visio.");
        assert_eq!((display.action, display.detail), (None, None));
    }

    #[test]
    fn names_the_metric_out_of_threshold_with_its_value_and_threshold() {
        // Arrange
        let view = create_view(
            Verdict::Graded(Grade::Unstable),
            Diagnosis::OutOfThreshold(Metric::Latency),
            Duration::from_millis(180),
        );

        // Act
        let display = ConnectionDisplay::from_view(&view, &TextCatalog::load());

        // Assert
        assert_eq!(display.state_label, "Instable");
        assert_eq!(
            display.detail.as_deref(),
            Some(format!("Latence 180{NBSP}ms, seuil 100{NBSP}ms").as_str())
        );
    }

    #[test]
    fn shows_the_warm_up_progress_while_measuring() {
        // Arrange
        let mut view = create_view(
            Verdict::Measuring,
            Diagnosis::Measuring {
                collected: 6,
                needed: 10,
            },
            Duration::ZERO,
        );
        view.latency = None;

        // Act
        let display = ConnectionDisplay::from_view(&view, &TextCatalog::load());

        // Assert
        assert_eq!(display.state, DisplayState::Measuring);
        assert_eq!(display.detail.as_deref(), Some("6 sur 10"));
        assert_eq!(display.metrics[0].value, None);
        assert_eq!(display.metrics[0].gauge, None);
    }

    #[test]
    fn suggests_a_check_when_no_server_answers() {
        // Arrange
        let view = VerdictView {
            verdict: Verdict::Offline,
            diagnosis: Diagnosis::NoServerReachable,
            latency: None,
            jitter: None,
            loss: None,
            context: None,
        };

        // Act
        let display = ConnectionDisplay::from_view(&view, &TextCatalog::load());

        // Assert
        assert_eq!(display.state_label, "Hors ligne");
        assert_eq!(display.cause, "Aucun serveur ne répond.");
        assert_eq!(
            display.action.as_deref(),
            Some("Vérifier le Wi-Fi ou le câble.")
        );
    }

    #[test]
    fn places_the_value_and_the_zones_on_each_gauge() {
        // Arrange
        let view = create_view(
            Verdict::Graded(Grade::Unstable),
            Diagnosis::OutOfThreshold(Metric::Latency),
            Duration::from_millis(180),
        );

        // Act
        let display = ConnectionDisplay::from_view(&view, &TextCatalog::load());

        // Assert
        let latency = &display.metrics[0];
        assert_eq!(
            latency.gauge,
            Some(Gauge {
                position: 0.45,
                unstable_from: 0.25,
                degraded_above: 0.75,
                label: format!("Solide sous 100{NBSP}ms, Dégradée au-delà de 300{NBSP}ms"),
            })
        );
    }

    #[test]
    fn hints_at_a_metric_close_to_its_next_threshold() {
        // Arrange
        let view = create_view(
            Verdict::Graded(Grade::Solid),
            Diagnosis::NearThreshold(Metric::Latency),
            Duration::from_millis(85),
        );

        // Act
        let display = ConnectionDisplay::from_view(&view, &TextCatalog::load());

        // Assert
        assert_eq!(display.cause, "Bonne connexion, proche de la limite.");
        assert_eq!(display.metrics[0].hint.as_deref(), Some("Proche du seuil"));
        assert_eq!(display.metrics[1].hint, None);
    }

    #[test]
    fn says_the_connection_is_recovering_without_figures() {
        // Arrange
        let view = create_view(
            Verdict::Graded(Grade::Degraded),
            Diagnosis::Recovering,
            Duration::from_millis(20),
        );

        // Act
        let display = ConnectionDisplay::from_view(&view, &TextCatalog::load());

        // Assert
        assert_eq!(display.state_label, "Dégradée");
        assert_eq!(
            display.cause,
            "La connexion s'améliore, confirmation en cours."
        );
        assert_eq!(display.detail, None);
    }

    #[test]
    fn hides_the_progress_once_the_warm_up_is_complete() {
        // Arrange
        let view = VerdictView {
            verdict: Verdict::Measuring,
            diagnosis: Diagnosis::Measuring {
                collected: 10,
                needed: 10,
            },
            latency: None,
            jitter: None,
            loss: None,
            context: None,
        };

        // Act
        let display = ConnectionDisplay::from_view(&view, &TextCatalog::load());

        // Assert
        assert_eq!(display.detail, None);
    }

    fn create_context(kind: ConnectionKind, is_vpn: bool) -> ConnectionContext {
        ConnectionContext {
            kind,
            is_vpn,
            gateway: None,
        }
    }

    #[test]
    fn names_the_network_and_how_it_is_reached() {
        // Arrange
        let catalog = TextCatalog::load();

        // Act
        let wifi =
            NetworkDisplay::from_context(&create_context(ConnectionKind::WiFi, false), &catalog);
        let hotspot = NetworkDisplay::from_context(
            &create_context(ConnectionKind::PhoneHotspot, false),
            &catalog,
        );

        // Assert
        assert_eq!(
            (wifi.label.as_str(), wifi.connection.as_str()),
            ("Réseau Wi-Fi", "Wi-Fi")
        );
        assert_eq!(hotspot.label, "Partage de connexion");
    }

    #[test]
    fn says_when_the_measure_goes_through_a_vpn() {
        // Arrange
        let catalog = TextCatalog::load();

        // Act
        let over_ethernet =
            NetworkDisplay::from_context(&create_context(ConnectionKind::Ethernet, true), &catalog);
        let hidden =
            NetworkDisplay::from_context(&create_context(ConnectionKind::Other, true), &catalog);

        // Assert
        assert_eq!(over_ethernet.connection, "Ethernet, via un VPN");
        assert_eq!(
            (hidden.label.as_str(), hidden.connection.as_str()),
            ("Réseau via un VPN", "VPN")
        );
    }

    #[test]
    fn shows_no_network_without_a_default_route() {
        // Act
        let display = ConnectionDisplay::create_initial(&TextCatalog::load());

        // Assert
        assert_eq!(display.network, None);
    }

    #[test]
    fn starts_measuring_with_no_sample_collected() {
        // Act
        let display = ConnectionDisplay::create_initial(&TextCatalog::load());

        // Assert
        assert_eq!(display.state, DisplayState::Measuring);
        assert_eq!(display.detail.as_deref(), Some("0 sur 10"));
    }

    #[test]
    fn writes_numbers_the_french_way() {
        // Arrange
        let view = create_view(
            Verdict::Graded(Grade::Unstable),
            Diagnosis::OutOfThreshold(Metric::Loss),
            Duration::from_millis(20),
        );

        // Act
        let display = ConnectionDisplay::from_view(&view, &TextCatalog::load());

        // Assert
        let names: Vec<&str> = display
            .metrics
            .iter()
            .map(|metric| metric.name.as_str())
            .collect();
        assert_eq!(names, ["Latence", "Gigue", "Perte"]);
        assert_eq!(
            display.metrics[2].value.as_deref(),
            Some(format!("2,5{NBSP}%").as_str())
        );
        assert_eq!(display.metrics[0].method, "p90, 15\u{a0}s");
        assert_eq!(
            display.detail.as_deref(),
            Some(format!("Perte 2,5{NBSP}%, seuil 1{NBSP}%").as_str())
        );
    }
}
