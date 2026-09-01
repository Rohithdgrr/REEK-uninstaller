// Main entry point for the TUI application

use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use greek_core::{ConfigManager, GreekAppService};
use greek_tui::TuiApp;
use ratatui::{
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Terminal,
};
use std::io;

fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;
    // Structured logging for TUI (file JSON + stderr; stdout reserved for TUI)
    let _log_guard = greek_common::logging::init_logging(false)
        .map_err(|e| color_eyre::eyre::eyre!("Failed to init logging: {}", e))?;
    greek_common::logging::prune_old_logs(14);
    tracing::info!("reek-tui starting");

    // Graceful shutdown: ensure terminal is restored even on panic (audit §6.1)
    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let _ = disable_raw_mode();
        let _ = execute!(io::stdout(), LeaveAlternateScreen, DisableMouseCapture);
        original_hook(info);
    }));

    let config_manager = ConfigManager::new()?;
    let config = config_manager.load_config()?;
    let mut service = GreekAppService::new(config.clone())?;

    // Setup terminal with mouse support
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    terminal.clear()?;

    // Show scanning splash
    terminal.draw(|f| {
        let area = f.area();
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(0)])
            .split(area);

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Rgb(180, 195, 220)))
            .style(Style::default().bg(Color::Rgb(230, 240, 252)));

        let inner = block.inner(chunks[0]);
        f.render_widget(block, chunks[0]);

        let lines = vec![
            Line::from(""),
            Line::from(vec![
                Span::styled(
                    "  REEK ",
                    Style::default()
                        .fg(Color::Rgb(30, 64, 175))
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled("Uninstaller", Style::default().fg(Color::Rgb(55, 100, 200))),
            ]),
            Line::from(""),
            Line::from(vec![Span::styled(
                "  Scanning installed applications...",
                Style::default().fg(Color::Rgb(100, 116, 139)),
            )]),
            Line::from(""),
            Line::from(vec![Span::styled(
                "  Reading Windows Registry...",
                Style::default().fg(Color::Rgb(148, 163, 184)),
            )]),
        ];

        f.render_widget(Paragraph::new(lines).alignment(Alignment::Left), inner);
    })?;

    // Do initial scan
    let rt = tokio::runtime::Runtime::new()?;
    let scan_result = rt.block_on(async { service.scan_all_apps().await });

    let mut app = TuiApp::new(config, service, rt.handle().clone());

    match scan_result {
        Ok(apps) => {
            app.set_apps(apps);
        }
        Err(e) => {
            app.set_scan_error(format!("Scan failed: {}", e));
        }
    }

    let result = app.run(&mut terminal);

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    result.map_err(|e| color_eyre::eyre::eyre!("{}", e))?;
    Ok(())
}
