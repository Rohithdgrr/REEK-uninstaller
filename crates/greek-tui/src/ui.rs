// UI rendering for the TUI

use crate::app::{
    app_avatar, app_icon_pixel_color, app_icon_pixels, context_menu_items, OperationState,
    ScanStatus, TuiApp,
};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Wrap},
    Frame,
};

pub fn render(f: &mut Frame, app: &TuiApp) {
    let area = f.area();

    let main_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Header
            Constraint::Min(0),    // Content
            Constraint::Length(3), // Footer
        ])
        .split(area);

    render_header(f, main_chunks[0], app);
    render_content(f, main_chunks[1], app);
    render_footer(f, main_chunks[2], app);

    if app.is_showing_help() {
        render_help(f);
    }
    if app.show_context_menu() {
        render_context_menu(f, app);
    }
    if let Some(op) = app.current_operation() {
        match op {
            OperationState::Scanning => {
                render_overlay(f, "Scanning...", "Reading Windows Registry...")
            }
            OperationState::Uninstalling(name) => {
                render_overlay(f, "Uninstalling...", &format!("Removing {}...", name))
            }
            OperationState::ForceRemoving(name) => {
                render_overlay(f, "Force Removing...", &format!("Deleting {}...", name))
            }
            OperationState::AnalyzingLeftovers(name) => render_overlay(
                f,
                "Analyzing Leftovers...",
                &format!("Scanning for {} leftovers...", name),
            ),
            OperationState::AddingToBatch(name) => {
                render_overlay(f, "Adding to Batch...", &format!("Adding {}...", name))
            }
        }
    }
}

// ── Header ───────────────────────────────────────────────────────────────
fn render_header(f: &mut Frame, area: Rect, app: &TuiApp) {
    let bg = Color::Rgb(230, 240, 252);
    let border_color = Color::Rgb(180, 195, 220);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color))
        .style(Style::default().bg(bg));
    let inner = block.inner(area);
    f.render_widget(block, area);

    let total = app.get_all_apps().len();
    let selected = app.get_selected_apps().len();

    let mut spans = vec![
        Span::styled(
            "  REEK ",
            Style::default()
                .fg(Color::Rgb(30, 64, 175))
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            "Uninstaller  ",
            Style::default().fg(Color::Rgb(55, 100, 200)),
        ),
        Span::styled(
            format!("{} apps", total),
            Style::default()
                .fg(Color::Rgb(30, 64, 175))
                .add_modifier(Modifier::BOLD),
        ),
    ];

    if selected > 0 {
        spans.push(Span::styled(
            format!("  |  {} selected", selected),
            Style::default().fg(Color::Rgb(16, 185, 129)),
        ));
    }

    spans.push(Span::raw("  "));

    // Show status message if present
    if let Some((msg, is_err)) = app.status_message() {
        let color = if *is_err {
            Color::Rgb(239, 68, 68)
        } else {
            Color::Rgb(16, 185, 129)
        };
        spans.push(Span::styled(
            format!("  {}  ", msg),
            Style::default().fg(color),
        ));
    }

    f.render_widget(
        Paragraph::new(Line::from(spans)).style(Style::default().bg(bg)),
        inner,
    );
}

// ── Content ──────────────────────────────────────────────────────────────
fn render_content(f: &mut Frame, area: Rect, app: &TuiApp) {
    if app.is_showing_details() {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(55), Constraint::Percentage(45)])
            .split(area);
        render_app_list(f, chunks[0], app);
        render_details(f, chunks[1], app);
    } else {
        render_app_list(f, area, app);
    }
}

