//! Throughput transfers against Cloudflare's speed test endpoints (`__down`, `__up`), in plain
//! HTTPS. While a transfer runs, empty requests every 250 ms time the round trip under load; HTTP/2
//! carries them on the transfer's own connection, so no handshake weighs on them.

use std::time::{Duration, Instant};

use application::{ThroughputFailure, ThroughputProbe};
use domain::{Direction, Transfer};

const DOWNLOAD_URL: &str = "https://speed.cloudflare.com/__down";
const UPLOAD_URL: &str = "https://speed.cloudflare.com/__up";
const ROUND_TRIP_EVERY: Duration = Duration::from_millis(250);
/// Longer than any transfer a test starts: the slowest line stops climbing after 1 MB.
const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);
const USER_AGENT: &str = concat!("network-quality/", env!("CARGO_PKG_VERSION"));

pub struct CloudflareThroughputProbe {
    client: reqwest::Client,
}

impl CloudflareThroughputProbe {
    /// # Errors
    ///
    /// The HTTP client could not be built (no TLS backend): the tests are then unavailable.
    pub fn create() -> Result<Self, String> {
        // The process-wide TLS provider; one installed earlier is just as good.
        let _ = rustls::crypto::ring::default_provider().install_default();
        let client = reqwest::Client::builder()
            .timeout(REQUEST_TIMEOUT)
            .user_agent(USER_AGENT)
            .build()
            .map_err(|error| error.to_string())?;
        Ok(Self { client })
    }

    /// Downloads `bytes` to the last byte. O(bytes / chunk).
    async fn download(&self, bytes: u64) -> Result<u64, reqwest::Error> {
        let mut response = self
            .client
            .get(format!("{DOWNLOAD_URL}?bytes={bytes}"))
            .send()
            .await?
            .error_for_status()?;
        let mut received = 0;
        while let Some(chunk) = response.chunk().await? {
            received += u64::try_from(chunk.len()).unwrap_or(u64::MAX);
        }
        Ok(received)
    }

    /// Uploads `bytes` of zeros; the answer comes once the server has them all. O(bytes).
    async fn upload(&self, bytes: u64) -> Result<u64, reqwest::Error> {
        let body = vec![0_u8; usize::try_from(bytes).unwrap_or(usize::MAX)];
        self.client
            .post(UPLOAD_URL)
            .body(body)
            .send()
            .await?
            .error_for_status()?;
        Ok(bytes)
    }

    /// One empty request, timed to its answer. O(1).
    async fn measure_round_trip(&self) -> Option<Duration> {
        let started = Instant::now();
        let response = self
            .client
            .get(format!("{DOWNLOAD_URL}?bytes=0"))
            .send()
            .await
            .ok()?;
        response.error_for_status().ok()?;
        Some(started.elapsed())
    }

    /// Times the round trip every `ROUND_TRIP_EVERY` until it is dropped; the first one once the
    /// transfer has had time to fill the line.
    async fn sample_round_trips(&self, round_trips: &mut Vec<Duration>) {
        let mut ticks = tokio::time::interval_at(
            tokio::time::Instant::now() + ROUND_TRIP_EVERY,
            ROUND_TRIP_EVERY,
        );
        loop {
            ticks.tick().await;
            if let Some(round_trip) = self.measure_round_trip().await {
                round_trips.push(round_trip);
            }
        }
    }
}

impl ThroughputProbe for CloudflareThroughputProbe {
    async fn transfer(
        &self,
        direction: Direction,
        bytes: u64,
    ) -> Result<Transfer, ThroughputFailure> {
        let started = Instant::now();
        let mut loaded_round_trips = Vec::new();
        let moved = tokio::select! {
            moved = async {
                match direction {
                    Direction::Download => self.download(bytes).await,
                    Direction::Upload => self.upload(bytes).await,
                }
            } => moved,
            () = self.sample_round_trips(&mut loaded_round_trips) => {
                unreachable!("the sampling never ends")
            }
        };
        let elapsed = started.elapsed();
        match moved {
            Ok(moved) => Ok(Transfer {
                bytes: moved,
                elapsed,
                loaded_round_trips,
            }),
            Err(error) => {
                eprintln!("A throughput transfer failed: {error}");
                Err(ThroughputFailure::Unreachable)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use application::run_throughput_test;
    use domain::TestProfile;

    use super::*;

    // Reaches the Internet and moves about 5 MB: `cargo test -p infrastructure -- --ignored`.
    #[tokio::test]
    #[ignore = "measures the real line against Cloudflare"]
    async fn measures_the_line_against_cloudflare() {
        // Arrange
        let probe = CloudflareThroughputProbe::create().expect("an HTTP client");

        // Act
        let result = run_throughput_test(&probe, TestProfile::Quick).await;

        // Assert
        let result = result.expect("a result");
        println!("{result:?}");
        assert!(result.download.is_some());
        assert!(result.upload.is_some());
    }
}
