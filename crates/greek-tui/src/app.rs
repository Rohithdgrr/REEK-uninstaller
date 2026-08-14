// Main TUI application

use crate::theme::TuiTheme;
use crossterm::event::{
    self, Event as CrosstermEvent, KeyCode, KeyModifiers, MouseButton, MouseEventKind,
};
use fuzzy_matcher::skim::SkimMatcherV2;
use fuzzy_matcher::FuzzyMatcher;
use greek_common::{
    GreekConfig, InstallSource, InstalledApp, ProcessUsage, SystemStats, UninstallOptions,
};
use greek_core::GreekAppService;
use ratatui::backend::CrosstermBackend;
use ratatui::style::Color;
use ratatui::Terminal;
use std::sync::{mpsc, Arc};
#[cfg(all(target_os = "windows", feature = "windows"))]
use std::time::Duration;
use tokio::sync::Mutex;

pub struct TuiApp {
    _config: GreekConfig,
    theme: TuiTheme,
    service: Arc<Mutex<GreekAppService>>,
    should_quit: bool,

    // App state
    apps: Vec<InstalledApp>,
    filtered_apps: Vec<InstalledApp>,
    selected_app_index: usize,
    selected_apps: Vec<usize>,

    // UI state
    show_details: bool,
    show_help: bool,
    current_operation: Option<OperationState>,
    scan_status: ScanStatus,
    status_message: Option<(String, bool)>, // (message, is_error)

    // Context menu
    show_context_menu: bool,
    context_menu_index: usize,
    context_menu_app: Option<InstalledApp>,
    context_menu_y: u16,

    // Scroll
    scroll_offset: usize,

    // Channels
    scan_result_receiver: Option<mpsc::Receiver<Result<Vec<InstalledApp>, String>>>,
    action_result_receiver: Option<mpsc::Receiver<Result<String, String>>>,

    // System stats (bottom nav bar)
    stats: Option<SystemStats>,
    #[cfg(all(target_os = "windows", feature = "windows"))]
    stats_receiver: Option<mpsc::Receiver<SystemStats>>,

    // Search
    search_query: String,
    search_mode: bool,
    fuzzy_matcher: SkimMatcherV2,
}

#[derive(Debug, Clone)]
pub enum ScanStatus {
    Idle,
    Scanning,
    Complete(usize),
    Error(String),
}

#[derive(Debug, Clone)]
pub enum OperationState {
    Scanning,
    Uninstalling(String),
    ForceRemoving(String),
    AnalyzingLeftovers(String),
    AddingToBatch(String),
}

impl TuiApp {
    pub fn new(config: GreekConfig, service: GreekAppService) -> Self {
        let fuzzy_matcher = SkimMatcherV2::default();

        #[cfg(all(target_os = "windows", feature = "windows"))]
        let stats_receiver = {
            let (stats_tx, stats_rx) = mpsc::channel::<SystemStats>();
            let tx = stats_tx.clone();
            std::thread::spawn(move || {
                let mut collector = greek_windows::SystemStatsCollector::new();
                // Discard first sample
                let _ = collector.collect();
                loop {
                    let stats = collector.collect();
                    if tx.send(stats).is_err() {
                        break;
                    }
                    std::thread::sleep(Duration::from_secs(2));
                }
            });
            Some(stats_rx)
        };

        Self {
            _config: config,
            theme: TuiTheme::light(),
            service: Arc::new(Mutex::new(service)),
            should_quit: false,
            apps: Vec::new(),
            filtered_apps: Vec::new(),
            selected_app_index: 0,
            selected_apps: Vec::new(),
            show_details: true,
            show_help: false,
            current_operation: None,
            scan_status: ScanStatus::Idle,
            status_message: None,
            show_context_menu: false,
            context_menu_index: 0,
            context_menu_app: None,
            context_menu_y: 0,
            scroll_offset: 0,
            scan_result_receiver: None,
            action_result_receiver: None,
            stats: None,
            search_query: String::new(),
            search_mode: false,
            fuzzy_matcher,
            #[cfg(all(target_os = "windows", feature = "windows"))]
            stats_receiver,
        }
    }

    pub fn run(
        &mut self,
        terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        loop {
            self.check_results();
            terminal.draw(|f| {
                crate::ui::render(f, self);
            })?;
            if event::poll(std::time::Duration::from_millis(50))? {
                match event::read()? {
                    CrosstermEvent::Key(key) => self.handle_key_event(key),
                    CrosstermEvent::Mouse(mouse) => self.handle_mouse_event(mouse),
                    _ => {}
                }
            }
            if self.should_quit {
                break;
            }
        }
        Ok(())
    }