// ── App list ─────────────────────────────────────────────────────────────
fn render_app_list(f: &mut Frame, area: Rect, app: &TuiApp) {
    let border = Color::Rgb(200, 210, 225);
    let header_bg = Color::Rgb(225, 232, 242);
    let header_fg = Color::Rgb(55, 65, 85);

    // Outer block with title
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border))
        .title(format!(" {} apps ", app.get_filtered_apps().len()))
        .title_style(
            Style::default()
                .fg(Color::Rgb(59, 130, 246))
                .add_modifier(Modifier::BOLD),
        );
    let inner = block.inner(area);
    f.render_widget(block, area);

    // Table header row
    let header_h = 1u16;
    let header_area = Rect::new(inner.x, inner.y, inner.width, header_h);
    let col_widths = compute_column_widths(inner.width);

    let hdr_style = Style::default()
        .fg(header_fg)
        .bg(header_bg)
        .add_modifier(Modifier::BOLD);
    let headers = [
        "",
        "Icon",
        "Name",
        "Version",
        "Publisher",
        "Size",
        "CPU",
        "RAM",
    ];
    let aligns = [
        Alignment::Left,
        Alignment::Center,
        Alignment::Left,
        Alignment::Left,
        Alignment::Left,
        Alignment::Right,
        Alignment::Right,
        Alignment::Right,
    ];

    let mut x = inner.x;
    for (i, title) in headers.iter().enumerate() {
        let w = col_widths[i];
        let cell = Rect::new(x, header_area.y, w, header_h);
        f.render_widget(
            Paragraph::new(*title).style(hdr_style).alignment(aligns[i]),
            cell,
        );
        x += w;
    }

    // List body
    let body_area = Rect::new(
        inner.x,
        inner.y + header_h,
        inner.width,
        inner.height.saturating_sub(header_h),
    );
    let filtered = app.get_filtered_apps();
    let sel = app.get_selected_index();
    let checked = app.get_selected_apps();
    let scroll = app.scroll_offset();
    let visible_h = body_area.height as usize;

    if filtered.is_empty() {
        let msg = if app.get_all_apps().is_empty() {
            "No applications found. Press [R] to scan."
        } else {
            "No applications match."
        };
        f.render_widget(
            Paragraph::new(msg)
                .alignment(Alignment::Center)
                .style(Style::default().fg(Color::Rgb(148, 163, 184))),
            body_area,
        );
        return;
    }

    let items: Vec<ListItem> = filtered
        .iter()
        .enumerate()
        .skip(scroll)
        .take(visible_h)
        .map(|(i, app_item)| {
            let is_sel = i == sel;
            let is_chk = checked.contains(&i);

            let (row_bg, row_fg) = if is_sel {
                (Color::Rgb(59, 130, 246), Color::White)
            } else if i % 2 == 0 {
                (Color::Rgb(248, 250, 252), Color::Rgb(44, 62, 80))
            } else {
                (Color::Rgb(255, 255, 255), Color::Rgb(44, 62, 80))
            };

            let muted = if is_sel {
                Color::Rgb(200, 215, 240)
            } else {
                Color::Rgb(120, 130, 150)
            };
            let bg = row_bg;

            let (avatar, avatar_color) = app_avatar(app_item);
            let px = app_icon_pixels(app_item);
            let check = if is_chk { "x" } else { " " };
            let name = &app_item.name;
            let ver = app_item.version.as_deref().unwrap_or("");
            let pub_str = app_item.publisher.as_deref().unwrap_or("");
            let size = app_item.display_size();

            let name_trunc = truncate(name, col_widths[2] as usize - 1);
            let pub_trunc = truncate(pub_str, col_widths[4] as usize - 1);

            let mut spans: Vec<Span> = Vec::new();
            spans.push(Span::styled(
                format!("{} ", check),
                Style::default()
                    .fg(if is_chk {
                        Color::Rgb(16, 185, 129)
                    } else {
                        muted
                    })
                    .bg(bg),
            ));

            // Real icon: 8 half-block pixels (8x2 of the actual icon), else colored letter
            if let Some(px) = &px {
                for x in 0..8 {
                    spans.push(Span::styled(
                        "▀",
                        Style::default()
                            .fg(app_icon_pixel_color(px, x, 0))
                            .bg(app_icon_pixel_color(px, x, 1)),
                    ));
                }
                spans.push(Span::raw(" "));
            } else {
                spans.push(Span::styled(
                    format!(" {} ", avatar),
                    Style::default()
                        .fg(if is_sel { Color::White } else { avatar_color })
                        .add_modifier(Modifier::BOLD)
                        .bg(bg),
                ));
                spans.push(Span::raw(
                    " ".repeat(col_widths[1].saturating_sub(4) as usize),
                ));
            }

            spans.push(Span::styled(
                format!("{:<width$}", name_trunc, width = col_widths[2] as usize - 1),
                Style::default().fg(row_fg).bg(bg),
            ));
            spans.push(Span::styled(
                format!("{:<width$}", ver, width = col_widths[3] as usize),
                Style::default().fg(muted).bg(bg),
            ));
            spans.push(Span::styled(
                format!("{:<width$}", pub_trunc, width = col_widths[4] as usize),
                Style::default().fg(muted).bg(bg),
            ));
            spans.push(Span::styled(
                format!("{:>width$}", size, width = col_widths[5] as usize),
                Style::default().fg(muted).bg(bg),
            ));

            // Live CPU / RAM for the app's process (if running)
            let live = app.process_for(app_item);
            let cpu = live
                .map(|p| format!("{:.0}%", p.cpu_usage))
                .unwrap_or_else(|| "-".into());
            let ram = live
                .map(|p| fmt_bytes(p.memory_bytes))
                .unwrap_or_else(|| "-".into());
            let live_color = if is_sel {
                Color::Rgb(200, 215, 240)
            } else {
                Color::Rgb(100, 116, 139)
            };
            let running = live.is_some();
            let cpu_color = if running {
                bar_color(live.map(|p| p.cpu_usage).unwrap_or(0.0))
            } else {
                live_color
            };
            spans.push(Span::styled(
                format!("{:>width$}", cpu, width = col_widths[6] as usize),
                Style::default().fg(cpu_color).bg(bg),
            ));
            spans.push(Span::styled(
                format!("{:>width$}", ram, width = col_widths[7] as usize),
                Style::default().fg(live_color).bg(bg),
            ));

            ListItem::new(Line::from(spans)).style(Style::default().bg(bg))
        })
        .collect();

    f.render_widget(List::new(items), body_area);
}

