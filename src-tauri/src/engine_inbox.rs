//! The measurement actor's inbox, for messages that come from outside its clock: the dashboard's
//! reads, the last minute before quitting, sleep and wake.

use std::sync::mpsc::SyncSender;
use std::time::{Duration, Instant};

use application::{EngineMessage, LiveHistory, PowerEvent};
use tokio::sync::{mpsc, oneshot};

/// How long quitting waits for the last minute to be written.
pub const FLUSH_TIMEOUT: Duration = Duration::from_secs(2);
/// Windows lets about 2 s pass between the sleep notice and the sleep: answer before it.
pub const SLEEP_TIMEOUT: Duration = Duration::from_millis(1500);
const SEND_RETRY_EVERY: Duration = Duration::from_millis(20);

#[derive(Clone)]
pub struct EngineInbox(pub mpsc::Sender<EngineMessage>);

impl EngineInbox {
    /// The last hour and the current network; empty when the engine is gone.
    pub async fn read_live(&self) -> LiveHistory {
        let (answer, answered) = oneshot::channel();
        if self
            .0
            .send(EngineMessage::ReadLiveHistory(answer))
            .await
            .is_err()
        {
            return LiveHistory::default();
        }
        answered.await.unwrap_or_default()
    }

    /// Writes the minute in progress before the app quits.
    pub fn flush(&self) {
        if !self.send_and_wait(EngineMessage::Flush, FLUSH_TIMEOUT) {
            eprintln!("The last minute could not be written before quitting");
        }
    }

    /// Before a sleep, returns once the minute in progress is written or the delay ran out; a
    /// wake is only posted: the engine catches up at its next tick.
    pub fn relay_power_event(&self, event: PowerEvent) {
        match event {
            PowerEvent::WillSleep => {
                if !self.send_and_wait(EngineMessage::Sleep, SLEEP_TIMEOUT) {
                    eprintln!("The minute in progress could not be written before the sleep");
                }
            }
            PowerEvent::DidWake => {
                // A full inbox drops it: the gap between two ticks is noticed as a wake anyway.
                let _ = self.0.try_send(EngineMessage::Wake);
            }
        }
    }

    /// Sends a message that answers once done, and waits for the answer. Never waits more than
    /// `timeout`, sending included (full inbox, engine gone). Blocking: never call it from the
    /// engine's runtime.
    fn send_and_wait(
        &self,
        create: impl FnOnce(SyncSender<()>) -> EngineMessage,
        timeout: Duration,
    ) -> bool {
        let deadline = Instant::now() + timeout;
        let (answer, answered) = std::sync::mpsc::sync_channel(1);
        let mut message = create(answer);
        loop {
            match self.0.try_send(message) {
                Ok(()) => break,
                Err(mpsc::error::TrySendError::Full(returned)) if Instant::now() < deadline => {
                    message = returned;
                    std::thread::sleep(SEND_RETRY_EVERY);
                }
                Err(_) => return false,
            }
        }
        answered
            .recv_timeout(deadline.saturating_duration_since(Instant::now()))
            .is_ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Plays the engine on a thread of its own: answers sleeps, or keeps them unanswered.
    fn start_engine(answers: bool) -> (EngineInbox, std::thread::JoinHandle<Vec<&'static str>>) {
        let (sender, mut inbox) = mpsc::channel(4);
        let engine = std::thread::spawn(move || {
            let mut received = Vec::new();
            let mut unanswered = Vec::new();
            while let Some(message) = inbox.blocking_recv() {
                match message {
                    EngineMessage::Sleep(answer) | EngineMessage::Flush(answer) => {
                        received.push("answering");
                        if answers {
                            let _ = answer.send(());
                        } else {
                            unanswered.push(answer);
                        }
                    }
                    EngineMessage::Wake => received.push("wake"),
                    EngineMessage::Tick
                    | EngineMessage::NetworkChanged
                    | EngineMessage::ReadLiveHistory(_) => received.push("other"),
                }
            }
            received
        });
        (EngineInbox(sender), engine)
    }

    #[test]
    fn lets_the_system_sleep_once_the_minute_is_written() {
        // Arrange
        let (inbox, engine) = start_engine(true);

        // Act
        let started = Instant::now();
        inbox.relay_power_event(PowerEvent::WillSleep);
        let waited = started.elapsed();

        // Assert
        drop(inbox);
        assert_eq!(engine.join().expect("the engine"), vec!["answering"]);
        assert!(waited < SLEEP_TIMEOUT);
    }

    #[test]
    fn stops_waiting_for_an_engine_that_does_not_answer() {
        // Arrange
        let (inbox, engine) = start_engine(false);

        // Act
        let started = Instant::now();
        inbox.relay_power_event(PowerEvent::WillSleep);
        let waited = started.elapsed();

        // Assert
        drop(inbox);
        let _ = engine.join();
        assert!(waited >= SLEEP_TIMEOUT);
        assert!(waited < SLEEP_TIMEOUT + Duration::from_millis(500));
    }

    #[test]
    fn posts_a_wake_without_waiting() {
        // Arrange
        let (inbox, engine) = start_engine(false);

        // Act
        inbox.relay_power_event(PowerEvent::DidWake);

        // Assert
        drop(inbox);
        assert_eq!(engine.join().expect("the engine"), vec!["wake"]);
    }
}