    fn check_results(&mut self) {
        // Check scan results
        if let Some(receiver) = &self.scan_result_receiver {
            if let Ok(result) = receiver.try_recv() {
                match result {
                    Ok(apps) => {
                        let count = apps.len();
                        self.apps = apps;
                        self.apply_filter();
                        self.selected_app_index = 0;
                        self.selected_apps.clear();
                        self.scroll_offset = 0;
                        self.scan_status = ScanStatus::Complete(count);
                        self.current_operation = None;
                        self.status_message =
                            Some((format!("Scan complete: {} apps found", count), false));
                    }
                    Err(e) => {
                        self.scan_status = ScanStatus::Error(e.clone());
                        self.current_operation = None;
                        self.status_message = Some((e, true));
                    }
                }
                self.scan_result_receiver = None;
            }
        }

        // Check action results
        if let Some(receiver) = &self.action_result_receiver {
            if let Ok(result) = receiver.try_recv() {
                match result {
                    Ok(msg) => {
                        self.status_message = Some((msg, false));
                    }
                    Err(e) => {
                        self.status_message = Some((e, true));
                    }
                }
                self.current_operation = None;
                self.action_result_receiver = None;
            }
        }

        // Poll live system stats (Windows-only collector)
        #[cfg(all(target_os = "windows", feature = "windows"))]
        if let Some(receiver) = &self.stats_receiver {
            while let Ok(stats) = receiver.try_recv() {
                self.stats = Some(stats);
            }
        }
    }

    fn handle_mouse_event(&mut self, mouse: crossterm::event::MouseEvent) {
        match mouse.kind {
            MouseEventKind::Down(MouseButton::Left) => {
                if self.show_context_menu {
                    self.handle_context_menu_click(mouse.row);
                    return;
                }
                let list_start: u16 = 4;
                let list_end: u16 = crossterm::terminal::size()
                    .map(|(_, h)| h - 4)
                    .unwrap_or(21);
                if mouse.row >= list_start && mouse.row < list_end {
                    let idx = (mouse.row - list_start) as usize + self.scroll_offset;
                    if idx < self.filtered_apps.len() {
                        self.selected_app_index = idx;
                    }
                }
            }
            MouseEventKind::Down(MouseButton::Right) => {
                if self.show_context_menu {
                    self.show_context_menu = false;
                    return;
                }
                let list_start: u16 = 4;
                let list_end: u16 = crossterm::terminal::size()
                    .map(|(_, h)| h - 4)
                    .unwrap_or(21);
                if mouse.row >= list_start && mouse.row < list_end {
                    let idx = (mouse.row - list_start) as usize + self.scroll_offset;
                    if idx < self.filtered_apps.len() {
                        self.selected_app_index = idx;
                        self.show_context_menu = true;
                        self.context_menu_index = 0;
                        self.context_menu_app = Some(self.filtered_apps[idx].clone());
                        self.context_menu_y = mouse
                            .row
                            .min(crossterm::terminal::size().map(|(_, h)| h).unwrap_or(24) - 9);
                    }
                }
            }
            MouseEventKind::ScrollUp => {
                if self.scroll_offset > 0 {
                    self.scroll_offset -= 1;
                }
            }
            MouseEventKind::ScrollDown => {
                let max = self.filtered_apps.len().saturating_sub(1);
                if self.scroll_offset < max {
                    self.scroll_offset += 1;
                }
            }
            _ => {}
        }
    }

    fn handle_context_menu_click(&mut self, row: u16) {
        let items = context_menu_items();
        let start = self.context_menu_y;
        if row >= start && row < start + items.len() as u16 {
            let action = items[(row - start) as usize];
            self.execute_action(action);
        } else {
            self.show_context_menu = false;
        }
    }

