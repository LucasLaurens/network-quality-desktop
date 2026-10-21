//! logind's `PrepareForSleep` signal, with a delay lock: logind waits for the lock to be released
//! (5 s at most by default) before sleeping, and a new lock is taken after each wake.

use std::sync::mpsc;
use std::time::Duration;

use application::PowerEvent;
use dbus::arg::OwnedFd;
use dbus::blocking::Connection;
use dbus::message::MatchRule;

use super::PowerHandler;

const LOGIND: &str = "org.freedesktop.login1";
const LOGIND_PATH: &str = "/org/freedesktop/login1";
const MANAGER: &str = "org.freedesktop.login1.Manager";
const CALL_TIMEOUT: Duration = Duration::from_secs(2);
/// How long one wait for a signal lasts; the loop simply waits again.
const LISTEN_EVERY: Duration = Duration::from_secs(60);
/// Shown by `systemd-inhibit --list`.
const WHO: &str = "Qualité de connexion";
const WHY: &str = "Écrire la minute en cours avant la veille";

pub fn watch(on_event: PowerHandler) {
    let spawned = std::thread::Builder::new()
        .name("power-events".to_owned())
        .spawn(move || {
            if let Err(error) = listen(&on_event) {
                // Without logind (no systemd, a container), the gap between two ticks stands in.
                eprintln!("Watching sleep through logind failed: {error}");
            }
        });
    if let Err(error) = spawned {
        eprintln!("Starting the sleep watcher failed: {error}");
    }
}

/// Runs for the life of the app. O(1) per signal plus the engine's answer.
fn listen(on_event: &PowerHandler) -> Result<(), dbus::Error> {
    let connection = Connection::new_system()?;
    let (sender, received) = mpsc::channel();
    connection.add_match(
        MatchRule::new_signal(MANAGER, "PrepareForSleep"),
        move |(starting,): (bool,), _: &Connection, _: &dbus::Message| {
            // The receiver lives as long as this loop: a failed send means the loop is over.
            let _ = sender.send(starting);
            true
        },
    )?;
    let mut lock = DelayLock::take(&connection);
    loop {
        connection.process(LISTEN_EVERY)?;
        while let Ok(starting) = received.try_recv() {
            if starting {
                on_event(PowerEvent::WillSleep);
                lock.release();
            } else {
                on_event(PowerEvent::DidWake);
                lock = DelayLock::take(&connection);
            }
        }
    }
}

/// Holds the sleep back while held; empty when logind refused it (the sleep is then not delayed).
struct DelayLock(Option<OwnedFd>);

impl DelayLock {
    fn take(connection: &Connection) -> Self {
        let taken: Result<(OwnedFd,), dbus::Error> = connection
            .with_proxy(LOGIND, LOGIND_PATH, CALL_TIMEOUT)
            .method_call(MANAGER, "Inhibit", ("sleep", WHO, WHY, "delay"));
        match taken {
            Ok((descriptor,)) => Self(Some(descriptor)),
            Err(error) => {
                eprintln!("Taking the sleep delay lock failed: {error}");
                Self(None)
            }
        }
    }

    /// Closing the descriptor lets the sleep go on.
    fn release(&mut self) {
        self.0 = None;
    }
}
