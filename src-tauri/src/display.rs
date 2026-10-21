//! Turns each verdict into the ready-to-show object the front end renders: states, texts, gauges.

use std::time::Duration;

use application::{MetricReading, VerdictView};
use domain::{
    ConnectionKind, Diagnosis, Fault, Grade, LocalProblem, LocalSegment, LossRate, MeasuringCause,
    Metric, Unmeasurable, Verdict, WifiBand,
};
use fluent_bundle::FluentArgs;
use serde::Serialize;
use specta::Type;

use crate::network_display::NetworkDisplay;
use crate::path_display::PathDisplay;
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
    /// `None` without a default route.
    pub path: Option<PathDisplay>,
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
                cause: MeasuringCause::FirstMeasures,
            },
            latency: None,
            jitter: None,
            loss: None,
            context: None,
            local: LocalSegment::Unmeasurable(Unmeasurable::NoGateway),
            wifi: None,
        };
        Self::from_view(&view, catalog)
    }

    /// O(1): three metrics and a handful of short messages.
    #[must_use]
    pub fn from_view(view: &VerdictView, catalog: &TextCatalog) -> Self {
        let state = DisplayState::from(view.verdict);
        let wifi_band = view
            .wifi
            .as_ref()
            .filter(|_| {
                view.context
                    .is_some_and(|context| context.is_carried_by_wifi())
            })
            .map(|link| link.band);
        let (cause_key, action_key) = select_cause_keys(
            view.diagnosis,
            ConnectionKind::from_context(view.context),
            wifi_band,
        );
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
            path: PathDisplay::from_view(view, catalog),
            network: view
                .context
                .map(|context| NetworkDisplay::from_context(&context, view.wifi.as_ref(), catalog)),
        }
    }
}

/// A value that can be printed with its unit and placed on a gauge.
pub trait Displayable: domain::Scalable {
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
        Diagnosis::Measuring {
            collected, needed, ..
        } if collected < needed => {
            args.set("collected", collected.to_string());
            args.set("needed", needed.to_string());
            Some(catalog.render("detail-measuring", Some(&args)))
        }
        Diagnosis::NearThreshold(metric) | Diagnosis::OutOfThreshold { metric, .. } => {
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
        | Diagnosis::NoNetwork
        | Diagnosis::GatewayWithoutInternet
        | Diagnosis::NoServerReachable => None,
    }
}

/// The catalogue keys of the cause and its action. The link is named by what the user has: a
/// Wi-Fi, a phone, a cable. O(1).
const fn select_cause_keys(
    diagnosis: Diagnosis,
    kind: ConnectionKind,
    wifi_band: Option<WifiBand>,
) -> (&'static str, Option<&'static str>) {
    match diagnosis {
        Diagnosis::Healthy => ("cause-healthy", None),
        Diagnosis::NearThreshold(_) => ("cause-near-threshold", None),
        Diagnosis::OutOfThreshold { fault, .. } => select_fault_keys(fault, kind, wifi_band),
        Diagnosis::Recovering => ("cause-recovering", None),
        Diagnosis::Measuring {
            cause: MeasuringCause::FirstMeasures,
            ..
        } => ("cause-measuring", None),
        Diagnosis::Measuring {
            cause: MeasuringCause::NewNetwork,
            ..
        } => ("cause-new-network", None),
        Diagnosis::NoNetwork => ("cause-no-network", Some("action-no-network")),
        Diagnosis::GatewayWithoutInternet if matches!(kind, ConnectionKind::PhoneHotspot) => (
            "cause-phone-without-internet",
            Some("action-phone-without-internet"),
        ),
        Diagnosis::GatewayWithoutInternet => (
            "cause-box-without-internet",
            Some("action-box-without-internet"),
        ),
        Diagnosis::NoServerReachable => ("cause-no-server", Some("action-check-connection")),
    }
}

