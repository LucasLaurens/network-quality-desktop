use std::io::ErrorKind;
use std::net::SocketAddr;
use std::time::{Duration, Instant};

use domain::Target;
use tokio::net::TcpStream;

use crate::ConnectionOpener;

/// HTTPS port, open on the default targets and on most public servers.
const PORT: u16 = 443;
/// As long as the echo timeout: a slow link must not read as an unreachable one.
const CONNECT_TIMEOUT: Duration = Duration::from_millis(900);

/// Times a TCP handshake: roughly one round trip, without any special right.
#[derive(Debug, Clone, Copy, Default)]
pub struct TcpConnectionOpener;

impl ConnectionOpener for TcpConnectionOpener {
    async fn open_connection(&self, target: Target) -> Option<Duration> {
        let started = Instant::now();
        let attempt = tokio::time::timeout(
            CONNECT_TIMEOUT,
            TcpStream::connect(SocketAddr::new(target.to_address(), PORT)),
        );
        match attempt.await {
            Ok(Ok(_stream)) => Some(started.elapsed()),
            // A refusal still proves the host answered.
            Ok(Err(error)) if error.kind() == ErrorKind::ConnectionRefused => {
                Some(started.elapsed())
            }
            Ok(Err(_)) | Err(_) => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Needs network access, so it only runs on demand: `cargo test -- --ignored`.
    #[tokio::test]
    #[ignore = "real network"]
    async fn connects_to_a_default_target_over_https() {
        // Arrange
        let target = application::list_default_targets()[0];

        // Act
        let round_trip = TcpConnectionOpener.open_connection(target).await;

        // Assert
        assert!(round_trip.is_some_and(|round_trip| round_trip < CONNECT_TIMEOUT));
    }
}
