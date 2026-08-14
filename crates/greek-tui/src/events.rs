// Event handling for the TUI

use crossterm::event::{KeyEvent, MouseEvent};
use tokio::sync::mpsc;
use uuid;

/// Application events
#[derive(Debug, Clone, PartialEq)]
pub enum Event {
    /// Tick event for periodic updates
    Tick,
    /// Key press event
    Key(KeyEvent),
    /// Mouse event
    Mouse(MouseEvent),
    /// Application quit event
    Quit,
    /// Refresh event (force redraw)
    Refresh,
    /// Custom app event
    App(AppEvent),
}

/// Custom application events
#[derive(Debug, Clone, PartialEq)]
pub enum AppEvent {
    /// Scan started
    ScanStarted,
    /// Scan completed
    ScanCompleted,
    /// App selected
    AppSelected(uuid::Uuid),
    /// Uninstall started
    UninstallStarted(uuid::Uuid),
    /// Uninstall completed
    UninstallCompleted(uuid::Uuid, bool),
    /// Error occurred
    Error(String),
    /// Show help
    ShowHelp,
    /// Hide help
    HideHelp,
    /// Toggle details panel
    ToggleDetails,
    /// Search query changed
    SearchChanged(String),
    /// Filter changed
    FilterChanged(String),
}

/// Event handler
pub struct EventHandler {
    sender: mpsc::UnboundedSender<Event>,
    receiver: mpsc::UnboundedReceiver<Event>,
}

impl EventHandler {
    pub fn new() -> Self {
        let (sender, receiver) = mpsc::unbounded_channel();
        Self { sender, receiver }
    }

    pub fn sender(&self) -> mpsc::UnboundedSender<Event> {
        self.sender.clone()
    }

    pub async fn next(&mut self) -> Option<Event> {
        self.receiver.recv().await
    }
}

impl Default for EventHandler {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_event_handler() {
        let mut handler = EventHandler::new();
        let sender = handler.sender();

        sender.send(Event::Tick).unwrap();

        let event = handler.next().await;
        assert_eq!(event, Some(Event::Tick));
    }
}
