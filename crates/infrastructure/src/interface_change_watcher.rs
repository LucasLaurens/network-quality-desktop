//! Interface and address changes from the system (netlink on Linux, IP Helper on Windows, the
//! routing socket on macOS) through `netwatcher`, on a thread of its own.

use application::NetworkChangeWatcher;

#[derive(Debug, Clone, Copy, Default)]
pub struct InterfaceChangeWatcher;

impl NetworkChangeWatcher for InterfaceChangeWatcher {
    fn watch(self, mut on_change: impl FnMut() + Send + 'static) {
        let spawned = std::thread::Builder::new()
            .name("interface-changes".to_owned())
            .spawn(move || {
                let mut watch = match netwatcher::watch_interfaces_blocking() {
                    Ok(watch) => watch,
                    Err(error) => {
                        // The periodic reading still notices a new network, ten seconds later.
                        eprintln!("Watching network changes failed: {error:?}");
                        return;
                    }
                };
                // The first answer is the current state, not a change.
                watch.changed();
                loop {
                    watch.changed();
                    on_change();
                }
            });
        if let Err(error) = spawned {
            eprintln!("Starting the network change watcher failed: {error}");
        }
    }
}
