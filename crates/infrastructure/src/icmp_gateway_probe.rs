//! Pings the gateway with the same ICMP sender as the Internet targets, without its TCP fallback.

use std::net::Ipv4Addr;

use application::GatewayProbe;
use domain::{GatewayEcho, Target};

use crate::{EchoResult, EchoSender};

#[derive(Debug, Clone, Copy)]
pub struct IcmpGatewayProbe<E> {
    echo: E,
}

impl<E> IcmpGatewayProbe<E> {
    #[must_use]
    pub const fn new(echo: E) -> Self {
        Self { echo }
    }
}

impl<E: EchoSender + Sync> GatewayProbe for IcmpGatewayProbe<E> {
    async fn ping_gateway(&self, gateway: Ipv4Addr) -> GatewayEcho {
        match self.echo.send_echo(Target::new(gateway.into())).await {
            EchoResult::Answered(round_trip) => GatewayEcho::Answered(round_trip),
            EchoResult::NoAnswer => GatewayEcho::Lost,
            EchoResult::NotPermitted => GatewayEcho::NotPermitted,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::future::Future;
    use std::time::Duration;

    use super::*;

    const GATEWAY: Ipv4Addr = Ipv4Addr::new(192, 168, 1, 1);

    /// Answers every echo the same way.
    struct InMemoryEcho(EchoResult);

    impl EchoSender for InMemoryEcho {
        fn send_echo(&self, _target: Target) -> impl Future<Output = EchoResult> + Send {
            std::future::ready(self.0)
        }
    }

    #[tokio::test]
    async fn reports_the_round_trip_of_an_answered_ping() {
        // Arrange
        let round_trip = Duration::from_millis(3);
        let probe = IcmpGatewayProbe::new(InMemoryEcho(EchoResult::Answered(round_trip)));

        // Act
        let echo = probe.ping_gateway(GATEWAY).await;

        // Assert
        assert_eq!(echo, GatewayEcho::Answered(round_trip));
    }

    #[tokio::test]
    async fn passes_on_a_refusal_rather_than_a_loss() {
        // Arrange
        let probe = IcmpGatewayProbe::new(InMemoryEcho(EchoResult::NotPermitted));

        // Act
        let echo = probe.ping_gateway(GATEWAY).await;

        // Assert
        assert_eq!(echo, GatewayEcho::NotPermitted);
    }
}
