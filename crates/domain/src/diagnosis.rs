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
    Measuring { collected: u32, needed: u32 },
    NoNetwork,
}