    fn execute_action(&mut self, action: Action) {
        self.show_context_menu = false;
        let app = match action {
            Action::ViewDetails => {
                self.show_details = true;
                return;
            }
            Action::Cancel => return,
            Action::Uninstall
            | Action::ForceRemove
            | Action::ScanLeftovers
            | Action::AddToBatch => {
                match self
                    .context_menu_app
                    .as_ref()
                    .or_else(|| self.filtered_apps.get(self.selected_app_index))
                {
                    Some(a) => a.clone(),
                    None => return,
                }
            }
        };

        let service = Arc::clone(&self.service);
        let (tx, rx) = mpsc::channel();
        self.action_result_receiver = Some(rx);

        match action {
            Action::Uninstall => {
                self.current_operation = Some(OperationState::Uninstalling(app.name.clone()));
                tokio::task::spawn_blocking(move || {
                    let rt = tokio::runtime::Runtime::new().unwrap();
                    let result = rt.block_on(async {
                        let svc = service.lock().await;
                        svc.uninstall_app(&app, UninstallOptions::standard()).await
                    });
                    let _ = tx.send(match result {
                        Ok(r) => Ok(format!(
                            "Uninstall {}: {}",
                            app.name,
                            if r.success { "Success" } else { "Failed" }
                        )),
                        Err(e) => Err(format!("Uninstall failed: {}", e)),
                    });
                });
            }
            Action::ForceRemove => {
                self.current_operation = Some(OperationState::ForceRemoving(app.name.clone()));
                tokio::task::spawn_blocking(move || {
                    let rt = tokio::runtime::Runtime::new().unwrap();
                    let result = rt.block_on(async {
                        let svc = service.lock().await;
                        svc.force_remove_app(&app, UninstallOptions::force()).await
                    });
                    let _ = tx.send(match result {
                        Ok(r) => Ok(format!(
                            "Force remove {}: {}",
                            app.name,
                            if r.success { "Success" } else { "Failed" }
                        )),
                        Err(e) => Err(format!("Force remove failed: {}", e)),
                    });
                });
            }
            Action::ScanLeftovers => {
                self.current_operation = Some(OperationState::AnalyzingLeftovers(app.name.clone()));
                tokio::task::spawn_blocking(move || {
                    let rt = tokio::runtime::Runtime::new().unwrap();
                    let result = rt.block_on(async {
                        let svc = service.lock().await;
                        svc.analyze_leftovers(&app).await
                    });
                    let _ = tx.send(match result {
                        Ok(artifacts) => Ok(format!(
                            "Leftover scan for {}: {} artifacts found",
                            app.name,
                            artifacts.len()
                        )),
                        Err(e) => Err(format!("Leftover scan failed: {}", e)),
                    });
                });
            }
            Action::AddToBatch => {
                if !self.selected_apps.contains(&self.selected_app_index) {
                    self.selected_apps.push(self.selected_app_index);
                }
                self.status_message = Some((format!("Added {} to batch", app.name), false));
                self.action_result_receiver = None;
            }
            _ => {}
        }
    }