fn compute_column_widths(total_width: u16) -> [u16; 8] {
    // check(2) + icon(10) + name(26%) + version(11%) + publisher(24%) + size + cpu(6) + ram(8)
    let w = total_width;
    let name = (w as f64 * 0.26) as u16;
    let ver = (w as f64 * 0.11) as u16;
    let pub_ = (w as f64 * 0.24) as u16;
    let sz = w.saturating_sub(2 + 10 + name + ver + pub_ + 6 + 8);
    [2, 10, name, ver, pub_, sz.max(8), 6, 8]
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else if max > 3 {
        format!("{}...", &s[..max - 3])
    } else {
        s[..max].to_string()
    }
}

// ── Details panel ────────────────────────────────────────────────────────
fn render_details(f: &mut Frame, area: Rect, app: &TuiApp) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Rgb(200, 210, 225)))
        .title(" Details ")
        .title_style(
            Style::default()
                .fg(Color::Rgb(59, 130, 246))
                .add_modifier(Modifier::BOLD),
        )
        .style(Style::default().bg(Color::Rgb(250, 252, 255)));
    let inner = block.inner(area);
    f.render_widget(block, area);

    if let Some(a) = app.selected_app() {
        let label = Style::default().fg(Color::Rgb(100, 116, 139));
        let val = Style::default().fg(Color::Rgb(30, 41, 59));
        let val_b = val.add_modifier(Modifier::BOLD);
        let accent = Style::default().fg(Color::Rgb(59, 130, 246));

        let mut lines: Vec<Line> = Vec::new();

        // Avatar + Name
        let (avatar, avatar_color) = app_avatar(a);
        lines.push(Line::from(""));
        lines.push(Line::from(vec![
            Span::styled(
                format!("  {} ", avatar),
                Style::default()
                    .fg(avatar_color)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(&a.name, val_b),
        ]));
        lines.push(Line::from(""));

        // Full 8x8 real icon rendered with half-blocks (4 rows of 8)
        if let Some(px) = app_icon_pixels(a) {
            let detail_bg = Color::Rgb(250, 252, 255);
            for row in 0..4 {
                let mut spans = vec![Span::raw("    ")];
                for x in 0..8 {
                    let top = app_icon_pixel_color(&px, x, row * 2);
                    let bot = app_icon_pixel_color(&px, x, row * 2 + 1);
                    let top = if top == Color::Reset { detail_bg } else { top };
                    let bot = if bot == Color::Reset { detail_bg } else { bot };
                    spans.push(Span::styled("▀", Style::default().fg(top).bg(bot)));
                }
                lines.push(Line::from(spans));
            }
            lines.push(Line::from(""));
        }

        detail_row(
            &mut lines,
            "Publisher",
            a.publisher.as_deref().unwrap_or("-"),
            &label,
            &val,
        );
        detail_row(
            &mut lines,
            "Version",
            a.version.as_deref().unwrap_or("-"),
            &label,
            &val,
        );
        detail_row(&mut lines, "Size", &a.display_size(), &label, &val);
        detail_row(
            &mut lines,
            "Date",
            &a.install_date
                .map(|d| d.to_string())
                .unwrap_or_else(|| "-".into()),
            &label,
            &val,
        );
        detail_row(
            &mut lines,
            "Source",
            &format_source(&a.source),
            &label,
            &val,
        );

        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled("  Install Location", label)));
        let loc = a
            .install_location
            .as_ref()
            .and_then(|p| p.to_str())
            .unwrap_or("-");
        lines.push(Line::from(Span::styled(
            format!("    {}", truncate(loc, inner.width as usize - 6)),
            val,
        )));

        if let Some(us) = &a.uninstall_string {
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled("  Uninstall Command", label)));
            lines.push(Line::from(Span::styled(
                format!("    {}", truncate(us, inner.width as usize - 6)),
                val,
            )));
        }

        if let Some(us) = &a.quiet_uninstall_string {
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled("  Silent Uninstall", label)));
            lines.push(Line::from(Span::styled(
                format!("    {}", truncate(us, inner.width as usize - 6)),
                val,
            )));
        }

        if let Some(kp) = a.metadata.get("registry_key") {
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled("  Registry Key", label)));
            let full = format!(
                "{}\\{}",
                a.metadata.get("hive").map(|s| s.as_str()).unwrap_or("HKLM"),
                kp
            );
            lines.push(Line::from(Span::styled(
                format!("    {}", truncate(&full, inner.width as usize - 6)),
                val,
            )));
        }

        if let Some(ip) = &a.icon_path {
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled("  Icon", label)));
            lines.push(Line::from(Span::styled(
                format!(
                    "    {}",
                    truncate(&ip.to_string_lossy(), inner.width as usize - 6)
                ),
                val,
            )));
        }

        // Live process resources
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "  Process",
            accent.add_modifier(Modifier::BOLD),
        )));
        match app.process_for(a) {
            Some(p) => {
                let running = p.cpu_usage > 0.0 || p.memory_bytes > 0;
                let state_color = if running {
                    Color::Rgb(16, 185, 129)
                } else {
                    Color::Rgb(148, 163, 184)
                };
                lines.push(Line::from(vec![
                    Span::styled(format!("  {:<11}", "State"), label),
                    Span::styled(
                        if running { "● Running" } else { "○ Idle" },
                        Style::default()
                            .fg(state_color)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(format!("  {} (PID {})", p.name, p.pid), val),
                ]));

                let mut cpu_spans = vec![
                    Span::styled(format!("  {:<11}", "CPU"), label),
                    Span::styled(format!("{:.1}% ", p.cpu_usage), val),
                ];
                cpu_spans.extend(stat_bar(
                    p.cpu_usage,
                    bar_color(p.cpu_usage),
                    Color::Rgb(226, 232, 240),
                    4,
                ));
                lines.push(Line::from(cpu_spans));

                let ram_pct = pct(
                    p.memory_bytes,
                    app.stats().map(|s| s.ram_total_bytes).unwrap_or(0),
                );
                let mut ram_spans = vec![
                    Span::styled(format!("  {:<11}", "RAM"), label),
                    Span::styled(format!("{} ", fmt_bytes(p.memory_bytes)), val),
                    Span::styled(format!("{:.1}%", ram_pct), muted_style()),
                ];
                ram_spans.extend(stat_bar(
                    ram_pct,
                    bar_color(ram_pct),
                    Color::Rgb(226, 232, 240),
                    4,
                ));
                lines.push(Line::from(ram_spans));

                if p.vram_bytes > 0 {
                    let total_vram = app
                        .stats()
                        .and_then(|s| s.gpu.as_ref())
                        .map(|g| g.vram_total_bytes)
                        .unwrap_or(0);
                    let vram_pct = pct(p.vram_bytes, total_vram);
                    let mut vram_spans = vec![
                        Span::styled(format!("  {:<11}", "VRAM"), label),
                        Span::styled(format!("{} ", fmt_bytes(p.vram_bytes)), val),
                    ];
                    if total_vram > 0 {
                        vram_spans.push(Span::styled(format!("{:.1}%", vram_pct), muted_style()));
                    }
                    vram_spans.extend(stat_bar(
                        vram_pct,
                        bar_color(vram_pct),
                        Color::Rgb(226, 232, 240),
                        4,
                    ));
                    lines.push(Line::from(vram_spans));
                }

                if p.gpu_usage_pct > 0.0 {
                    let mut gpu_spans = vec![
                        Span::styled(format!("  {:<11}", "GPU"), label),
                        Span::styled(format!("{:.1}% ", p.gpu_usage_pct), val),
                    ];
                    gpu_spans.extend(stat_bar(
                        p.gpu_usage_pct,
                        bar_color(p.gpu_usage_pct),
                        Color::Rgb(226, 232, 240),
                        4,
                    ));
                    lines.push(Line::from(gpu_spans));
                }

                let vm = if p.virtual_memory > 0 {
                    format!(", VM {}", fmt_bytes(p.virtual_memory))
                } else {
                    String::new()
                };
                detail_row(
                    &mut lines,
                    "Mem (virt)",
                    &format!("{} threads{}", p.threads, vm),
                    &label,
                    &val,
                );
                detail_row(
                    &mut lines,
                    "Disk I/O",
                    &format!(
                        "↑ {} ↓ {}",
                        fmt_bytes(p.read_bytes),
                        fmt_bytes(p.written_bytes)
                    ),
                    &label,
                    &val,
                );
                detail_row(
                    &mut lines,
                    "Started",
                    &format_started(p.started_at, p.run_time_secs),
                    &label,
                    &val,
                );
            }
            None => {
                lines.push(Line::from(vec![
                    Span::styled(format!("  {:<11}", "State"), label),
                    Span::styled(
                        "○ Not running",
                        Style::default().fg(Color::Rgb(148, 163, 184)),
                    ),
                ]));
            }
        }

        // Actions
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "  Actions",
            accent.add_modifier(Modifier::BOLD),
        )));
        lines.push(Line::from(""));

        lines.push(action_line(
            "[U] ",
            "Uninstall",
            Color::Rgb(59, 130, 246),
            &val,
        ));
        lines.push(action_line(
            "[F] ",
            "Force Remove",
            Color::Rgb(239, 68, 68),
            &val,
        ));
        lines.push(action_line(
            "[L] ",
            "Scan Leftovers",
            Color::Rgb(16, 185, 129),
            &val,
        ));
        lines.push(action_line(
            "[B] ",
            "Add to Batch",
            Color::Rgb(245, 158, 11),
            &val,
        ));

        f.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), inner);
    } else {
        f.render_widget(
            Paragraph::new("No application selected\n\n[Up/Down] Navigate\n[Enter] Toggle details\n[M] Context menu")
                .alignment(Alignment::Center)
                .style(Style::default().fg(Color::Rgb(148, 163, 184))),
            inner,
        );
    }
}

