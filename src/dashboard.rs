use anyhow::{Context, Result};
use ratatui::{
    crossterm::{
        event::{self, Event, KeyCode, KeyEventKind},
        terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
        ExecutableCommand,
    },
    layout::{Alignment, Constraint, Direction, Layout, Margin},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Cell, Clear, Gauge, Paragraph, Row, Sparkline, Table},
    Frame, Terminal,
};
use std::io::{self, stdout};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio::time::interval;

use crate::i18n::lang;
use crate::monitor::{fetch_metrics, TunnelMetrics};
use crate::t;

/// Data point for sparkline (keeps last N values)
const HISTORY_SIZE: usize = 60;

#[derive(Debug, Clone)]
struct MetricsHistory {
    requests: Vec<u64>,
    streams: Vec<u64>,
    errors: Vec<u64>,
    timestamps: Vec<String>,
}

impl MetricsHistory {
    fn new() -> Self {
        Self {
            requests: Vec::with_capacity(HISTORY_SIZE),
            streams: Vec::with_capacity(HISTORY_SIZE),
            errors: Vec::with_capacity(HISTORY_SIZE),
            timestamps: Vec::with_capacity(HISTORY_SIZE),
        }
    }

    fn push(&mut self, m: &TunnelMetrics, ts: String) {
        if self.requests.len() >= HISTORY_SIZE {
            self.requests.remove(0);
            self.streams.remove(0);
            self.errors.remove(0);
            self.timestamps.remove(0);
        }
        self.requests.push(m.total_requests.unwrap_or(0.0) as u64);
        self.streams.push(m.active_streams.unwrap_or(0.0) as u64);
        self.errors.push(m.request_errors.unwrap_or(0.0) as u64);
        self.timestamps.push(ts);
    }
}

/// App state for TUI dashboard
struct App {
    metrics: Option<TunnelMetrics>,
    history: MetricsHistory,
    last_update: Option<Instant>,
    connected: bool,
    show_help: bool,
    quit: bool,
}

impl App {
    fn new() -> Self {
        Self {
            metrics: None,
            history: MetricsHistory::new(),
            last_update: None,
            connected: false,
            show_help: false,
            quit: false,
        }
    }

    async fn update(&mut self) {
        match fetch_metrics().await {
            Ok(m) => {
                self.connected = true;
                let ts = chrono::Local::now().format("%H:%M:%S").to_string();
                self.history.push(&m, ts);
                self.metrics = Some(m);
                self.last_update = Some(Instant::now());
            }
            Err(_) => {
                self.connected = false;
            }
        }
    }
}

/// Run the TUI dashboard
pub async fn run_dashboard() -> Result<()> {
    // Setup terminal
    enable_raw_mode().context("failed to enable raw mode")?;
    stdout().execute(EnterAlternateScreen).context("failed to enter alternate screen")?;
    let mut terminal = Terminal::new(ratatui::backend::CrosstermBackend::new(stdout()))?;

    let app = Arc::new(Mutex::new(App::new()));

    // Initial data fetch
    {
        let mut app = app.lock().unwrap();
        app.update().await;
    }

    // Spawn background update task
    let app_clone = app.clone();
    tokio::spawn(async move {
        let mut ticker = interval(Duration::from_secs(2));
        loop {
            ticker.tick().await;
            let mut app = app_clone.lock().unwrap();
            if app.quit {
                break;
            }
            app.update().await;
        }
    });

    // Main render loop
    let result = run_ui_loop(&mut terminal, app.clone()).await;

    // Cleanup
    disable_raw_mode().context("failed to disable raw mode")?;
    stdout().execute(LeaveAlternateScreen).context("failed to leave alternate screen")?;

    result
}

