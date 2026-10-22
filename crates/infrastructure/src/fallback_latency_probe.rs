//! ICMP first, TCP 443 when ICMP is refused or filtered, and TCP again to tell a lost echo from an outage.

use std::collections::HashMap;
use std::future::Future;
use std::sync::{Mutex, PoisonError};
use std::time::Duration;

use application::LatencyProbe;
use domain::{ProbeOutcome, Target};

/// Echoes lost in a row while TCP still connects, after which ICMP is considered filtered.
/// High enough that a lossy Wi-Fi (30 % loss) almost never reaches it by chance.
const FILTERED_AFTER_LOSSES: u32 = 10;

/// Result of one ICMP echo.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EchoResult {
    Answered(Duration),
    NoAnswer,
    /// The system refuses unprivileged ICMP (Linux without `ping_group_range`).
    NotPermitted,
}

pub trait EchoSender {
    fn send_echo(&self, target: Target) -> impl Future<Output = EchoResult> + Send;
}

/// Opens (and drops) a TCP connection; the time to connect, or `None` when nothing answered.
pub trait ConnectionOpener {
    fn open_connection(&self, target: Target) -> impl Future<Output = Option<Duration>> + Send;
}

/// How a target is measured; a target absent from the map is on ICMP.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Route {
    Tcp,
    /// Back from TCP, ICMP is tried again: TCP still gives the latency, so a filtered echo
    /// counts as no loss.
    Retrying,
}

/// Picks, per target, between ICMP and TCP, and classifies every probe for the engine.
pub struct FallbackLatencyProbe<E, O> {
    echo: E,
    tcp: O,
    routes: Mutex<HashMap<Target, Route>>,
    losses_in_a_row: Mutex<HashMap<Target, u32>>,
}

impl<E: EchoSender, O: ConnectionOpener> FallbackLatencyProbe<E, O> {
    #[must_use]
    pub fn new(echo: E, tcp: O) -> Self {
        Self {
            echo,
            tcp,
            routes: Mutex::default(),
            losses_in_a_row: Mutex::default(),
        }
    }

    /// `None` on ICMP. O(1).
    fn read_route(&self, target: Target) -> Option<Route> {
        self.routes
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .get(&target)
            .copied()
    }

    fn switch_to_tcp(&self, target: Target) {
        self.reset_losses(target);
        self.routes
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .insert(target, Route::Tcp);
    }

    fn switch_to_icmp(&self, target: Target) {
        self.reset_losses(target);
        self.routes
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .remove(&target);
    }

    /// Counts one more loss with TCP still up and returns the streak length. O(1).
    fn count_filtered_loss(&self, target: Target) -> u32 {
        let mut streaks = self
            .losses_in_a_row
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        let streak = streaks.entry(target).or_default();
        *streak += 1;
        *streak
    }

    fn reset_losses(&self, target: Target) {
        self.losses_in_a_row
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .remove(&target);
    }

    async fn send_tcp_probe(&self, target: Target) -> ProbeOutcome {
        self.tcp
            .open_connection(target)
            .await
            .map_or(ProbeOutcome::Unreachable, ProbeOutcome::Answered)
    }
}

impl<E: EchoSender + Sync, O: ConnectionOpener + Sync> LatencyProbe for FallbackLatencyProbe<E, O> {
    /// One echo, plus one TCP connection when the echo fails: at most two timeouts. O(1).
    async fn send_probe(&self, target: Target) -> ProbeOutcome {
        let route = self.read_route(target);
        if route == Some(Route::Tcp) {
            return self.send_tcp_probe(target).await;
        }
        match self.echo.send_echo(target).await {
            EchoResult::Answered(round_trip) => {
                self.switch_to_icmp(target);
                ProbeOutcome::Answered(round_trip)
            }
            EchoResult::NotPermitted => {
                self.switch_to_tcp(target);
                self.send_tcp_probe(target).await
            }
            EchoResult::NoAnswer => {
                let Some(round_trip) = self.tcp.open_connection(target).await else {
                    self.reset_losses(target);
                    return ProbeOutcome::Unreachable;
                };
                if self.count_filtered_loss(target) >= FILTERED_AFTER_LOSSES {
                    self.switch_to_tcp(target);
                }
                if route == Some(Route::Retrying) {
                    ProbeOutcome::Answered(round_trip)
                } else {
                    ProbeOutcome::LostButReachable
                }
            }
        }
    }