fn detail_row(lines: &mut Vec<Line>, label: &str, value: &str, ls: &Style, vs: &Style) {
    lines.push(Line::from(vec![
        Span::styled(format!("  {:<11}", label), *ls),
        Span::styled(value.to_string(), *vs),
    ]));
}

fn action_line(key: &str, label: &str, color: Color, base: &Style) -> Line<'static> {
    Line::from(vec![
        Span::styled(
            format!("    {}", key),
            Style::default().fg(color).add_modifier(Modifier::BOLD),
        ),
        Span::styled(label.to_string(), *base),
    ])
}

fn format_source(source: &greek_common::InstallSource) -> String {
    match source {
        greek_common::InstallSource::Registry { hive, .. } => {
            format!("Registry ({})", hive.as_str())
        }
        greek_common::InstallSource::WindowsStore { .. } => "Windows Store".into(),
        greek_common::InstallSource::Portable { .. } => "Portable".into(),
        greek_common::InstallSource::BrowserExtension { browser, .. } => format!("{:?}", browser),
        greek_common::InstallSource::WindowsFeature { .. } => "Windows Feature".into(),
        greek_common::InstallSource::PackageManager { manager, .. } => format!("{:?}", manager),
    }
}

// ── Footer ───────────────────────────────────────────────────────────────
fn render_footer(f: &mut Frame, area: Rect, app: &TuiApp) {
    let bg = Color::Rgb(245, 247, 250);
    let block = Block::default()
        .borders(Borders::TOP)
        .border_style(Style::default().fg(Color::Rgb(200, 210, 225)))
        .style(Style::default().bg(bg));
    let inner = block.inner(area);
    f.render_widget(block, area);

    let icon = Color::Rgb(59, 130, 246);
    let text = Color::Rgb(75, 85, 99);
    let muted = Color::Rgb(148, 163, 184);
    let bar_empty = Color::Rgb(226, 232, 240);

    let stats_line = match app.stats() {
        Some(s) => {
            let mut spans: Vec<Span> = Vec::new();
            spans.push(Span::styled("  ", Style::default()));

            // CPU
            spans.push(Span::styled(
                "⚙ ",
                Style::default().fg(icon).add_modifier(Modifier::BOLD),
            ));
            spans.push(Span::styled(
                format!("{:.0}% ", s.cpu_usage),
                Style::default().fg(text),
            ));
            spans.extend(stat_bar(s.cpu_usage, bar_color(s.cpu_usage), bar_empty, 4));
            spans.push(sep());

            // RAM
            spans.push(Span::styled(
                "▮ ",
                Style::default().fg(icon).add_modifier(Modifier::BOLD),
            ));
            let ram_pct = pct(s.ram_used_bytes, s.ram_total_bytes);
            spans.push(Span::styled(
                format!(
                    "{}/{} ",
                    fmt_bytes(s.ram_used_bytes),
                    fmt_bytes(s.ram_total_bytes)
                ),
                Style::default().fg(text),
            ));
            spans.extend(stat_bar(ram_pct, bar_color(ram_pct), bar_empty, 4));

            // Swap
            if s.swap_total_bytes > 0 {
                spans.push(sep());
                spans.push(Span::styled(
                    "▩ ",
                    Style::default().fg(icon).add_modifier(Modifier::BOLD),
                ));
                spans.push(Span::styled(
                    format!(
                        "{}/{} ",
                        fmt_bytes(s.swap_used_bytes),
                        fmt_bytes(s.swap_total_bytes)
                    ),
                    Style::default().fg(text),
                ));
            }

            // Disks (up to 2)
            for d in s.disks.iter().take(2) {
                spans.push(sep());
                let p = d.usage_pct();
                spans.push(Span::styled(
                    format!("{} ", d.label),
                    Style::default().fg(icon).add_modifier(Modifier::BOLD),
                ));
                spans.push(Span::styled(
                    format!("{:.0}% ", p),
                    Style::default().fg(text),
                ));
                spans.extend(stat_bar(p, bar_color(p), bar_empty, 4));
            }

            // GPU + VRAM
            if let Some(g) = &s.gpu {
                spans.push(sep());
                spans.push(Span::styled(
                    "◉ ",
                    Style::default().fg(icon).add_modifier(Modifier::BOLD),
                ));
                spans.push(Span::styled(
                    format!("{:.0}% ", g.usage_pct),
                    Style::default().fg(text),
                ));
                spans.extend(stat_bar(g.usage_pct, bar_color(g.usage_pct), bar_empty, 4));
                if g.vram_total_bytes > 0 {
                    spans.push(Span::styled(
                        "◆ ",
                        Style::default().fg(icon).add_modifier(Modifier::BOLD),
                    ));
                    spans.push(Span::styled(
                        format!(
                            "{}/{} ",
                            fmt_bytes(g.vram_used_bytes),
                            fmt_bytes(g.vram_total_bytes)
                        ),
                        Style::default().fg(text),
                    ));
                }
            }

            // Battery
            if let Some(b) = &s.battery {
                spans.push(sep());
                spans.push(Span::styled(
                    "◫ ",
                    Style::default().fg(icon).add_modifier(Modifier::BOLD),
                ));
                spans.push(Span::styled(
                    format!("{:.0}%{} ", b.percent, if b.charging { "+" } else { "" }),
                    Style::default().fg(text),
                ));
            }

            // Uptime
            spans.push(sep());
            spans.push(Span::styled(
                "◷ ",
                Style::default().fg(icon).add_modifier(Modifier::BOLD),
            ));
            spans.push(Span::styled(
                fmt_uptime(s.uptime_secs),
                Style::default().fg(text),
            ));
            spans.push(Span::styled("  ", Style::default()));

            spans
        }
        None => vec![Span::styled(
            "  Gathering system stats...",
            Style::default().fg(muted),
        )],
    };

    let sel_count = app.get_selected_apps().len();
    let status = match app.scan_status() {
        ScanStatus::Scanning => " Scanning... ".into(),
        ScanStatus::Complete(n) => format!(" {} apps ", n),
        ScanStatus::Error(e) => format!(" {} ", e),
        ScanStatus::Idle => " Ready ".into(),
    };

    let mut key_spans = vec![
        Span::styled(
            &status,
            Style::default()
                .fg(Color::Rgb(59, 130, 246))
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw("  "),
        Span::styled(
            "[U]ninstall ",
            Style::default().fg(Color::Rgb(59, 130, 246)),
        ),
        force_span(),
        Span::styled(
            "[L]eftovers ",
            Style::default().fg(Color::Rgb(16, 185, 129)),
        ),
        Span::styled("[B]atch ", Style::default().fg(Color::Rgb(245, 158, 11))),
        Span::raw("  "),
        Span::styled("[M]enu ", Style::default().fg(Color::Rgb(100, 116, 139))),
        Span::styled("[R]escan ", Style::default().fg(Color::Rgb(100, 116, 139))),
        Span::styled("[?]Help ", Style::default().fg(Color::Rgb(100, 116, 139))),
    ];

    if sel_count > 0 {
        key_spans.push(Span::styled(
            format!(" {} selected ", sel_count),
            Style::default().fg(Color::Rgb(16, 185, 129)),
        ));
    }

    let line1 = Line::from(stats_line);
    let line2 = Line::from(key_spans);
    f.render_widget(
        Paragraph::new(vec![line1, line2]).style(Style::default().bg(bg)),
        inner,
    );
}

fn sep() -> Span<'static> {
    Span::styled("│ ", Color::Rgb(203, 213, 225))
}

