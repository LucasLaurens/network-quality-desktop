use domain::{MeasurementServers, Target};

/// Two providers, so one slow or filtering target cannot decide the verdict alone.
#[must_use]
pub fn list_default_targets() -> Vec<Target> {
    vec![Target::parse("1.1.1.1"), Target::parse("8.8.8.8")]
}

/// The targets the probes go to with these servers. O(1).
#[must_use]
pub fn list_targets(servers: MeasurementServers) -> Vec<Target> {
    match servers {
        MeasurementServers::Default => list_default_targets(),
        MeasurementServers::Custom(target) => vec![target],
    }
}