    /// O(t) for t targets.
    fn forget_fallbacks(&self) {
        for route in self
            .routes
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .values_mut()
        {
            *route = Route::Retrying;
        }
        self.losses_in_a_row
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clear();
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::time::Duration;

    use application::LatencyProbe;
    use domain::{ProbeOutcome, Target};

    use super::*;

    const TARGET: &str = "192.0.2.1";
    const ECHO_ROUND_TRIP: Duration = Duration::from_millis(18);
    const TCP_ROUND_TRIP: Duration = Duration::from_millis(25);

    /// Answers every echo with the result set by the test and counts the echoes sent.
    struct InMemoryEcho {
        result: Mutex<EchoResult>,
        sent: AtomicU32,
    }

    impl InMemoryEcho {
        fn new(result: EchoResult) -> Self {
            Self {
                result: Mutex::new(result),
                sent: AtomicU32::new(0),
            }
        }
    }

    impl EchoSender for InMemoryEcho {
        fn send_echo(&self, _target: Target) -> impl Future<Output = EchoResult> + Send {
            self.sent.fetch_add(1, Ordering::SeqCst);
            std::future::ready(*self.result.lock().expect("unpoisoned"))
        }
    }

    /// Accepts or refuses every TCP connection, as set by the test.
    struct InMemoryTcp(Option<Duration>);

    impl ConnectionOpener for InMemoryTcp {
        fn open_connection(
            &self,
            _target: Target,
        ) -> impl Future<Output = Option<Duration>> + Send {
            std::future::ready(self.0)
        }
    }

    #[tokio::test]
    async fn returns_the_echo_round_trip_when_icmp_answers() {
        // Arrange
        let probe = FallbackLatencyProbe::new(
            InMemoryEcho::new(EchoResult::Answered(ECHO_ROUND_TRIP)),
            InMemoryTcp(None),
        );

        // Act
        let outcome = probe.send_probe(Target::parse(TARGET)).await;

        // Assert
        assert_eq!(outcome, ProbeOutcome::Answered(ECHO_ROUND_TRIP));
    }

    #[tokio::test]
    async fn reports_a_lost_echo_when_tcp_still_connects() {
        // Arrange
        let probe = FallbackLatencyProbe::new(
            InMemoryEcho::new(EchoResult::NoAnswer),
            InMemoryTcp(Some(TCP_ROUND_TRIP)),
        );

        // Act
        let outcome = probe.send_probe(Target::parse(TARGET)).await;

        // Assert
        assert_eq!(outcome, ProbeOutcome::LostButReachable);
    }

    #[tokio::test]
    async fn reports_unreachable_when_neither_icmp_nor_tcp_gets_through() {
        // Arrange
        let probe =
            FallbackLatencyProbe::new(InMemoryEcho::new(EchoResult::NoAnswer), InMemoryTcp(None));

        // Act
        let outcome = probe.send_probe(Target::parse(TARGET)).await;

        // Assert
        assert_eq!(outcome, ProbeOutcome::Unreachable);
    }

    #[tokio::test]
    async fn measures_over_tcp_when_icmp_is_not_permitted() {
        // Arrange
        let echo = InMemoryEcho::new(EchoResult::NotPermitted);
        let probe = FallbackLatencyProbe::new(echo, InMemoryTcp(Some(TCP_ROUND_TRIP)));
        probe.send_probe(Target::parse(TARGET)).await;

        // Act
        let outcome = probe.send_probe(Target::parse(TARGET)).await;

        // Assert
        assert_eq!(outcome, ProbeOutcome::Answered(TCP_ROUND_TRIP));
        assert_eq!(
            probe.echo.sent.load(Ordering::SeqCst),
            1,
            "a refused ICMP is not retried"
        );
    }

    #[tokio::test]
    async fn tries_icmp_again_once_the_fallbacks_are_forgotten() {
        // Arrange
        let probe = FallbackLatencyProbe::new(
            InMemoryEcho::new(EchoResult::NotPermitted),
            InMemoryTcp(Some(TCP_ROUND_TRIP)),
        );
        probe.send_probe(Target::parse(TARGET)).await;
        *probe.echo.result.lock().expect("unpoisoned") = EchoResult::Answered(ECHO_ROUND_TRIP);

        // Act
        probe.forget_fallbacks();
        let outcome = probe.send_probe(Target::parse(TARGET)).await;

        // Assert
        assert_eq!(outcome, ProbeOutcome::Answered(ECHO_ROUND_TRIP));
    }

    #[tokio::test]
    async fn falls_back_to_tcp_at_once_when_icmp_is_still_refused() {
        // Arrange
        let probe = FallbackLatencyProbe::new(
            InMemoryEcho::new(EchoResult::NotPermitted),
            InMemoryTcp(Some(TCP_ROUND_TRIP)),
        );
        probe.send_probe(Target::parse(TARGET)).await;
        probe.forget_fallbacks();

        // Act
        let outcome = probe.send_probe(Target::parse(TARGET)).await;

        // Assert
        assert_eq!(outcome, ProbeOutcome::Answered(TCP_ROUND_TRIP));
        assert_eq!(probe.echo.sent.load(Ordering::SeqCst), 2);
    }

    /// A target that fell back to TCP after ten filtered echoes, then forgotten.
    async fn create_probe_retrying_a_filtered_target()
    -> FallbackLatencyProbe<InMemoryEcho, InMemoryTcp> {
        let probe = FallbackLatencyProbe::new(
            InMemoryEcho::new(EchoResult::NoAnswer),
            InMemoryTcp(Some(TCP_ROUND_TRIP)),
        );
        for _ in 0..FILTERED_AFTER_LOSSES {
            probe.send_probe(Target::parse(TARGET)).await;
        }
        probe.forget_fallbacks();
        probe
    }

    #[tokio::test]
    async fn counts_no_loss_while_retrying_a_filtered_target() {
        // Arrange
        let probe = create_probe_retrying_a_filtered_target().await;

        // Act
        let outcome = probe.send_probe(Target::parse(TARGET)).await;

        // Assert
        assert_eq!(outcome, ProbeOutcome::Answered(TCP_ROUND_TRIP));
        assert_eq!(
            probe.echo.sent.load(Ordering::SeqCst),
            FILTERED_AFTER_LOSSES + 1,
            "the echo was tried again"
        );
    }

    #[tokio::test]
    async fn gives_the_retry_up_after_ten_filtered_echoes() {
        // Arrange
        let probe = create_probe_retrying_a_filtered_target().await;
        for _ in 0..FILTERED_AFTER_LOSSES {
            probe.send_probe(Target::parse(TARGET)).await;
        }

        // Act
        let outcome = probe.send_probe(Target::parse(TARGET)).await;

        // Assert
        assert_eq!(outcome, ProbeOutcome::Answered(TCP_ROUND_TRIP));
        assert_eq!(
            probe.echo.sent.load(Ordering::SeqCst),
            2 * FILTERED_AFTER_LOSSES
        );
    }

    #[tokio::test]
    async fn counts_losses_again_once_the_retry_succeeds() {
        // Arrange
        let probe = create_probe_retrying_a_filtered_target().await;
        *probe.echo.result.lock().expect("unpoisoned") = EchoResult::Answered(ECHO_ROUND_TRIP);
        probe.send_probe(Target::parse(TARGET)).await;
        *probe.echo.result.lock().expect("unpoisoned") = EchoResult::NoAnswer;

        // Act
        let outcome = probe.send_probe(Target::parse(TARGET)).await;

        // Assert
        assert_eq!(outcome, ProbeOutcome::LostButReachable);
    }

    #[tokio::test]
    async fn counts_the_filtered_echoes_from_zero_once_forgotten() {
        // Arrange
        let probe = FallbackLatencyProbe::new(
            InMemoryEcho::new(EchoResult::NoAnswer),
            InMemoryTcp(Some(TCP_ROUND_TRIP)),
        );
        for _ in 1..FILTERED_AFTER_LOSSES {
            probe.send_probe(Target::parse(TARGET)).await;
        }
        probe.forget_fallbacks();
        probe.send_probe(Target::parse(TARGET)).await;

        // Act
        let outcome = probe.send_probe(Target::parse(TARGET)).await;

        // Assert
        assert_eq!(outcome, ProbeOutcome::LostButReachable, "still on ICMP");
        assert_eq!(
            probe.echo.sent.load(Ordering::SeqCst),
            FILTERED_AFTER_LOSSES + 1
        );
    }

    #[tokio::test]
    async fn switches_to_tcp_after_ten_echoes_filtered_in_a_row() {
        // Arrange
        let probe = FallbackLatencyProbe::new(
            InMemoryEcho::new(EchoResult::NoAnswer),
            InMemoryTcp(Some(TCP_ROUND_TRIP)),
        );
        for _ in 0..FILTERED_AFTER_LOSSES {
            probe.send_probe(Target::parse(TARGET)).await;
        }

        // Act
        let outcome = probe.send_probe(Target::parse(TARGET)).await;

        // Assert
        assert_eq!(outcome, ProbeOutcome::Answered(TCP_ROUND_TRIP));
        assert_eq!(
            probe.echo.sent.load(Ordering::SeqCst),
            FILTERED_AFTER_LOSSES
        );
    }

    #[tokio::test]
    async fn keeps_icmp_when_an_echo_answers_between_losses() {
        // Arrange
        let probe = FallbackLatencyProbe::new(
            InMemoryEcho::new(EchoResult::NoAnswer),
            InMemoryTcp(Some(TCP_ROUND_TRIP)),
        );
        for _ in 1..FILTERED_AFTER_LOSSES {
            probe.send_probe(Target::parse(TARGET)).await;
        }
        *probe.echo.result.lock().expect("unpoisoned") = EchoResult::Answered(ECHO_ROUND_TRIP);
        probe.send_probe(Target::parse(TARGET)).await;
        *probe.echo.result.lock().expect("unpoisoned") = EchoResult::NoAnswer;
        for _ in 1..FILTERED_AFTER_LOSSES {
            probe.send_probe(Target::parse(TARGET)).await;
        }

        // Act
        let outcome = probe.send_probe(Target::parse(TARGET)).await;

        // Assert
        assert_eq!(outcome, ProbeOutcome::LostButReachable);
    }
}