/// Moving to 5 GHz is advice only for a link that is still on 2.4 GHz, or of unknown band.
const fn select_fault_keys(
    fault: Fault,
    kind: ConnectionKind,
    wifi_band: Option<WifiBand>,
) -> (&'static str, Option<&'static str>) {
    match (fault, kind) {
        (Fault::LocalLink(_), ConnectionKind::PhoneHotspot) => {
            ("cause-phone-link", Some("action-phone-link"))
        }
        (Fault::LocalLink(LocalProblem::WeakSignal), _) => match wifi_band {
            Some(WifiBand::FiveGhz | WifiBand::SixGhz) => {
                ("cause-wifi-weak", Some("action-wifi-weak-closer"))
            }
            _ => ("cause-wifi-weak", Some("action-wifi-weak")),
        },
        (Fault::LocalLink(LocalProblem::Loss), ConnectionKind::WiFi) => {
            ("cause-wifi-losses", Some("action-wifi-losses"))
        }
        (Fault::LocalLink(LocalProblem::Latency), ConnectionKind::WiFi) => {
            ("cause-wifi-slow", Some("action-wifi-slow"))
        }
        (Fault::LocalLink(_), _) => ("cause-local-link", Some("action-local-link")),
        (Fault::Operator, ConnectionKind::WiFi) => ("cause-operator", Some("action-operator")),
        (Fault::Operator, ConnectionKind::Ethernet) => {
            ("cause-operator-cable", Some("action-operator"))
        }
        (Fault::Operator, _) => ("cause-operator-local", Some("action-operator")),
        (Fault::MobileNetwork, ConnectionKind::PhoneHotspot) => {
            ("cause-mobile-network", Some("action-mobile-network"))
        }
        (Fault::MobileNetwork, _) => ("cause-mobile-network-modem", Some("action-mobile-network")),
        (Fault::Undetermined(Unmeasurable::NotPermitted), _) => {
            ("cause-undetermined-system", Some("action-allow-ping"))
        }
        (Fault::Undetermined(_), _) => ("cause-undetermined-network", None),
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
    use domain::{
        ConnectionContext, Diagnosis, Fault, Grade, GradeBounds, LocalSegment, LossRate, Medium,
        Metering, Metric, Scalable, Thresholds, Unmeasurable, Verdict,
    };

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
            local: LocalSegment::Unmeasurable(Unmeasurable::NoGateway),
            wifi: None,
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
            Diagnosis::OutOfThreshold {
                metric: Metric::Latency,
                fault: Fault::Operator,
            },
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
                cause: MeasuringCause::FirstMeasures,
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
            local: LocalSegment::Unmeasurable(Unmeasurable::NoGateway),
            wifi: None,
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
            Diagnosis::OutOfThreshold {
                metric: Metric::Latency,
                fault: Fault::Operator,
            },
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
                cause: MeasuringCause::FirstMeasures,
            },
            latency: None,
            jitter: None,
            loss: None,
            context: None,
            local: LocalSegment::Unmeasurable(Unmeasurable::NoGateway),
            wifi: None,
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
            gateway_address: None,
            metering: Metering::Unmetered,
            medium: if kind == ConnectionKind::WiFi {
                Medium::WiFi
            } else {
                Medium::Ethernet
            },
        }
    }

    #[test]
    fn says_when_data_counts_on_the_connection() {
        // Arrange
        let catalog = TextCatalog::load();
        let metered = ConnectionContext {
            metering: Metering::Metered,
            ..create_context(ConnectionKind::WiFi, true)
        };

        // Act
        let network = NetworkDisplay::from_context(&metered, None, &catalog);

        // Assert
        assert_eq!(network.connection, "Wi-Fi, via un VPN, connexion limitée");
    }

    #[test]
    fn says_a_new_network_is_being_measured() {
        // Arrange
        let view = VerdictView {
            verdict: Verdict::Measuring,
            diagnosis: Diagnosis::Measuring {
                collected: 2,
                needed: 10,
                cause: MeasuringCause::NewNetwork,
            },
            latency: None,
            jitter: None,
            loss: None,
            context: None,
            local: LocalSegment::Unmeasurable(Unmeasurable::NoGateway),
            wifi: None,
        };

        // Act
        let display = ConnectionDisplay::from_view(&view, &TextCatalog::load());

        // Assert
        assert_eq!(display.cause, "Nouveau réseau, premières mesures en cours.");
        assert_eq!(display.detail.as_deref(), Some("2 sur 10"));
    }

    #[test]
    fn names_the_network_and_how_it_is_reached() {
        // Arrange
        let catalog = TextCatalog::load();

        // Act
        let wifi = NetworkDisplay::from_context(
            &create_context(ConnectionKind::WiFi, false),
            None,
            &catalog,
        );
        let hotspot = NetworkDisplay::from_context(
            &create_context(ConnectionKind::PhoneHotspot, false),
            None,
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
        let over_ethernet = NetworkDisplay::from_context(
            &create_context(ConnectionKind::Ethernet, true),
            None,
            &catalog,
        );
        let hidden = NetworkDisplay::from_context(
            &create_context(ConnectionKind::Other, true),
            None,
            &catalog,
        );

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
            Diagnosis::OutOfThreshold {
                metric: Metric::Loss,
                fault: Fault::Operator,
            },
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

    fn create_offline_view(diagnosis: Diagnosis, kind: ConnectionKind) -> VerdictView {
        VerdictView {
            verdict: Verdict::Offline,
            diagnosis,
            latency: None,
            jitter: None,
            loss: None,
            context: Some(create_context(kind, false)),
            local: LocalSegment::Unmeasurable(Unmeasurable::NoAnswer),
            wifi: None,
        }
    }

    fn create_fault_view(fault: Fault, kind: ConnectionKind) -> VerdictView {
        let mut view = create_view(
            Verdict::Graded(Grade::Unstable),
            Diagnosis::OutOfThreshold {
                metric: Metric::Latency,
                fault,
            },
            Duration::from_millis(180),
        );
        view.context = Some(create_context(kind, false));
        view
    }

    #[test]
    fn names_the_segment_at_fault_in_the_cause() {
        // Arrange
        let catalog = TextCatalog::load();

        // Act
        let operator = ConnectionDisplay::from_view(
            &create_fault_view(Fault::Operator, ConnectionKind::WiFi),
            &catalog,
        );
        let wifi = ConnectionDisplay::from_view(
            &create_fault_view(Fault::LocalLink(LocalProblem::Loss), ConnectionKind::WiFi),
            &catalog,
        );
        let phone = ConnectionDisplay::from_view(
            &create_fault_view(Fault::MobileNetwork, ConnectionKind::PhoneHotspot),
            &catalog,
        );

        // Assert
        assert_eq!(
            operator.cause,
            "L'opérateur est en cause, le Wi-Fi va bien."
        );
        assert_eq!(
            wifi.cause,
            format!("Le Wi-Fi est en cause{NBSP}: des paquets se perdent avant la box.")
        );
        assert_eq!(
            phone.cause,
            "Le réseau mobile est en cause, le lien avec le téléphone va bien."
        );
    }

    #[test]
    fn suggests_allowing_the_ping_when_the_system_refuses_it() {
        // Act
        let display = ConnectionDisplay::from_view(
            &create_fault_view(
                Fault::Undetermined(Unmeasurable::NotPermitted),
                ConnectionKind::WiFi,
            ),
            &TextCatalog::load(),
        );

        // Assert
        assert_eq!(
            display.cause,
            "La cause exacte ne peut pas être déterminée sur ce système."
        );
        assert_eq!(
            display.action.as_deref(),
            Some("Autoriser le ping (net.ipv4.ping_group_range).")
        );
    }

    #[test]
    fn tells_a_box_without_internet_from_no_network_at_all() {
        // Arrange
        let catalog = TextCatalog::load();

        // Act
        let box_down = ConnectionDisplay::from_view(
            &create_offline_view(Diagnosis::GatewayWithoutInternet, ConnectionKind::WiFi),
            &catalog,
        );
        let mut no_route = create_offline_view(Diagnosis::NoNetwork, ConnectionKind::WiFi);
        no_route.context = None;
        let no_network = ConnectionDisplay::from_view(&no_route, &catalog);

        // Assert
        assert_eq!(
            box_down.cause,
            "La box répond, mais Internet ne répond plus."
        );
        assert_eq!(no_network.cause, "Aucun réseau connecté.");
        assert_eq!(no_network.path, None);
    }

    #[test]
    fn names_the_link_that_works_in_the_operator_and_mobile_causes() {
        // Arrange
        let catalog = TextCatalog::load();

        // Act
        let cable = ConnectionDisplay::from_view(
            &create_fault_view(Fault::Operator, ConnectionKind::Ethernet),
            &catalog,
        );
        let modem = ConnectionDisplay::from_view(
            &create_fault_view(Fault::MobileNetwork, ConnectionKind::Cellular),
            &catalog,
        );

        // Assert
        assert_eq!(cable.cause, "L'opérateur est en cause, le câble va bien.");
        assert_eq!(modem.cause, "Le réseau mobile est en cause.");
    }

    #[test]
    fn names_a_weak_wifi_signal_as_the_cause() {
        // Act
        let display = ConnectionDisplay::from_view(
            &create_fault_view(
                Fault::LocalLink(LocalProblem::WeakSignal),
                ConnectionKind::WiFi,
            ),
            &TextCatalog::load(),
        );

        // Assert
        assert_eq!(
            display.cause,
            format!("Le Wi-Fi est en cause{NBSP}: signal faible.")
        );
        assert_eq!(
            display.action,
            Some(format!("Se rapprocher de la box ou passer en 5{NBSP}GHz."))
        );
    }

    #[test]
    fn advises_moving_to_5_ghz_only_on_a_2_4_ghz_link() {
        // Arrange
        let catalog = TextCatalog::load();
        let mut view = create_fault_view(
            Fault::LocalLink(LocalProblem::WeakSignal),
            ConnectionKind::WiFi,
        );
        view.wifi = Some(domain::WifiLink {
            name: None,
            signal_dbm: Some(-78),
            rate_mbps: None,
            channel: Some(36),
            band: WifiBand::FiveGhz,
        });

        // Act
        let display = ConnectionDisplay::from_view(&view, &catalog);

        // Assert
        assert_eq!(display.action.as_deref(), Some("Se rapprocher de la box."));
    }
}
