// ui.rs — TUI rendering using tui 0.19 + crossterm 0.27

use tui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Span, Spans},
    widgets::{Block, Borders, Cell, Gauge, Paragraph, Row, Table},
    Frame, Terminal,
};
use crossterm::{
    event::{self, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use std::{io, time::Duration};
use crate::system::SystemStats;

// ── Terminal lifecycle ────────────────────────────────────────────────────────

pub fn setup() -> io::Result<Terminal<CrosstermBackend<io::Stdout>>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    Terminal::new(CrosstermBackend::new(stdout))
}

pub fn teardown(term: &mut Terminal<CrosstermBackend<io::Stdout>>) -> io::Result<()> {
    disable_raw_mode()?;
    execute!(term.backend_mut(), LeaveAlternateScreen)?;
    term.show_cursor()
}

// Returns true if 'q' was pressed
pub fn should_quit() -> bool {
    if event::poll(Duration::from_millis(0)).unwrap_or(false) {
        if let Ok(Event::Key(k)) = event::read() {
            return matches!(k.code, KeyCode::Char('q') | KeyCode::Char('Q'));
        }
    }
    false
}

// ── Root draw ─────────────────────────────────────────────────────────────────

pub fn draw(f: &mut Frame<CrosstermBackend<io::Stdout>>, stats: &SystemStats) {
    let area = f.size();

    // guard: if terminal is too small, just show a message and bail
    // prevents Layout::split from panicking with negative constraint values
    if area.width < 80 || area.height < 24 {
        let msg = Paragraph::new(Spans::from(vec![
            Span::styled(" ktop ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
            Span::styled(
                format!("— terminal too small ({}x{}, need 80x24)", area.width, area.height),
                Style::default().fg(Color::Yellow),
            ),
        ]));
        f.render_widget(msg, area);
        return;
    }

    // vertical: 1 header | body | 1 footer
    let root = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2),
            Constraint::Min(0),
            Constraint::Length(1),
        ])
        .split(area);

    draw_header(f, root[0]);

    // body: left 50% | right 50%
    let body = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(root[1]);

    draw_left(f, body[0], stats);
    draw_right(f, body[1], stats);

    draw_footer(f, root[2]);
}

// ── Header ────────────────────────────────────────────────────────────────────

fn draw_header(f: &mut Frame<CrosstermBackend<io::Stdout>>, area: Rect) {
    let now = wall_time();
    let text = Spans::from(vec![
        Span::styled(" ktop ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        Span::styled("— koktail's system monitor", Style::default().fg(Color::DarkGray)),
        Span::styled(format!("  {}", now), Style::default().fg(Color::DarkGray)),
    ]);
    f.render_widget(Paragraph::new(text), area);
}

// ── Left column: CPU → Disk → Processes ──────────────────────────────────────

fn draw_left(f: &mut Frame<CrosstermBackend<io::Stdout>>, area: Rect, s: &SystemStats) {
    let core_rows = (s.cpu_cores.len().min(8) + 2) as u16; // +2 for border+freq
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(core_rows + 4),  // CPU box
            Constraint::Length(5),              // Disk box
            Constraint::Min(0),                 // Processes
        ])
        .split(area);

    draw_cpu(f, chunks[0], s);
    draw_disk(f, chunks[1], s);
    draw_processes(f, chunks[2], s);
}

// ── Right column: Memory → Network ───────────────────────────────────────────

fn draw_right(f: &mut Frame<CrosstermBackend<io::Stdout>>, area: Rect, s: &SystemStats) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(8),   // Memory
            Constraint::Length(7),   // Network
            Constraint::Min(0),
        ])
        .split(area);

    draw_memory(f, chunks[0], s);
    draw_network(f, chunks[1], s);
}

// ── CPU ───────────────────────────────────────────────────────────────────────

fn draw_cpu(f: &mut Frame<CrosstermBackend<io::Stdout>>, area: Rect, s: &SystemStats) {
    let title = format!(" CPU  {} ", clip(&s.cpu_name, 34));
    let block = styled_block(&title);
    let inner = block.inner(area);
    f.render_widget(block, area);

    let core_count = s.cpu_cores.len().min(8);
    // one row per core + 1 overall + 1 freq line
    let mut constraints: Vec<Constraint> = vec![Constraint::Length(1)]; // overall
    for _ in 0..core_count { constraints.push(Constraint::Length(1)); }
    constraints.push(Constraint::Length(1)); // freq
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(inner);

    f.render_widget(gauge(s.cpu_total as u16, "Total"), rows[0]);
    for (i, &usage) in s.cpu_cores.iter().take(8).enumerate() {
        f.render_widget(gauge(usage as u16, &format!("Core{}", i)), rows[i + 1]);
    }

    let freq = Spans::from(vec![
        Span::styled(" Freq: ", Style::default().fg(Color::DarkGray)),
        Span::styled(format!("{} MHz", s.cpu_freq), Style::default().fg(Color::Cyan)),
    ]);
    f.render_widget(Paragraph::new(freq), rows[core_count + 1]);
}

// ── Memory ────────────────────────────────────────────────────────────────────

fn draw_memory(f: &mut Frame<CrosstermBackend<io::Stdout>>, area: Rect, s: &SystemStats) {
    let block = styled_block(" Memory ");
    let inner = block.inner(area);
    f.render_widget(block, area);

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
        ])
        .split(inner);

    f.render_widget(gauge(s.mem_percent as u16, "RAM "), rows[0]);
    f.render_widget(info_line("Used", &format!("{} MB / {} MB", s.mem_used, s.mem_total)), rows[1]);
    f.render_widget(gauge(s.swap_percent as u16, "Swap"), rows[2]);
    f.render_widget(info_line("Used", &format!("{} MB / {} MB", s.swap_used, s.swap_total)), rows[3]);
}

