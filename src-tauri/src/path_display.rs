//! The path from the computer to the Internet through its gateway, with the segment at fault
//! named by a word, never by colour alone.

use application::VerdictView;
use domain::{ConnectionKind, Diagnosis, Fault, LocalSegment, Verdict};
use fluent_bundle::FluentArgs;
use serde::Serialize;
use specta::Type;

use crate::display::{DisplayState, Displayable};
use crate::text_catalog::TextCatalog;

const COMPUTER_KEY: &str = if cfg!(target_os = "macos") {
    "path-this-mac"
} else {
    "path-this-computer"
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct SegmentDisplay {
    /// The link and its latency, e.g. "Wi-Fi, 4 ms".
    pub label: String,
    /// "en cause" or "coupé" on the segment where the problem lies.
    pub fault: Option<String>,
    /// `None` while there is nothing to grade: measuring, or not measurable.
    pub state: Option<DisplayState>,
    /// Drawn as broken: the box answers, the Internet does not.
    pub is_cut: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct PathDisplay {
    pub computer: String,
    pub gateway: String,
    pub internet: String,
    pub local: SegmentDisplay,
    pub internet_segment: SegmentDisplay,
    /// The whole path in one sentence, for screen readers.
    pub description: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Segment {
    Local,
    Internet,
}

impl PathDisplay {
    /// `None` without a default route: there is no path to draw. O(1).
    #[must_use]
    pub fn from_view(view: &VerdictView, catalog: &TextCatalog) -> Option<Self> {
        let context = view.context?;
        let state = DisplayState::from(view.verdict);
        let at_fault = locate_fault(view.diagnosis);
        let is_measuring = view.verdict == Verdict::Measuring;
        let is_cut = view.diagnosis == Diagnosis::GatewayWithoutInternet;
        let local_link = catalog.render(select_link_key(context.kind), None);
        let internet_link = catalog.render(select_internet_key(context.kind), None);
        let local = SegmentDisplay {
            label: compose_label(
                &local_link,
                match view.local {
                    LocalSegment::Measured {
                        latency: Some(latency),
                        ..
                    } => latency.format(catalog),
                    LocalSegment::Measured { latency: None, .. } => {
                        catalog.render("path-no-value", None)
                    }
                    LocalSegment::Unmeasurable(_) => catalog.render("path-unmeasurable", None),
                },
                catalog,
            ),
            fault: (at_fault == Some(Segment::Local))
                .then(|| catalog.render("path-at-fault", None)),
            state: match view.local {
                _ if is_measuring => None,
                LocalSegment::Unmeasurable(_) => None,
                LocalSegment::Measured { .. } if at_fault == Some(Segment::Local) => Some(state),
                LocalSegment::Measured { .. } => Some(DisplayState::Solid),
            },
            is_cut: false,
        };
        let internet_segment = SegmentDisplay {
            label: compose_label(
                &internet_link,
                view.latency.map_or_else(
                    || {
                        let key = if view.verdict == Verdict::Offline {
                            "path-no-answer"
                        } else {
                            "path-no-value"
                        };
                        catalog.render(key, None)
                    },
                    |reading| reading.value.format(catalog),
                ),
                catalog,
            ),
            fault: match at_fault {
                Some(Segment::Internet) if is_cut => Some(catalog.render("path-cut", None)),
                Some(Segment::Internet) => Some(catalog.render("path-at-fault", None)),
                _ => None,
            },
            state: (!is_measuring).then_some(state),
            is_cut,
        };
        let gateway = catalog.render(select_gateway_key(context.kind), None);
        let computer = catalog.render(COMPUTER_KEY, None);
        let internet = catalog.render("path-internet", None);
        let description = describe(
            [&computer, &gateway, &internet],
            [&local.label, &internet_segment.label],
            at_fault.map(|segment| match segment {
                Segment::Local => (&local_link, &local.fault),
                Segment::Internet => (&internet_link, &internet_segment.fault),
            }),
            catalog,
        );
        Some(Self {
            computer,
            gateway,
            internet,
            local,
            internet_segment,
            description,
        })
    }
}

const fn locate_fault(diagnosis: Diagnosis) -> Option<Segment> {
    match diagnosis {
        Diagnosis::OutOfThreshold {
            fault: Fault::LocalLink(_),
            ..
        } => Some(Segment::Local),
        Diagnosis::OutOfThreshold {
            fault: Fault::Operator | Fault::MobileNetwork,
            ..
        }
        | Diagnosis::GatewayWithoutInternet => Some(Segment::Internet),
        _ => None,
    }
}

const fn select_gateway_key(kind: ConnectionKind) -> &'static str {
    match kind {
        ConnectionKind::PhoneHotspot => "path-phone",
        ConnectionKind::Cellular => "path-modem",
        ConnectionKind::Ethernet | ConnectionKind::WiFi | ConnectionKind::Other => "path-box",
    }
}

const fn select_link_key(kind: ConnectionKind) -> &'static str {
    match kind {
        ConnectionKind::WiFi => "link-wifi",
        ConnectionKind::Ethernet => "link-cable",
        ConnectionKind::PhoneHotspot | ConnectionKind::Cellular | ConnectionKind::Other => {
            "link-local"
        }
    }
}

const fn select_internet_key(kind: ConnectionKind) -> &'static str {
    match kind {
        ConnectionKind::PhoneHotspot | ConnectionKind::Cellular => "link-mobile-network",
        ConnectionKind::Ethernet | ConnectionKind::WiFi | ConnectionKind::Other => "link-operator",
    }
}

