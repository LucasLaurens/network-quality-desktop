use crate::Fault;

/// The three measured quantities, in the order they are shown.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Metric {
    Latency,
    Jitter,
    Loss,
}

/// Why the measurements start over.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MeasuringCause {
    /// At launch, or back from an outage.
    FirstMeasures,
    /// Another network took over: what was measured before says nothing about it.
    NewNetwork,
    /// Back from sleep: the connection may have changed while the computer slept.
    Wake,
    /// The user picked other servers: what was measured against the old ones says nothing.
    NewServers,
}

/// Why the connection is in its current state: the « pourquoi » shown under the state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Diagnosis {
    Healthy,
    NearThreshold(Metric),
    /// The Internet segment is past a threshold, and where the fault lies.
    OutOfThreshold {
        metric: Metric,
        fault: Fault,
    },
    /// Every metric is back within its thresholds; the state waits before improving.
    Recovering,
    Measuring {
        collected: u32,
        needed: u32,
        cause: MeasuringCause,
    },
    /// No default route: the computer is not connected to any network.
    NoNetwork,
    /// The gateway answers while no target does: the box has lost its Internet access.
    GatewayWithoutInternet,
    /// No target answered for 5 s, and the gateway is silent or cannot be pinged.
    NoServerReachable,
}