    fn handle_key_event(&mut self, key: crossterm::event::KeyEvent) {
        if self.show_help {
            self.show_help = false;
            return;
        }

        if self.search_mode {
            match key.code {
                KeyCode::Char('/') | KeyCode::Esc => {
                    self.toggle_search();
                }
                KeyCode::Backspace => {
                    self.search_backspace();
                }
                KeyCode::Char(c) => {
                    self.search_input(c);
                }
                _ => {}
            }
            return;
        }

        if self.show_context_menu {
            match key.code {
                KeyCode::Esc | KeyCode::Char('q') => {
                    self.show_context_menu = false;
                }
                KeyCode::Up | KeyCode::Char('k') => {
                    self.context_menu_index = self.context_menu_index.saturating_sub(1);
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    let max = context_menu_items().len().saturating_sub(1);
                    if self.context_menu_index < max {
                        self.context_menu_index += 1;
                    }
                }
                KeyCode::Enter | KeyCode::Char(' ') => {
                    let action = context_menu_items()[self.context_menu_index];
                    self.execute_action(action);
                }
                _ => {}
            }
            return;
        }

        match key.code {
            // Quit
            KeyCode::Char('q') | KeyCode::Char('c')
                if key.modifiers.contains(KeyModifiers::CONTROL) =>
            {
                self.should_quit = true;
            }
            KeyCode::Esc => {
                if self.selected_apps.is_empty() {
                    self.should_quit = true;
                } else {
                    self.selected_apps.clear();
                    self.status_message = Some(("Selection cleared".into(), false));
                }
            }

            // Help
            KeyCode::Char('?') | KeyCode::Char('h') => {
                self.show_help = true;
            }
            KeyCode::Char('/') => {
                self.toggle_search();
            }

            // Navigation
            KeyCode::Up | KeyCode::Char('k') => {
                if self.selected_app_index > 0 {
                    self.selected_app_index -= 1;
                    self.ensure_visible();
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if self.selected_app_index < self.filtered_apps.len().saturating_sub(1) {
                    self.selected_app_index += 1;
                    self.ensure_visible();
                }
            }
            KeyCode::Home => {
                self.selected_app_index = 0;
                self.scroll_offset = 0;
            }
            KeyCode::End => {
                self.selected_app_index = self.filtered_apps.len().saturating_sub(1);
                self.ensure_visible();
            }
            KeyCode::PageUp => {
                self.selected_app_index = self.selected_app_index.saturating_sub(20);
                self.ensure_visible();
            }
            KeyCode::PageDown => {
                self.selected_app_index =
                    (self.selected_app_index + 20).min(self.filtered_apps.len().saturating_sub(1));
                self.ensure_visible();
            }

            // View toggle
            KeyCode::Char('d') | KeyCode::Enter | KeyCode::Tab => {
                self.show_details = !self.show_details;
            }

            // Selection
            KeyCode::Char(' ') => {
                if let Some(pos) = self
                    .selected_apps
                    .iter()
                    .position(|&i| i == self.selected_app_index)
                {
                    self.selected_apps.remove(pos);
                } else {
                    self.selected_apps.push(self.selected_app_index);
                }
            }
            KeyCode::Char('a') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.selected_apps = (0..self.filtered_apps.len()).collect();
            }
            KeyCode::Char('a') => {
                self.selected_apps = (0..self.filtered_apps.len()).collect();
            }
            KeyCode::Char('n') => {
                self.selected_apps.clear();
            }

            // Actions on selected app
            KeyCode::Char('u') | KeyCode::Char('U') => {
                self.execute_action(Action::Uninstall);
            }
            KeyCode::Char('f') | KeyCode::Char('F') => {
                self.execute_action(Action::ForceRemove);
            }
            KeyCode::Char('l') | KeyCode::Char('L') => {
                self.execute_action(Action::ScanLeftovers);
            }
            KeyCode::Char('b') | KeyCode::Char('B') => {
                self.execute_action(Action::AddToBatch);
            }

            // Rescan
            KeyCode::Char('r') | KeyCode::Char('R') => {
                self.trigger_rescan();
            }

            // Context menu
            KeyCode::Char('m') => {
                if let Some(app) = self.filtered_apps.get(self.selected_app_index).cloned() {
                    self.show_context_menu = true;
                    self.context_menu_index = 0;
                    self.context_menu_app = Some(app);
                    let (_, h) = crossterm::terminal::size().unwrap_or((80, 24));
                    let menu_items = context_menu_items();
                    let list_row = (self.selected_app_index - self.scroll_offset) as u16 + 4;
                    self.context_menu_y =
                        list_row.min(h.saturating_sub(menu_items.len() as u16 + 3));
                }
            }
            _ => {}
        }
    }

    fn ensure_visible(&mut self) {
        let h = crossterm::terminal::size().map(|(_, h)| h).unwrap_or(24);
        let visible = h.saturating_sub(8) as usize;
        if self.selected_app_index < self.scroll_offset {
            self.scroll_offset = self.selected_app_index;
        } else if self.selected_app_index >= self.scroll_offset + visible {
            self.scroll_offset = self.selected_app_index - visible + 1;
        }
    }

    fn trigger_rescan(&mut self) {
        if matches!(self.scan_status, ScanStatus::Scanning) {
            return;
        }
        self.scan_status = ScanStatus::Scanning;
        self.current_operation = Some(OperationState::Scanning);
        let service = Arc::clone(&self.service);
        let (tx, rx) = mpsc::channel();
        self.scan_result_receiver = Some(rx);
        tokio::task::spawn_blocking(move || {
            let rt = tokio::runtime::Runtime::new().unwrap();
            let result = rt.block_on(async {
                let svc = service.lock().await;
                svc.scan_all_apps().await
            });
            let _ = tx.send(match result {
                Ok(a) => Ok(a),
                Err(e) => Err(e.to_string()),
            });
        });
    }

    pub fn set_apps(&mut self, apps: Vec<InstalledApp>) {
        let count = apps.len();
        self.apps = apps;
        self.apply_filter();
        self.selected_app_index = 0;
        self.selected_apps.clear();
        self.scroll_offset = 0;
        self.scan_status = ScanStatus::Complete(count);
        self.current_operation = None;
    }

    pub fn set_scan_error(&mut self, error: String) {
        self.scan_status = ScanStatus::Error(error);
        self.current_operation = None;
    }

