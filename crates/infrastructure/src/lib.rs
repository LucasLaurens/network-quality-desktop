//! OS adapters, SQLite storage and HTTP clients behind the application ports.
//! The only crate allowed to contain `unsafe` code, for native FFI.

mod icmp_latency_probe;
mod system_clock;

pub use icmp_latency_probe::IcmpLatencyProbe;
pub use system_clock::SystemClock;