/// Whether force-remove is available. On Windows this requires an elevated
/// process; the footer shows the shield lock warning otherwise.
fn force_span() -> Span<'static> {
    #[cfg(all(target_os = "windows", feature = "windows"))]
    {
        if greek_windows::is_elevated() {
            Span::styled("[F]orce ", Style::default().fg(Color::Rgb(239, 68, 68)))
        } else {
            Span::styled("[F]orce (admin) ", Color::Rgb(251, 146, 60))
        }
    }
    #[cfg(not(all(target_os = "windows", feature = "windows")))]
    {
        Span::styled("[F]orce ", Style::default().fg(Color::Rgb(239, 68, 68)))
    }
}

/// Color for a usage percentage: green < 50%, amber < 80%, red above.
fn bar_color(pct: f32) -> Color {
    if pct < 50.0 {
        Color::Rgb(16, 185, 129)
    } else if pct < 80.0 {
        Color::Rgb(245, 158, 11)
    } else {
        Color::Rgb(239, 68, 68)
    }
}

/// Filled/unfilled bar segments, e.g. 4 segments for 62% -> "███░".
fn stat_bar(pct: f32, fill: Color, empty: Color, width: usize) -> Vec<Span<'static>> {
    let filled = ((pct / 100.0) * width as f32)
        .round()
        .clamp(0.0, width as f32) as usize;
    let mut spans = Vec::with_capacity(width);
    for i in 0..width {
        spans.push(Span::styled(
            "█",
            if i < filled {
                Style::default().fg(fill)
            } else {
                Style::default().fg(empty)
            },
        ));
    }
    spans.push(Span::raw(" "));
    spans
}

