//! Event handling for keyboard input and periodic UI updates.

use crossterm::event::{self, Event as CrosstermEvent, KeyEvent};
use std::time::Duration;
use tokio::sync::mpsc;

/// Events that can occur in the TUI application.
#[derive(Debug, Clone)]
pub enum Event {
    /// Keyboard input event
    Key(KeyEvent),
    /// Periodic tick for UI refresh
    Tick,
    /// Terminal resize event
    #[allow(dead_code)]
    Resize(u16, u16),
}

/// Event handler that polls for terminal events and sends them through a channel.
pub struct EventHandler {
    /// Channel sender for events
    sender: mpsc::UnboundedSender<Event>,
    /// How often to send Tick events (for UI refresh)
    tick_rate: Duration,
}

impl EventHandler {
    /// Create a new event handler.
    ///
    /// # Arguments
    ///
    /// * `tick_rate` - How often to send Tick events (default 100ms for 10 FPS)
    pub fn new(tick_rate: Duration) -> (Self, mpsc::UnboundedReceiver<Event>) {
        let (sender, receiver) = mpsc::unbounded_channel();
        let handler = Self { sender, tick_rate };
        (handler, receiver)
    }

    /// Run the event handler loop.
    ///
    /// This spawns a blocking task that polls for terminal events and sends them
    /// through the channel. It runs until the channel is closed.
    pub async fn run(self) {
        tokio::task::spawn_blocking(move || {
            let mut last_tick = std::time::Instant::now();

            loop {
                // Calculate timeout until next tick
                let timeout = self.tick_rate.saturating_sub(last_tick.elapsed());

                // Poll for events with timeout
                if event::poll(timeout).unwrap_or(false) {
                    match event::read() {
                        Ok(CrosstermEvent::Key(key)) => {
                            if self.sender.send(Event::Key(key)).is_err() {
                                // Channel closed, exit loop
                                break;
                            }
                        }
                        Ok(CrosstermEvent::Resize(w, h))
                            if self.sender.send(Event::Resize(w, h)).is_err() =>
                        {
                            break;
                        }
                        _ => {}
                    }
                }

                // Send tick event if enough time has passed
                if last_tick.elapsed() >= self.tick_rate {
                    if self.sender.send(Event::Tick).is_err() {
                        break;
                    }
                    last_tick = std::time::Instant::now();
                }
            }
        });
    }
}