    /// Apply the current search query to filter the app list.
    fn apply_filter(&mut self) {
        if self.search_query.is_empty() {
            self.filtered_apps = self.apps.clone();
        } else {
            let query = self.search_query.clone();
            let matcher = &self.fuzzy_matcher;
            self.filtered_apps = self
                .apps
                .iter()
                .filter(|app| {
                    matcher.fuzzy_match(&app.name, &query).is_some()
                        || app.name.to_lowercase().contains(&query.to_lowercase())
                        || app
                            .publisher
                            .as_ref()
                            .map(|p| p.to_lowercase().contains(&query.to_lowercase()))
                            .unwrap_or(false)
                })
                .cloned()
                .collect();
        }
    }

    /// Start or cancel search mode.
    pub fn toggle_search(&mut self) {
        self.search_mode = !self.search_mode;
        if !self.search_mode {
            self.search_query.clear();
            self.apply_filter();
        }
    }

    /// Type a character into the search box.
    pub fn search_input(&mut self, ch: char) {
        self.search_query.push(ch);
        self.apply_filter();
    }

    /// Delete the last character in the search query.
    pub fn search_backspace(&mut self) {
        self.search_query.pop();
        self.apply_filter();
    }

    pub fn search_query(&self) -> &str {
        &self.search_query
    }

    pub fn is_search_mode(&self) -> bool {
        self.search_mode
    }

    // Accessors
    pub fn selected_app(&self) -> Option<&InstalledApp> {
        self.filtered_apps.get(self.selected_app_index)
    }
    pub fn theme(&self) -> &TuiTheme {
        &self.theme
    }
    pub fn is_showing_details(&self) -> bool {
        self.show_details
    }
    pub fn is_showing_help(&self) -> bool {
        self.show_help
    }
    pub fn scan_status(&self) -> &ScanStatus {
        &self.scan_status
    }
    pub fn current_operation(&self) -> Option<&OperationState> {
        self.current_operation.as_ref()
    }
    pub fn status_message(&self) -> Option<&(String, bool)> {
        self.status_message.as_ref()
    }
    pub fn get_filtered_apps(&self) -> &[InstalledApp] {
        &self.filtered_apps
    }
    pub fn get_all_apps(&self) -> &[InstalledApp] {
        &self.apps
    }
    pub fn get_selected_index(&self) -> usize {
        self.selected_app_index
    }
    pub fn get_selected_apps(&self) -> &[usize] {
        &self.selected_apps
    }
    pub fn scroll_offset(&self) -> usize {
        self.scroll_offset
    }
    pub fn show_context_menu(&self) -> bool {
        self.show_context_menu
    }
    pub fn context_menu_index(&self) -> usize {
        self.context_menu_index
    }
    pub fn context_menu_y(&self) -> u16 {
        self.context_menu_y
    }
    pub fn stats(&self) -> Option<&SystemStats> {
        self.stats.as_ref()
    }

    /// Live process usage for an app, matched by exe path or install location.
    /// On non-Windows platforms this always returns None (no process data).
    #[cfg(all(target_os = "windows", feature = "windows"))]
    pub fn process_for(&self, app: &InstalledApp) -> Option<&ProcessUsage> {
        let stats = self.stats.as_ref()?;
        // 1. Exact match on exe_path metadata
        if let Some(exe) = app.metadata.get("exe_path") {
            let key = exe.trim_matches('"').to_lowercase();
            if let Some(p) = stats.processes.get(&key) {
                return Some(p);
            }
        }
        // 2. Match by install_location parent directory
        if let Some(loc) = &app.install_location {
            let loc_lower = loc.to_string_lossy().to_lowercase();
            if let Some((_, p)) = stats
                .processes
                .iter()
                .find(|(k, _)| k.starts_with(&loc_lower))
            {
                return Some(p);
            }
        }
        // 3. Last resort: match by app name in the exe path
        let name_lower = app.name.to_lowercase();
        stats
            .processes
            .iter()
            .find(|(_k, p)| {
                p.name.to_lowercase().contains(&name_lower)
                    || name_lower.contains(&p.name.trim_end_matches(".exe").to_lowercase())
            })
            .map(|(_, p)| p)
    }

