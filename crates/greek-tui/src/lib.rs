// greek-tui - Terminal User Interface for REEK Ultimate Uninstaller

pub mod app;
pub mod events;
pub mod theme;
pub mod ui;
pub mod widgets;

pub use app::ScanStatus;
pub use app::TuiApp;
pub use events::{AppEvent, Event, EventHandler};
pub use theme::TuiTheme;
pub use ui::render;
pub use widgets::{StatusBarWidget, TextWidget};