fn pct(used: u64, total: u64) -> f32 {
    if total == 0 {
        0.0
    } else {
        (used as f32 / total as f32) * 100.0
    }
}

/// Compact byte size: "6.2G", "512M", "18K".
fn fmt_bytes(bytes: u64) -> String {
    const GB: u64 = 1024 * 1024 * 1024;
    const MB: u64 = 1024 * 1024;
    const KB: u64 = 1024;
    if bytes >= GB {
        let g = bytes as f64 / GB as f64;
        if g < 10.0 {
            format!("{:.1}G", g)
        } else {
            format!("{:.0}G", g)
        }
    } else if bytes >= MB {
        format!("{:.0}M", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.0}K", bytes as f64 / KB as f64)
    } else {
        format!("{}B", bytes)
    }
}

/// Uptime as "3h12m", "1d4h" or "9m".
fn fmt_uptime(secs: u64) -> String {
    let d = secs / 86400;
    let h = (secs % 86400) / 3600;
    let m = (secs % 3600) / 60;
    if d > 0 {
        format!("{}d{}h", d, h)
    } else if h > 0 {
        format!("{}h{}m", h, m)
    } else {
        format!("{}m", m)
    }
}

fn muted_style() -> Style {
    Style::default().fg(Color::Rgb(148, 163, 184))
}