// ── Disk ──────────────────────────────────────────────────────────────────────

fn draw_disk(f: &mut Frame<CrosstermBackend<io::Stdout>>, area: Rect, s: &SystemStats) {
    let title = format!(" Disk  [{}] ", clip(&s.disk_name, 18));
    let block = styled_block(&title);
    let inner = block.inner(area);
    f.render_widget(block, area);

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Length(1)])
        .split(inner);

    f.render_widget(gauge(s.disk_percent as u16, "/   "), rows[0]);
    f.render_widget(
        info_line("Used", &format!("{:.1} GB / {:.1} GB", s.disk_used_gb, s.disk_total_gb)),
        rows[1],
    );
}

// ── Network ───────────────────────────────────────────────────────────────────

fn draw_network(f: &mut Frame<CrosstermBackend<io::Stdout>>, area: Rect, s: &SystemStats) {
    let title = format!(" Network  [{}] ", clip(&s.net_iface, 12));
    let block = styled_block(&title);
    let inner = block.inner(area);
    f.render_widget(block, area);

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Length(1), Constraint::Length(1)])
        .split(inner);

    let speed = Spans::from(vec![
        Span::styled(" ↓ ", Style::default().fg(Color::Green)),
        Span::styled(format!("{} KB/s", s.net_rx_kbs), Style::default().fg(Color::White)),
        Span::styled("   ↑ ", Style::default().fg(Color::Yellow)),
        Span::styled(format!("{} KB/s", s.net_tx_kbs), Style::default().fg(Color::White)),
    ]);
    f.render_widget(Paragraph::new(speed), rows[0]);

    let totals = Spans::from(vec![
        Span::styled(" Total ↓ ", Style::default().fg(Color::DarkGray)),
        Span::styled(format!("{} MB", s.net_rx_total_mb), Style::default().fg(Color::White)),
        Span::styled("   ↑ ", Style::default().fg(Color::DarkGray)),
        Span::styled(format!("{} MB", s.net_tx_total_mb), Style::default().fg(Color::White)),
    ]);
    f.render_widget(Paragraph::new(totals), rows[1]);
}

// ── Process table ─────────────────────────────────────────────────────────────

fn draw_processes(f: &mut Frame<CrosstermBackend<io::Stdout>>, area: Rect, s: &SystemStats) {
    let header = Row::new(vec![
        Cell::from("PID")   .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        Cell::from("Name")  .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        Cell::from("CPU%")  .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        Cell::from("MEM MB").style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
    ]).height(1).bottom_margin(0);

    let rows: Vec<Row> = s.processes.iter().map(|p| {
        let cpu_color = if p.cpu > 50.0 { Color::Red }
        else if p.cpu > 15.0 { Color::Yellow }
        else { Color::Green };
        Row::new(vec![
            Cell::from(p.pid.to_string()),
            Cell::from(clip(&p.name, 22)),
            Cell::from(format!("{:.1}%", p.cpu)).style(Style::default().fg(cpu_color)),
            Cell::from(format!("{}", p.mem_mb)),
        ])
    }).collect();

    let table = Table::new(rows)
        .header(header)
        .block(styled_block(" Processes "))
        .widths(&[
            Constraint::Length(7),
            Constraint::Min(22),
            Constraint::Length(7),
            Constraint::Length(7),
        ]);

    f.render_widget(table, area);
}

// ── Footer ────────────────────────────────────────────────────────────────────

fn draw_footer(f: &mut Frame<CrosstermBackend<io::Stdout>>, area: Rect) {
    let text = Spans::from(vec![
        Span::styled(" q", Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
        Span::styled(":quit", Style::default().fg(Color::DarkGray)),
    ]);
    f.render_widget(Paragraph::new(text), area);
}

// ── Shared widget helpers ─────────────────────────────────────────────────────

// Colored gauge — green < 60, yellow < 85, red >= 85
fn gauge(percent: u16, label: &str) -> Gauge {
    let pct = percent.min(100);
    let color = if pct >= 85 { Color::Red }
    else if pct >= 60 { Color::Yellow }
    else { Color::Green };

    Gauge::default()
        .block(Block::default())
        .gauge_style(Style::default().fg(color).bg(Color::DarkGray))
        .percent(pct)
        .label(format!("{} {:3}%", label, pct))
}

// Box with cyan border + bold title
fn styled_block(title: &str) -> Block {
    Block::default()
        .title(Span::styled(title, Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan))
}

// "  Label: value" dim/white info line
fn info_line<'a>(label: &'a str, value: &'a str) -> Paragraph<'a> {
    Paragraph::new(Spans::from(vec![
        Span::styled(format!("  {}: ", label), Style::default().fg(Color::DarkGray)),
        Span::styled(value.to_string(), Style::default().fg(Color::White)),
    ]))
}

// Truncate string with ellipsis
fn clip(s: &str, max: usize) -> String {
    if s.len() <= max { s.to_string() }
    else { format!("{}…", &s[..max.saturating_sub(1)]) }
}

// Simple UTC time from epoch (no external crate needed)
fn wall_time() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    format!("{:02}:{:02}:{:02} UTC", (secs % 86400) / 3600, (secs % 3600) / 60, secs % 60)
}