async fn run_ui_loop(
    terminal: &mut Terminal<ratatui::backend::CrosstermBackend<io::Stdout>>,
    app: Arc<Mutex<App>>,
) -> Result<()> {
    let mut last_tick = Instant::now();
    let tick_rate = Duration::from_millis(100);

    loop {
        // Draw
        {
            let app = app.lock().unwrap();
            if app.quit {
                break;
            }
            terminal.draw(|f| draw_ui(f, &app))?;
        }

        // Handle events (non-blocking)
        let timeout = tick_rate.saturating_sub(last_tick.elapsed());
        if event::poll(timeout)? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    let mut app = app.lock().unwrap();
                    match key.code {
                        KeyCode::Char('q') | KeyCode::Esc => app.quit = true,
                        KeyCode::Char('h') | KeyCode::Char('?') => app.show_help = !app.show_help,
                        KeyCode::Char('r') => {
                            // Force refresh (handled by background task)
                        }
                        _ => {}
                    }
                }
            }
        }

        if last_tick.elapsed() >= tick_rate {
            last_tick = Instant::now();
        }
    }

    Ok(())
}

fn draw_ui(f: &mut Frame, app: &App) {
    let l = lang();
    let size = f.area();

    // Main layout
    let main_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(3)])
        .split(size);

    let content_area = main_layout[0];
    let footer_area = main_layout[1];

    // Content layout: header + metrics + charts
    let content_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Header
            Constraint::Length(9), // Metrics cards
            Constraint::Min(10),   // Charts
        ])
        .margin(1)
        .split(content_area);

    // Draw blocks
    let block = Block::default()
        .title(format!(" {} ", t!(l, "openTunnel Dashboard", "openTunnel 仪表盘")))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));
    f.render_widget(block, content_area);

    // Header with connection status
    draw_header(f, app, content_layout[0]);

    // Metrics cards
    draw_metrics(f, app, content_layout[1]);

    // Charts
    draw_charts(f, app, content_layout[2]);

    // Footer with help
    draw_footer(f, app, footer_area);

    // Help popup
    if app.show_help {
        draw_help_popup(f, size);
    }
}

fn draw_header(f: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    let l = lang();

    let status_text = if app.connected {
        t!(l, "● Connected", "● 已连接").to_string()
    } else {
        t!(l, "○ Disconnected", "○ 未连接").to_string()
    };

    let status_color = if app.connected { Color::Green } else { Color::Red };

    let header_text = format!(
        "{} | {}: {} | {}: {}",
        t!(l, "Real-time Tunnel Monitor", "实时隧道监控"),
        t!(l, "Status", "状态"),
        status_text,
        t!(l, "Press ? for help", "按 ? 查看帮助"),
        ""
    );

    let header = Paragraph::new(header_text)
        .style(Style::default().fg(status_color).add_modifier(Modifier::BOLD))
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::BOTTOM));

    f.render_widget(header, area);
}

fn draw_metrics(f: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    let l = lang();

    let metrics_layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(25),
            Constraint::Percentage(25),
            Constraint::Percentage(25),
            Constraint::Percentage(25),
        ])
        .split(area);

    let metrics = app.metrics.as_ref();

    // Requests card
    let requests_val = metrics
        .and_then(|m| m.total_requests)
        .map(|v| format_num(v))
        .unwrap_or_else(|| "-".to_string());
    draw_metric_card(
        f,
        metrics_layout[0],
        t!(l, "Total Requests", "总请求数"),
        &requests_val,
        Color::Cyan,
    );

    // Streams card
    let streams_val = metrics
        .and_then(|m| m.active_streams)
        .map(|v| format_num(v))
        .unwrap_or_else(|| "-".to_string());
    draw_metric_card(
        f,
        metrics_layout[1],
        t!(l, "Active Streams", "活跃连接"),
        &streams_val,
        Color::Green,
    );

    // Errors card
    let errors_val = metrics
        .and_then(|m| m.request_errors)
        .map(|v| format_num(v))
        .unwrap_or_else(|| "-".to_string());
    draw_metric_card(
        f,
        metrics_layout[2],
        t!(l, "Errors", "错误数"),
        &errors_val,
        Color::Red,
    );

    // Update time card
    let update_text = app
        .last_update
        .map(|t| format!("{:.1}s", t.elapsed().as_secs_f64()))
        .unwrap_or_else(|| "-".to_string());
    draw_metric_card(
        f,
        metrics_layout[3],
        t!(l, "Last Update", "上次更新"),
        &update_text,
        Color::Yellow,
    );
}