/// "14 Mar 14:32:05 · up 3h12m" from process start time (unix secs) and run time.
fn format_started(started_at: Option<u64>, run_time_secs: u64) -> String {
    match started_at {
        Some(ts) => {
            let when = chrono::DateTime::from_timestamp(ts as i64, 0)
                .map(|u| {
                    u.with_timezone(&chrono::Local)
                        .format("%d %b %H:%M:%S")
                        .to_string()
                })
                .unwrap_or_else(|| "-".into());
            format!("{} · up {}", when, fmt_uptime(run_time_secs))
        }
        None => "-".into(),
    }
}

// ── Help overlay ─────────────────────────────────────────────────────────
fn render_help(f: &mut Frame) {
    let area = centered_rect(55, 75, f.area());
    f.render_widget(Clear, area);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Rgb(59, 130, 246)))
        .title(" Keyboard Shortcuts ")
        .title_style(
            Style::default()
                .fg(Color::Rgb(59, 130, 246))
                .add_modifier(Modifier::BOLD),
        )
        .style(Style::default().bg(Color::Rgb(255, 255, 255)));
    let inner = block.inner(area);
    f.render_widget(block, area);

    let bold_blue = Style::default()
        .fg(Color::Rgb(59, 130, 246))
        .add_modifier(Modifier::BOLD);
    let key_color = Style::default()
        .fg(Color::Rgb(59, 130, 246))
        .add_modifier(Modifier::BOLD);
    let desc_color = Style::default().fg(Color::Rgb(51, 65, 85));

    let help = vec![
        Line::from(""),
        sec("Navigation", &bold_blue),
        keyline(
            "Up / Down / j / k",
            "Move selection",
            &key_color,
            &desc_color,
        ),
        keyline(
            "PageUp / PageDown",
            "Scroll by page",
            &key_color,
            &desc_color,
        ),
        keyline("Home / End", "Jump to start / end", &key_color, &desc_color),
        Line::from(""),
        sec("Selection", &bold_blue),
        keyline("Space", "Toggle app selection", &key_color, &desc_color),
        keyline("a", "Select all", &key_color, &desc_color),
        keyline("n", "Clear selection", &key_color, &desc_color),
        Line::from(""),
        sec("Actions", &bold_blue),
        keyline("u", "Uninstall selected app", &key_color, &desc_color),
        keyline("f", "Force remove selected app", &key_color, &desc_color),
        keyline("l", "Scan for leftovers", &key_color, &desc_color),
        keyline("b", "Add to batch queue", &key_color, &desc_color),
        keyline("r", "Rescan all applications", &key_color, &desc_color),
        Line::from(""),
        sec("View", &bold_blue),
        keyline(
            "d / Enter / Tab",
            "Toggle details panel",
            &key_color,
            &desc_color,
        ),
        keyline("m", "Open context menu", &key_color, &desc_color),
        keyline("? / h", "Show / hide this help", &key_color, &desc_color),
        keyline("q / Esc", "Quit", &key_color, &desc_color),
        Line::from(""),
        Line::from(vec![Span::styled(
            "  Right-click on an app for context menu",
            Style::default().fg(Color::Rgb(148, 163, 184)),
        )]),
        Line::from(vec![Span::styled(
            "  Mouse scroll to navigate list",
            Style::default().fg(Color::Rgb(148, 163, 184)),
        )]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "  Press any key to close",
            Style::default().fg(Color::Rgb(148, 163, 184)),
        )]),
    ];

    f.render_widget(Paragraph::new(help).wrap(Wrap { trim: false }), inner);
}

