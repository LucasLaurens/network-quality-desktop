//! Turns each verdict into the ready-to-show object the front end renders: states, texts, gauges.

use std::time::Duration;

use application::{MetricReading, VerdictView};
use domain::{Diagnosis, Grade, LossRate, Metric, Verdict};
use fluent_bundle::FluentArgs;
use serde::Serialize;
use specta::Type;

use crate::text_catalog::TextCatalog;

/// Right end of each gauge: values beyond it pin the marker to the end.
const LATENCY_GAUGE_MAX: Duration = Duration::from_millis(400);
const JITTER_GAUGE_MAX: Duration = Duration::from_millis(70);
const LOSS_GAUGE_MAX_PERCENT: f64 = 8.0;

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

/// Positions on a gauge, as fractions of its length (0 = left end, 1 = right end).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Gauge {
    pub position: f64,
    pub unstable_from: f64,
    pub degraded_above: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct MetricDisplay {
    pub name: String,
    pub method: String,
    /// `None` while the metric cannot be computed yet.
    pub value: Option<String>,
    pub state: Option<DisplayState>,
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
            Diagnosis::Measuring { .. } => ("cause-measuring", None),
            Diagnosis::NoNetwork => ("cause-no-network", Some("action-no-network")),
        };
        Self {
            state,
            state_label: catalog.render(state.to_catalogue_key(), None),
            cause: catalog.render(cause_key, None),
            action: action_key.map(|key| catalog.render(key, None)),
            detail: compose_detail(view, catalog),
            metrics: vec![
                compose_metric(Metric::Latency, view.latency.as_ref(), catalog),
                compose_metric(Metric::Jitter, view.jitter.as_ref(), catalog),
                compose_metric(Metric::Loss, view.loss.as_ref(), catalog),
            ],
        }
    }
}

/// A value that can be printed with its unit and placed on a gauge.
trait Displayable: domain::Scalable {
    fn format(self, catalog: &TextCatalog) -> String;
    fn compute_fraction(self, metric: Metric) -> f64;
}

impl Displayable for Duration {
    fn format(self, catalog: &TextCatalog) -> String {
        let mut args = FluentArgs::new();
        args.set("value", self.as_millis().to_string());
        catalog.render("unit-milliseconds", Some(&args))
    }

    fn compute_fraction(self, metric: Metric) -> f64 {
        let max = if metric == Metric::Jitter {
            JITTER_GAUGE_MAX
        } else {
            LATENCY_GAUGE_MAX
        };
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

    fn compute_fraction(self, _metric: Metric) -> f64 {
        (self.to_percent() / LOSS_GAUGE_MAX_PERCENT).min(1.0)
    }
}

fn compose_metric<T: Displayable>(
    metric: Metric,
    reading: Option<&MetricReading<T>>,
    catalog: &TextCatalog,
) -> MetricDisplay {
    let (name_key, method_key) = match metric {
        Metric::Latency => ("metric-latency", "method-latency"),
        Metric::Jitter => ("metric-jitter", "method-jitter"),
        Metric::Loss => ("metric-loss", "method-loss"),
    };
    MetricDisplay {
        name: catalog.render(name_key, None),
        method: catalog.render(method_key, None),
        value: reading.map(|reading| reading.value.format(catalog)),
        state: reading.map(|reading| reading.grade.into()),
        gauge: reading.map(|reading| Gauge {
            position: reading.value.compute_fraction(metric),
            unstable_from: reading.bounds.read_unstable_from().compute_fraction(metric),
            degraded_above: reading
                .bounds
                .read_degraded_above()
                .compute_fraction(metric),
        }),
        hint: reading
            .filter(|reading| reading.is_near_next_threshold)
            .map(|_| catalog.render("hint-near-threshold", None)),
    }
}

fn compose_detail(view: &VerdictView, catalog: &TextCatalog) -> Option<String> {
    let mut args = FluentArgs::new();
    match view.diagnosis {
        Diagnosis::Measuring { collected, needed } => {
            args.set("collected", collected.to_string());
            args.set("needed", needed.to_string());
            Some(catalog.render("detail-measuring", Some(&args)))
        }
        Diagnosis::NearThreshold(metric) | Diagnosis::OutOfThreshold(metric) => {
            let (name_key, value, threshold) = match metric {
                Metric::Latency => {
                    describe_reading("metric-latency", view.latency.as_ref(), catalog)?
                }
                Metric::Jitter => describe_reading("metric-jitter", view.jitter.as_ref(), catalog)?,
                Metric::Loss => describe_reading("metric-loss", view.loss.as_ref(), catalog)?,
            };
            args.set("metric", catalog.render(name_key, None));
            args.set("value", value);
            args.set("threshold", threshold);
            Some(catalog.render("detail-metric", Some(&args)))
        }
        Diagnosis::Healthy | Diagnosis::NoNetwork => None,
    }
}

fn describe_reading<T: Displayable>(
    name_key: &'static str,
    reading: Option<&MetricReading<T>>,
    catalog: &TextCatalog,
) -> Option<(&'static str, String, String)> {
    let reading = reading?;
    let threshold = reading.bounds.find_reference_threshold(reading.value);
    Some((
        name_key,
        reading.value.format(catalog),
        threshold.format(catalog),
    ))
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
    fn suggests_an_action_when_there_is_no_network() {
        // Arrange
        let view = VerdictView {
            verdict: Verdict::Offline,
            diagnosis: Diagnosis::NoNetwork,
            latency: None,
            jitter: None,
            loss: None,
        };

        // Act
        let display = ConnectionDisplay::from_view(&view, &TextCatalog::load());

        // Assert
        assert_eq!(display.state_label, "Hors ligne");
        assert_eq!(display.cause, "Aucun réseau connecté.");
        assert_eq!(
            display.action.as_deref(),
            Some("Se connecter à un Wi-Fi ou brancher un câble.")
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
                degraded_above: 0.75
            })
        );
        assert_eq!(latency.state, Some(DisplayState::Unstable));
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
