//! The traffic seen on the interface, sampled every second on a thread of its own for as long as
//! the app runs, so the peak is there whenever the details open.

use std::sync::{Arc, Mutex, PoisonError};
use std::time::{Duration, Instant};

use application::TrafficCounterReader;
use domain::{TrafficMeter, TrafficReading};

use crate::windows::report_failure;

pub const SAMPLE_EVERY: Duration = Duration::from_secs(1);

#[derive(Clone, Default)]
pub struct TrafficStore(Arc<Mutex<TrafficMeter>>);

impl TrafficStore {
    /// Without a thread, the details say the traffic is not available yet.
    #[must_use]
    pub fn start(reader: impl TrafficCounterReader + Send + 'static) -> Self {
        let store = Self::default();
        let meter = Arc::clone(&store.0);
        let started = Instant::now();
        let spawned = std::thread::Builder::new()
            .name("traffic".to_owned())
            .spawn(move || {
                loop {
                    if let Some(counters) = reader.read_counters() {
                        meter
                            .lock()
                            .unwrap_or_else(PoisonError::into_inner)
                            .record(started.elapsed(), counters);
                    }
                    std::thread::sleep(SAMPLE_EVERY);
                }
            });
        report_failure("Starting the traffic sampling", spawned);
        store
    }

    /// The last window and the peak of the last hour. O(w) for w ≤ 720 windows.
    #[must_use]
    pub fn read(&self) -> TrafficReading {
        self.0
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .compute_reading()
    }
}
