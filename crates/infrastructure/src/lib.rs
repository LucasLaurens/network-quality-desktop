//! OS adapters, SQLite storage and HTTP clients behind the application ports.
//! The only crate allowed to contain `unsafe` code, for native FFI.

mod background;
mod cloudflare_throughput_probe;
mod fallback_latency_probe;
mod history;
mod icmp_echo_sender;
mod icmp_gateway_probe;
mod interface_change_watcher;
mod metering;
mod netdev_context_reader;
mod power;
mod system_clock;
mod system_tray_host;
mod tcp_connection_opener;
mod upnp_access_detector;
mod wifi;

pub use background::keep_measuring_in_background;
pub use cloudflare_throughput_probe::CloudflareThroughputProbe;
pub use fallback_latency_probe::{ConnectionOpener, EchoResult, EchoSender, FallbackLatencyProbe};
pub use history::{OpenedHistory, SqliteHistoryRecorder, SqliteMeasurementRepository};
pub use icmp_echo_sender::IcmpEchoSender;
pub use icmp_gateway_probe::IcmpGatewayProbe;
pub use interface_change_watcher::InterfaceChangeWatcher;
pub use netdev_context_reader::NetdevContextReader;
pub use power::SystemPowerEventSource;
pub use system_clock::SystemClock;
pub use system_tray_host::SystemTrayHost;
pub use tcp_connection_opener::TcpConnectionOpener;
pub use upnp_access_detector::UpnpAccessDetector;
pub use wifi::{LocationRequest, SystemWifiReader, request_location_permission};