fn compose_label(link: &str, value: String, catalog: &TextCatalog) -> String {
    let mut args = FluentArgs::new();
    args.set("link", link.to_owned());
    args.set("value", value);
    catalog.render("path-segment", Some(&args))
}

fn describe(
    [computer, gateway, internet]: [&String; 3],
    [local, remote]: [&String; 2],
    fault: Option<(&String, &Option<String>)>,
    catalog: &TextCatalog,
) -> String {
    let mut args = FluentArgs::new();
    args.set("computer", computer.clone());
    args.set("gateway", gateway.clone());
    args.set("internet", internet.clone());
    args.set("local", local.clone());
    args.set("remote", remote.clone());
    let path = catalog.render("path-description", Some(&args));
    let Some((link, Some(word))) = fault else {
        return path;
    };
    let mut args = FluentArgs::new();
    args.set("link", link.clone());
    args.set("fault", word.clone());
    format!(
        "{path} {}",
        catalog.render("path-description-fault", Some(&args))
    )
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use application::MetricReading;
    use domain::{ConnectionContext, Grade, Medium, Metering, Metric, Thresholds, Unmeasurable};

    use super::*;

    const NBSP: char = '\u{a0}';

    fn create_view(diagnosis: Diagnosis, verdict: Verdict, local: LocalSegment) -> VerdictView {
        let bounds = Thresholds::VIDEO_CALLS.latency;
        let latency = Duration::from_millis(180);
        VerdictView {
            verdict,
            diagnosis,
            latency: Some(MetricReading {
                value: latency,
                grade: bounds.compute_grade(latency),
                is_near_next_threshold: false,
                bounds,
            }),
            jitter: None,
            loss: None,
            context: Some(ConnectionContext {
                kind: ConnectionKind::WiFi,
                is_vpn: false,
                gateway: None,
                gateway_address: None,
                metering: Metering::Unmetered,
                medium: Medium::WiFi,
            }),
            local,
            wifi: None,
        }
    }

    const GOOD_LOCAL: LocalSegment = LocalSegment::Measured {
        latency: Some(Duration::from_millis(4)),
        loss: None,
        lost: 0,
    };

    fn render_computer_label() -> String {
        TextCatalog::load().render(COMPUTER_KEY, None)
    }

    #[test]
    fn marks_the_operator_segment_with_a_word() {
        // Arrange
        let view = create_view(
            Diagnosis::OutOfThreshold {
                metric: Metric::Latency,
                fault: Fault::Operator,
            },
            Verdict::Graded(Grade::Unstable),
            GOOD_LOCAL,
        );

        // Act
        let path = PathDisplay::from_view(&view, &TextCatalog::load()).expect("a path");

        // Assert
        assert_eq!(path.local.label, format!("Wi-Fi, 4{NBSP}ms"));
        assert_eq!(path.local.fault, None);
        assert_eq!(path.local.state, Some(DisplayState::Solid));
        assert_eq!(
            path.internet_segment.label,
            format!("Opérateur, 180{NBSP}ms")
        );
        assert_eq!(path.internet_segment.fault.as_deref(), Some("en cause"));
        assert_eq!(
            path.description,
            format!(
                "Chemin{NBSP}: Ce Mac, Box, Internet. Wi-Fi, 4{NBSP}ms, puis Opérateur, 180{NBSP}ms. Tronçon Opérateur en cause."
            )
            .replace("Ce Mac", &render_computer_label())
        );
    }

    #[test]
    fn draws_the_internet_segment_cut_when_only_the_box_answers() {
        // Arrange
        let mut view = create_view(
            Diagnosis::GatewayWithoutInternet,
            Verdict::Offline,
            GOOD_LOCAL,
        );
        view.latency = None;

        // Act
        let path = PathDisplay::from_view(&view, &TextCatalog::load()).expect("a path");

        // Assert
        assert!(path.internet_segment.is_cut);
        assert_eq!(path.internet_segment.fault.as_deref(), Some("coupé"));
        assert_eq!(path.internet_segment.label, "Opérateur, aucune réponse");
    }

    #[test]
    fn leaves_an_unmeasurable_local_segment_ungraded() {
        // Arrange
        let view = create_view(
            Diagnosis::OutOfThreshold {
                metric: Metric::Latency,
                fault: Fault::Undetermined(Unmeasurable::NoAnswer),
            },
            Verdict::Graded(Grade::Unstable),
            LocalSegment::Unmeasurable(Unmeasurable::NoAnswer),
        );

        // Act
        let path = PathDisplay::from_view(&view, &TextCatalog::load()).expect("a path");

        // Assert
        assert_eq!(path.local.label, "Wi-Fi, non mesurable");
        assert_eq!(path.local.state, None);
        assert_eq!(path.internet_segment.fault, None);
    }
}
