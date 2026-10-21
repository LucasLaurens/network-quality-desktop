use std::time::Duration;

/// What one probe to one target taught us this second.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProbeOutcome {
    /// The echo came back in time.
    Answered(Duration),
    /// The echo was lost, but the target still accepts a TCP connection: a lost packet, not an outage.
    LostButReachable,
    /// Neither the echo nor a TCP connection got through.
    Unreachable,
}