fn sec<'a>(text: &'a str, style: &'a Style) -> Line<'a> {
    Line::from(Span::styled(format!("  {}", text), *style))
}

fn keyline<'a>(key: &'a str, desc: &'a str, ks: &'a Style, ds: &'a Style) -> Line<'a> {
    Line::from(vec![
        Span::styled(format!("    {:<20}", key), *ks),
        Span::styled(desc, *ds),
    ])
}

// ── Context menu ─────────────────────────────────────────────────────────
fn render_context_menu(f: &mut Frame, app: &TuiApp) {
    let items = context_menu_items();
    let labels = [
        "View Details",
        "Uninstall",
        "Force Remove",
        "Scan Leftovers",
        "Add to Batch",
        "Cancel",
    ];
    let colors = [
        Color::Rgb(51, 65, 85),
        Color::Rgb(239, 68, 68),
        Color::Rgb(239, 68, 68),
        Color::Rgb(16, 185, 129),
        Color::Rgb(245, 158, 11),
        Color::Rgb(148, 163, 184),
    ];

    let menu_h = items.len() as u16 + 2;
    let menu_w = 22;
    let y = app.context_menu_y();
    let x = f.area().width.saturating_sub(menu_w + 2);

    let menu_area = Rect::new(x, y, menu_w, menu_h);

    // Shadow
    f.render_widget(Clear, Rect::new(x + 1, y + 1, menu_w, menu_h));
    f.render_widget(
        Block::default()
            .borders(Borders::ALL)
            .style(Style::default().bg(Color::Rgb(190, 195, 205))),
        Rect::new(x + 1, y + 1, menu_w, menu_h),
    );

    // Menu
    f.render_widget(Clear, menu_area);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Rgb(170, 180, 200)))
        .title(" Actions ")
        .title_style(
            Style::default()
                .fg(Color::Rgb(59, 130, 246))
                .add_modifier(Modifier::BOLD),
        )
        .style(Style::default().bg(Color::Rgb(255, 255, 255)));
    let inner = block.inner(menu_area);
    f.render_widget(block, menu_area);

    let sel = app.context_menu_index();
    let list_items: Vec<ListItem> = labels
        .iter()
        .enumerate()
        .map(|(i, text)| {
            let style = if i == sel {
                Style::default()
                    .fg(Color::White)
                    .bg(Color::Rgb(59, 130, 246))
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(colors[i])
            };
            ListItem::new(Line::from(Span::styled(format!(" {} ", text), style)))
        })
        .collect();

    f.render_widget(List::new(list_items), inner);
}

// ── Overlay ──────────────────────────────────────────────────────────────
fn render_overlay(f: &mut Frame, title: &str, msg: &str) {
    let area = centered_rect(35, 15, f.area());
    f.render_widget(Clear, area);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Rgb(59, 130, 246)))
        .title(format!(" {} ", title))
        .title_style(
            Style::default()
                .fg(Color::Rgb(59, 130, 246))
                .add_modifier(Modifier::BOLD),
        )
        .style(Style::default().bg(Color::Rgb(255, 255, 255)));
    let inner = block.inner(area);
    f.render_widget(block, area);

    f.render_widget(
        Paragraph::new(msg)
            .alignment(Alignment::Center)
            .style(Style::default().fg(Color::Rgb(100, 116, 139))),
        inner,
    );
}

// ── Helpers ──────────────────────────────────────────────────────────────
fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(layout[1])[1]
}
