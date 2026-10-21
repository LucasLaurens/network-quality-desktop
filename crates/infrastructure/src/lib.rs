//! OS adapters, SQLite storage and HTTP clients behind the application ports.
//! The only crate allowed to contain `unsafe` code, for native FFI.

mod fallback_latency_probe;
mod icmp_echo_sender;
mod system_clock;
mod tcp_connection_opener;

pub use fallback_latency_probe::{ConnectionOpener, EchoResult, EchoSender, FallbackLatencyProbe};
pub use icmp_echo_sender::IcmpEchoSender;
pub use system_clock::SystemClock;
pub use tcp_connection_opener::TcpConnectionOpener;