    /// Non-Windows stub: no process usage data available.
    #[cfg(not(all(target_os = "windows", feature = "windows")))]
    pub fn process_for(&self, _app: &InstalledApp) -> Option<&ProcessUsage> {
        None
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Action {
    ViewDetails,
    Uninstall,
    ForceRemove,
    ScanLeftovers,
    AddToBatch,
    Cancel,
}

pub fn context_menu_items() -> &'static [Action] {
    &[
        Action::ViewDetails,
        Action::Uninstall,
        Action::ForceRemove,
        Action::ScanLeftovers,
        Action::AddToBatch,
        Action::Cancel,
    ]
}

/// Returns a Unicode icon based on the app's install source
pub fn app_icon(app: &InstalledApp) -> &'static str {
    match &app.source {
        InstallSource::WindowsStore { .. } => "\u{1F4E6}", // 📦
        InstallSource::Registry { .. } => "\u{1F4BB}",     // 💻
        InstallSource::Portable { .. } => "\u{1F50D}",     // 🔍
        InstallSource::BrowserExtension { browser, .. } => match browser {
            greek_common::BrowserType::Chrome => "\u{1F310}", // 🌐
            greek_common::BrowserType::Firefox => "\u{1F525}", // 🔥
            greek_common::BrowserType::Edge => "\u{1F310}",   // 🌐
            _ => "\u{1F517}",                                 // 🔗
        },
        InstallSource::WindowsFeature { .. } => "\u{2699}", // ⚙
        InstallSource::PackageManager { .. } => "\u{1F4E6}", // 📦
    }
}

/// Returns a colored per-app avatar: an initial letter tinted with the app's
/// real icon dominant color (extracted from the exe). Falls back to a
/// deterministic palette color when no icon is available.
pub fn app_avatar(app: &InstalledApp) -> (String, Color) {
    let letter = app
        .name
        .chars()
        .next()
        .map(|c| c.to_uppercase().next().unwrap_or('?'))
        .filter(|c| c.is_ascii() && c.is_alphabetic())
        .unwrap_or('•')
        .to_string();

    let color = app
        .metadata
        .get("icon_color")
        .and_then(|s| {
            let mut it = s.split(',');
            let r = it.next()?.trim().parse::<u8>().ok()?;
            let g = it.next()?.trim().parse::<u8>().ok()?;
            let b = it.next()?.trim().parse::<u8>().ok()?;
            Some(Color::Rgb(r, g, b))
        })
        .unwrap_or_else(|| palette_color(&app.name));

    (letter, color)
}

fn palette_color(name: &str) -> Color {
    const PALETTE: [Color; 8] = [
        Color::Rgb(59, 130, 246),
        Color::Rgb(239, 68, 68),
        Color::Rgb(16, 185, 129),
        Color::Rgb(245, 158, 11),
        Color::Rgb(139, 92, 246),
        Color::Rgb(236, 72, 153),
        Color::Rgb(14, 165, 233),
        Color::Rgb(132, 204, 22),
    ];
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut h = DefaultHasher::new();
    name.hash(&mut h);
    PALETTE[(h.finish() as usize) % PALETTE.len()]
}

/// Decode the 8x8 RGBA pixel buffer of the app's real icon (64 * [r,g,b,a]).
pub fn app_icon_pixels(app: &InstalledApp) -> Option<Vec<[u8; 4]>> {
    let b64 = app.metadata.get("icon_rgba")?;
    use base64::Engine;
    let raw = base64::engine::general_purpose::STANDARD.decode(b64).ok()?;
    if raw.len() != 256 {
        return None;
    }
    let mut px = Vec::with_capacity(64);
    for i in (0..256).step_by(4) {
        px.push([raw[i], raw[i + 1], raw[i + 2], raw[i + 3]]);
    }
    Some(px)
}

/// Color of a single pixel of the app icon (transparent → Reset).
pub fn app_icon_pixel_color(px: &[[u8; 4]], x: usize, y: usize) -> Color {
    let p = px[y * 8 + x];
    if p[3] < 40 {
        Color::Reset
    } else {
        Color::Rgb(p[0], p[1], p[2])
    }
}

/// Returns a simple ASCII icon for the app source (for terminals that don't support Unicode)
pub fn app_icon_simple(app: &InstalledApp) -> &'static str {
    match &app.source {
        InstallSource::WindowsStore { .. } => "[S]",
        InstallSource::Registry { .. } => "[R]",
        InstallSource::Portable { .. } => "[P]",
        InstallSource::BrowserExtension { .. } => "[B]",
        InstallSource::WindowsFeature { .. } => "[F]",
        InstallSource::PackageManager { .. } => "[M]",
    }
}
