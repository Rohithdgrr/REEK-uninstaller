// Integration tests for greek-tui

use greek_tui::*;
use greek_common::*;
use greek_core::*;

#[test]
fn test_tui_theme_creation() {
    let theme = TuiTheme::default();
    assert_eq!(theme.theme.name, "greek-blue");
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
    
    // Should return a style even for nonexistent keys (fallback to default)
    assert_eq!(nonexistent_style, Style::default());
}

#[test]
fn test_tui_theme_get_color() {
    let theme = TuiTheme::default();
    
    let accent_color = theme.get_color("accent");
    let nonexistent_color = theme.get_color("nonexistent");
    
    // Should return Reset for nonexistent colors
    assert_eq!(nonexistent_color, ratatui::style::Color::Reset);
}

#[test]
fn test_builtin_themes() {
    let themes = get_builtin_themes();
    assert!(!themes.is_empty());
    assert_eq!(themes[0].name, "greek-blue");
    
    // Check for expected themes
    let theme_names: Vec<_> = themes.iter().map(|t| &t.name).collect();
    assert!(theme_names.contains(&"greek-blue"));
    assert!(theme_names.contains(&"greek-light"));
    assert!(theme_names.contains(&"matrix"));
}

#[test]
fn test_event_handler() {
    let mut handler = EventHandler::new();
    let sender = handler.sender();
    
    sender.send(Event::Tick).unwrap();
    
    // In a real test, we'd use tokio::test and await the receiver
    // For now, just test creation
}

#[test]
fn test_text_widget() {
    let widget = TextWidget::new("Test content".to_string());
    assert_eq!(widget.content, "Test content");
    
    let styled_widget = widget.style(ratatui::style::Style::default());
}

#[test]
fn test_status_bar_widget() {
    let widget = StatusBarWidget::new("Left".to_string(), "Right".to_string());
    assert_eq!(widget.left_content, "Left");
    assert_eq!(widget.right_content, "Right");
}

#[test]
fn test_tui_app_creation() {
    let config = GreekConfig::default();
    let service = GreekAppService::new(config.clone()).unwrap();
    let app = TuiApp::new(config, service);
    
    assert_eq!(app.selected_app_index, 0);
    assert!(app.selected_apps.is_empty());
    assert!(app.is_showing_details());
    assert!(!app.is_showing_help());
}

#[test]
fn test_tui_app_theme() {
    let config = GreekConfig::default();
    let service = GreekAppService::new(config.clone()).unwrap();
    let app = TuiApp::new(config, service);
    
    let theme = app.theme();
    assert_eq!(theme.theme.name, "greek-blue");
}

#[test]
fn test_tui_app_toggle_details() {
    let config = GreekConfig::default();
    let service = GreekAppService::new(config.clone()).unwrap();
    let mut app = TuiApp::new(config, service);
    
    assert!(app.is_showing_details());
    
    // Toggle details (would normally be done via event)
    // For now, just test the getter
}

#[test]
fn test_tui_app_toggle_help() {
    let config = GreekConfig::default();
    let service = GreekAppService::new(config.clone()).unwrap();
    let mut app = TuiApp::new(config, service);
    
    assert!(!app.is_showing_help());
    
    // Toggle help (would normally be done via event)
    // For now, just test the getter
}

#[test]
fn test_tui_app_batch_queue() {
    let config = GreekConfig::default();
    let service = GreekAppService::new(config.clone()).unwrap();
    let mut app = TuiApp::new(config, service);
    
    let test_app = InstalledApp::new(
        "Test App".to_string(),
        InstallSource::Registry {
            hive: RegistryHive::Hklm,
            key_path: "test".to_string(),
            is_64_bit: true,
        },
    );
    
    app.add_to_batch(test_app);
    
    assert!(app.batch_queue.is_some());
}
