use std::io::ErrorKind;
use std::time::Duration;

use domain::Target;
use ping_async::{IcmpEchoRequestor, IcmpEchoStatus};

use crate::{EchoResult, EchoSender};

/// A slow answer is still a latency, not a loss; a following TCP check may then overrun the tick,
/// which only delays the next one (`MissedTickBehavior::Delay`).
const ECHO_TIMEOUT: Duration = Duration::from_millis(900);

/// Unprivileged ICMP echo: DGRAM sockets on macOS and Linux, `IcmpSendEcho2Ex` on Windows.
#[derive(Debug, Clone, Copy, Default)]
pub struct IcmpEchoSender;

impl EchoSender for IcmpEchoSender {
    async fn send_echo(&self, target: Target) -> EchoResult {
        // shortcut: one socket per echo (two per second); cache them per target if the cost shows.
        let requestor =
            match IcmpEchoRequestor::new(target.get_address(), None, None, Some(ECHO_TIMEOUT)) {
                Ok(requestor) => requestor,
                Err(error) if error.kind() == ErrorKind::PermissionDenied => {
                    return EchoResult::NotPermitted;
                }
                Err(_) => return EchoResult::NoAnswer,
            };
        match requestor.send().await {
            Ok(reply) if reply.status() == IcmpEchoStatus::Success => {
                EchoResult::Answered(reply.round_trip_time())
            }
            Ok(_) | Err(_) => EchoResult::NoAnswer,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Needs network access and ICMP rights, so it only runs on demand: `cargo test -- --ignored`.
    #[tokio::test]
    #[ignore = "real network"]
    async fn gets_an_echo_back_from_a_default_target() {
        // Arrange
        let target = application::list_default_targets()[0];

        // Act
        let result = IcmpEchoSender.send_echo(target).await;

        // Assert
        assert!(matches!(result, EchoResult::Answered(round_trip) if round_trip < ECHO_TIMEOUT));
    }
}