fn draw_metric_card(
    f: &mut Frame,
    area: ratatui::layout::Rect,
    title: &str,
    value: &str,
    color: Color,
) {
    let block = Block::default()
        .title(format!(" {} ", title))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(color));

    let value_paragraph = Paragraph::new(value.to_string())
        .style(Style::default().fg(color).add_modifier(Modifier::BOLD))
        .alignment(Alignment::Center);

    f.render_widget(block, area);

    // Render value in the center of the block
    let inner = area.inner(Margin::new(1, 1));
    f.render_widget(value_paragraph, inner);
}

fn draw_charts(f: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    let l = lang();

    let charts_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    // Requests sparkline
    let requests_data: Vec<u64> = app.history.requests.clone();
    let requests_sparkline = Sparkline::default()
        .block(
            Block::default()
                .title(format!(" {} ", t!(l, "Requests History", "请求历史")))
                .borders(Borders::ALL),
        )
        .data(&requests_data)
        .style(Style::default().fg(Color::Cyan));
    f.render_widget(requests_sparkline, charts_layout[0]);

    // Streams sparkline
    let streams_data: Vec<u64> = app.history.streams.clone();
    let streams_sparkline = Sparkline::default()
        .block(
            Block::default()
                .title(format!(" {} ", t!(l, "Streams History", "连接历史")))
                .borders(Borders::ALL),
        )
        .data(&streams_data)
        .style(Style::default().fg(Color::Green));
    f.render_widget(streams_sparkline, charts_layout[1]);
}

fn draw_footer(f: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    let l = lang();

    let help_text = if app.connected {
        format!(
            "{}: q={} | ?=help | r=refresh",
            t!(l, "Keys", "按键"),
            t!(l, "quit", "退出")
        )
    } else {
        format!(
            "{}: {} | q={}",
            t!(l, "Status", "状态"),
            t!(
                l,
                "cloudflared not running or metrics disabled",
                "cloudflared 未运行或指标未开启"
            ),
            t!(l, "quit", "退出")
        )
    };

    let footer = Paragraph::new(help_text)
        .style(Style::default().fg(Color::Gray))
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::TOP));

    f.render_widget(footer, area);
}

fn draw_help_popup(f: &mut Frame, area: ratatui::layout::Rect) {
    let l = lang();

    let popup_block = Block::default()
        .title(format!(" {} ", t!(l, "Help", "帮助")))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Yellow))
        .style(Style::default().bg(Color::Black));

    let popup_area = centered_rect(60, 40, area);
    f.render_widget(Clear, popup_area);
    f.render_widget(popup_block, popup_area);

    let help_content = format!(
        "{}

{}:
  q / Esc  - {}
  ? / h    - {}
  r        - {}

{}:
  - {}
  - {}
  - {}",
        t!(l, "openTunnel Dashboard Controls", "openTunnel 仪表盘控制"),
        t!(l, "Keys", "按键"),
        t!(l, "Quit", "退出"),
        t!(l, "Toggle help", "切换帮助"),
        t!(l, "Force refresh", "强制刷新"),
        t!(l, "Notes", "说明"),
        t!(l, "Data updates every 2 seconds", "数据每 2 秒更新"),
        t!(l, "Requires cloudflared metrics enabled", "需要开启 cloudflared 指标"),
        t!(l, "Default metrics endpoint: 127.0.0.1:20241", "默认指标端点: 127.0.0.1:20241")
    );

    let help_paragraph = Paragraph::new(help_content)
        .alignment(Alignment::Left)
        .wrap(ratatui::widgets::Wrap { trim: true });

    let inner = popup_area.inner(Margin::new(2, 1));
    f.render_widget(help_paragraph, inner);
}

fn centered_rect(percent_x: u16, percent_y: u16, r: ratatui::layout::Rect) -> ratatui::layout::Rect {
    let popup_layout = Layout::default()
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
        .split(popup_layout[1])[1]
}

fn format_num(n: f64) -> String {
    if n >= 1_000_000.0 {
        format!("{:.1}M", n / 1_000_000.0)
    } else if n >= 1_000.0 {
        format!("{:.1}K", n / 1_000.0)
    } else {
        format!("{:.0}", n)
    }
}
