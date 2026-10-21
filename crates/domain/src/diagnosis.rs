/// The three measured quantities, in the order they are shown.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Metric {
    Latency,
    Jitter,
    Loss,
}

/// Why the connection is in its current state: the « pourquoi » shown under the state.
/// Segment attribution (Wi-Fi, operator, mobile network) comes with the gateway probe.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Diagnosis {
    Healthy,
    NearThreshold(Metric),
    OutOfThreshold(Metric),
    /// Every metric is back within its thresholds; the state waits before improving.
    Recovering,
    Measuring {
        collected: u32,
        needed: u32,
    },
    /// No target answered for 5 s; whether a network is connected at all is not known yet.
    NoServerReachable,
}
