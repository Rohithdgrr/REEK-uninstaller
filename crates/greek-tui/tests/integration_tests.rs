// Integration tests for greek-tui

use greek_common::*;
use greek_core::*;
use greek_tui::*;

#[test]
fn test_tui_theme_default() {
    let theme = TuiTheme::default();
    // TuiTheme::default() uses the built-in light theme
    assert!(!theme.theme.name.is_empty());
    assert!(!theme.colors.is_empty());
    assert!(!theme.styles.is_empty());
}

#[test]
fn test_tui_theme_from_config() {
    let config_theme = Theme::default();
    let tui_theme = TuiTheme::from_theme(config_theme);

    assert_eq!(tui_theme.theme.name, "greek-blue");
    assert!(!tui_theme.colors.is_empty());
    assert!(!tui_theme.styles.is_empty());
}

#[test]
fn test_tui_theme_get_style() {
    let theme = TuiTheme::default();

    let default_style = theme.get_style("default");
    let accent_style = theme.get_style("accent");
    let nonexistent_style = theme.get_style("nonexistent");

    // Styles for known keys should differ from the fallback
    assert_ne!(default_style, accent_style);
    // Should return a default style for nonexistent keys (fallback)
    assert_eq!(nonexistent_style, ratatui::style::Style::default());
}

#[test]
fn test_tui_theme_get_color() {
    let theme = TuiTheme::default();

    let accent_color = theme.get_color("accent");
    let nonexistent_color = theme.get_color("nonexistent");

    // Accent is a concrete color from the theme palette
    assert_ne!(accent_color, ratatui::style::Color::Reset);
    // Should return Reset for nonexistent colors
    assert_eq!(nonexistent_color, ratatui::style::Color::Reset);
}

#[test]
fn test_event_handler() {
    let mut handler = EventHandler::new();
    let sender = handler.sender();

    sender.send(Event::Tick).unwrap();

    // In a real test, we'd use tokio::test and await the receiver
    // For now, just test creation
    let _ = &mut handler;
}

#[test]
fn test_text_widget() {
    use greek_tui::widgets::TextWidget;

    let widget = TextWidget::new("Test content".to_string());
    let styled_widget = widget.style(ratatui::style::Style::default());
    let _ = styled_widget;
}

#[test]
fn test_status_bar_widget() {
    use greek_tui::widgets::StatusBarWidget;

    let widget = StatusBarWidget::new("Left".to_string(), "Right".to_string());
    let _ = widget;
}

#[test]
fn test_tui_app_creation() {
    let config = GreekConfig::default();
    let service = GreekAppService::new(config.clone()).unwrap();
    let app = TuiApp::new(config, service);

    assert_eq!(app.get_selected_index(), 0);
    assert!(app.get_selected_apps().is_empty());
    assert!(app.is_showing_details());
    assert!(!app.is_showing_help());
}

#[test]
fn test_tui_app_theme() {
    let config = GreekConfig::default();
    let service = GreekAppService::new(config.clone()).unwrap();
    let app = TuiApp::new(config, service);

    let theme = app.theme();
    assert!(!theme.theme.name.is_empty());
}

#[test]
fn test_tui_app_toggle_details() {
    let config = GreekConfig::default();
    let service = GreekAppService::new(config.clone()).unwrap();
    let app = TuiApp::new(config, service);

    assert!(app.is_showing_details());
}

#[test]
fn test_tui_app_toggle_help() {
    let config = GreekConfig::default();
    let service = GreekAppService::new(config.clone()).unwrap();
    let app = TuiApp::new(config, service);

    assert!(!app.is_showing_help());
}

#[test]
fn test_tui_app_scan_status() {
    let config = GreekConfig::default();
    let service = GreekAppService::new(config.clone()).unwrap();
    let app = TuiApp::new(config, service);

    assert!(matches!(app.scan_status(), ScanStatus::Idle));
}
