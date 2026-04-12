// ui.rs — futuristic dashboard grid with mouse + sorting
// tui 0.19 + crossterm 0.27

use tui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Span, Spans},
    widgets::{Block, Borders, BorderType, Cell, Gauge, Paragraph, Row, Table},
    Frame, Terminal,
};
use crossterm::{
    event::{
        self, Event, KeyCode, MouseButton, MouseEvent, MouseEventKind,
        EnableMouseCapture, DisableMouseCapture,
    },
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use std::{io, time::Duration};
use crate::system::{SystemStats, SortBy};

// ── Re-export so main.rs can use these ───────────────────────────────────────

// ── Color palette ─────────────────────────────────────────────────────────────
const C_ACCENT:  Color = Color::Cyan;
const C_PURPLE:  Color = Color::Magenta;
const C_GREEN:   Color = Color::Green;
const C_YELLOW:  Color = Color::Yellow;
const C_RED:     Color = Color::Red;
const C_DIM:     Color = Color::DarkGray;
const C_WHITE:   Color = Color::White;

// ── App state (sort + mouse selection) ───────────────────────────────────────
#[derive(Default)]
pub struct AppState {
    pub sort_by:       SortBy,
    pub selected_row:  Option<usize>,  // hovered process row
    // cached rects for hit-testing mouse clicks
    pub proc_area:     Option<Rect>,
    pub proc_header_y: Option<u16>,    // y of the process table header row
    pub sort_buttons:  Vec<(Rect, SortBy)>, // clickable sort button areas
}

// ── Actions returned from event poll ─────────────────────────────────────────
pub enum Action {
    Quit,
    Refresh,
    None,
}

// ── Terminal lifecycle ────────────────────────────────────────────────────────

pub fn setup() -> io::Result<Terminal<CrosstermBackend<io::Stdout>>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    // enable mouse capture so we get click/scroll events
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    Terminal::new(CrosstermBackend::new(stdout))
}

pub fn teardown(term: &mut Terminal<CrosstermBackend<io::Stdout>>) -> io::Result<()> {
    disable_raw_mode()?;
    execute!(term.backend_mut(), LeaveAlternateScreen, DisableMouseCapture)?;
    term.show_cursor()
}

// ── Event polling ─────────────────────────────────────────────────────────────

pub fn poll_event(state: &mut AppState) -> Action {
    if !event::poll(Duration::from_millis(0)).unwrap_or(false) {
        return Action::None;
    }
    match event::read() {
        // ── Keyboard ──────────────────────────────────────────────────────────
        Ok(Event::Key(k)) => match k.code {
            KeyCode::Char('q') | KeyCode::Char('Q') => Action::Quit,
            KeyCode::Char('r') | KeyCode::Char('R') => Action::Refresh,
            // sort shortcuts
            KeyCode::Char('c') => { state.sort_by = SortBy::CpuDesc;  Action::Refresh }
            KeyCode::Char('C') => { state.sort_by = SortBy::CpuAsc;   Action::Refresh }
            KeyCode::Char('m') => { state.sort_by = SortBy::MemDesc;  Action::Refresh }
            KeyCode::Char('M') => { state.sort_by = SortBy::MemAsc;   Action::Refresh }
            KeyCode::Char('p') => { state.sort_by = SortBy::Pid;      Action::Refresh }
            KeyCode::Char('n') => { state.sort_by = SortBy::Name;     Action::Refresh }
            _ => Action::None,
        },

        // ── Mouse ─────────────────────────────────────────────────────────────
        Ok(Event::Mouse(m)) => handle_mouse(state, m),

        _ => Action::None,
    }
}

fn handle_mouse(state: &mut AppState, m: MouseEvent) -> Action {
    match m.kind {
        // left click — check sort buttons + process rows
        MouseEventKind::Down(MouseButton::Left) => {
            let (col, row) = (m.column, m.row);

            // check sort button hits
            for (rect, sort) in &state.sort_buttons {
                if in_rect(col, row, *rect) {
                    state.sort_by = *sort;
                    return Action::Refresh;
                }
            }

            // check process row click (select row)
            if let (Some(area), Some(header_y)) = (state.proc_area, state.proc_header_y)
                && in_rect(col, row, area) && row > header_y {
                let idx = (row - header_y - 1) as usize;
                state.selected_row = Some(idx);
            }
            Action::None
        }

        // scroll wheel in process table — move selection
        MouseEventKind::ScrollDown => {
            if let Some(ref mut sel) = state.selected_row {
                *sel = sel.saturating_add(1).min(9);
            }
            Action::None
        }
        MouseEventKind::ScrollUp => {
            if let Some(ref mut sel) = state.selected_row {
                *sel = sel.saturating_sub(1);
            }
            Action::None
        }

        // hover — highlight process row
        MouseEventKind::Moved => {
            if let (Some(area), Some(header_y)) = (state.proc_area, state.proc_header_y) {
                let (col, row) = (m.column, m.row);
                if in_rect(col, row, area) && row > header_y {
                    state.selected_row = Some((row - header_y - 1) as usize);
                } else {
                    state.selected_row = None;
                }
            }
            Action::None
        }

        _ => Action::None,
    }
}

fn in_rect(col: u16, row: u16, r: Rect) -> bool {
    col >= r.x && col < r.x + r.width && row >= r.y && row < r.y + r.height
}

// ── Root draw ─────────────────────────────────────────────────────────────────

pub fn draw(f: &mut Frame<CrosstermBackend<io::Stdout>>, stats: &SystemStats, state: &mut AppState) {
    let area = f.size();

    if area.width < 80 || area.height < 24 {
        let msg = Paragraph::new(Spans::from(vec![
            Span::styled(" ktop-r ", Style::default().fg(C_ACCENT).add_modifier(Modifier::BOLD)),
            Span::styled(
                format!("terminal too small ({}x{}, need 80x24)", area.width, area.height),
                Style::default().fg(C_YELLOW),
            ),
        ]));
        f.render_widget(msg, area);
        return;
    }

    // clear sort buttons each frame (rebuilt below)
    state.sort_buttons.clear();

    // root: header(3) | grid | footer(1)
    let root = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(0),
            Constraint::Length(1),
        ])
        .split(area);

    draw_header(f, root[0], stats);

    // 4-quadrant grid
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(root[1]);

    // top: CPU 60% | Memory 40%
    let top = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
        .split(rows[0]);

    // bottom: Processes 60% | Disk+Net 40%
    let bot = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
        .split(rows[1]);

    let bot_right = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(bot[1]);

    draw_cpu(f, top[0], stats);
    draw_memory(f, top[1], stats);
    draw_processes(f, bot[0], stats, state);
    draw_disk(f, bot_right[0], stats);
    draw_network(f, bot_right[1], stats);

    draw_footer(f, root[2], state);
}

// ── Header ────────────────────────────────────────────────────────────────────

fn draw_header(f: &mut Frame<CrosstermBackend<io::Stdout>>, area: Rect, s: &SystemStats) {
    let now = wall_time();
    let sep = "─".repeat(area.width as usize);

    f.render_widget(
        Paragraph::new(Spans::from(Span::styled(&sep, Style::default().fg(C_PURPLE)))),
        Rect { height: 1, ..area },
    );

    let header_area = Rect { y: area.y + 1, height: 1, ..area };
    f.render_widget(
        Paragraph::new(Spans::from(vec![
            Span::styled("  ◈ ", Style::default().fg(C_PURPLE)),
            Span::styled("ktop-r", Style::default().fg(C_ACCENT).add_modifier(Modifier::BOLD)),
            Span::styled("  koktail's system monitor", Style::default().fg(C_DIM)),
            Span::styled(
                format!("  ·  cpu {:.0}%  ·  mem {} MB  ·  {}  ", s.cpu_total, s.mem_used, now),
                Style::default().fg(C_DIM),
            ),
        ])),
        header_area,
    );

    f.render_widget(
        Paragraph::new(Spans::from(Span::styled(&sep, Style::default().fg(C_PURPLE)))),
        Rect { y: area.y + 2, height: 1, ..area },
    );
}

// ── CPU ───────────────────────────────────────────────────────────────────────


fn draw_cpu(f: &mut Frame<CrosstermBackend<io::Stdout>>, area: Rect, s: &SystemStats) {
    let title = format!("◈ CPU  ⟨{}⟩", clip(&s.cpu_name, 28));
    let block = future_block(&title, C_ACCENT);
    let inner = block.inner(area);
    f.render_widget(block, area);

    let core_count = s.cpu_cores.len().min(8);

    let mut constraints = vec![
        Constraint::Length(1), // total bar
        Constraint::Length(1), // cores label
    ];
    for _ in 0..core_count { constraints.push(Constraint::Length(1)); }
    constraints.push(Constraint::Length(1)); // freq
    constraints.push(Constraint::Min(0));

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(inner);

    // total bar — full width
    let bar_w = inner.width.saturating_sub(14) as usize;
    f.render_widget(cpu_bar(s.cpu_total as u16, bar_w, "TOTAL", C_ACCENT, true), rows[0]);

    // cores divider
    f.render_widget(
        Paragraph::new(Spans::from(Span::styled(
            " ┄ cores ┄──────────────────────────────────────",
            Style::default().fg(C_DIM),
        ))),
        rows[1],
    );

    // per-core bars — full width each
    let core_w = inner.width.saturating_sub(14) as usize;
    for (i, &usage) in s.cpu_cores.iter().take(8).enumerate() {
        let color = if i % 2 == 0 { C_ACCENT } else { C_PURPLE };
        f.render_widget(cpu_bar(usage as u16, core_w, &format!("C{}", i), color, false), rows[i + 2]);
    }

    // freq + core count
    f.render_widget(
        Paragraph::new(Spans::from(vec![
            Span::styled(" ◇ freq ", Style::default().fg(C_DIM)),
            Span::styled(format!("{} MHz", s.cpu_freq), Style::default().fg(C_ACCENT).add_modifier(Modifier::BOLD)),
            Span::styled("  ◇ cores ", Style::default().fg(C_DIM)),
            Span::styled(format!("{}", s.cpu_cores.len()), Style::default().fg(C_ACCENT).add_modifier(Modifier::BOLD)),
        ])),
        rows[core_count + 2],
    );
}

// Custom text bar — uses actual terminal width, no tui Gauge quirks
fn cpu_bar(percent: u16, bar_width: usize, label: &str, color: Color, bold: bool) -> Paragraph<'static> {
    let pct   = percent.min(100) as usize;
    let bar_color = if pct >= 85 { C_RED } else if pct >= 60 { C_YELLOW } else { color };
    let filled = bar_width * pct / 100;
    let empty  = bar_width.saturating_sub(filled);
    let modifier = if bold { Modifier::BOLD } else { Modifier::empty() };

    Paragraph::new(Spans::from(vec![
        Span::styled(format!(" {:<6}", label), Style::default().fg(C_DIM).add_modifier(modifier)),
        Span::styled("▐".to_string(), Style::default().fg(bar_color)),
        Span::styled("█".repeat(filled), Style::default().fg(bar_color).add_modifier(modifier)),
        Span::styled("░".repeat(empty),  Style::default().fg(Color::Rgb(30, 30, 50))),
        Span::styled("▌".to_string(), Style::default().fg(bar_color)),
        Span::styled(format!(" {:3}%", pct), Style::default().fg(C_WHITE).add_modifier(Modifier::BOLD)),
    ]))
}

// ── Memory ────────────────────────────────────────────────────────────────────

fn draw_memory(f: &mut Frame<CrosstermBackend<io::Stdout>>, area: Rect, s: &SystemStats) {
    let block = future_block("◈ MEMORY", C_PURPLE);
    let inner = block.inner(area);
    f.render_widget(block, area);

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Min(0),
        ])
        .split(inner);

    f.render_widget(fancy_gauge(s.mem_percent as u16, "  RAM ", C_PURPLE), rows[0]);
    f.render_widget(stat_line("  ◇", &format!("{} MB / {} MB", s.mem_used, s.mem_total), C_PURPLE), rows[1]);
    f.render_widget(
        Paragraph::new(Spans::from(Span::styled(" ┄ swap ┄", Style::default().fg(C_DIM)))),
        rows[2],
    );
    f.render_widget(fancy_gauge(s.swap_percent as u16, "  SWAP", C_ACCENT), rows[3]);
    f.render_widget(stat_line("  ◇", &format!("{} MB / {} MB", s.swap_used, s.swap_total), C_ACCENT), rows[4]);
}

// ── Processes (with sort buttons + mouse highlight) ───────────────────────────

fn draw_processes(
    f: &mut Frame<CrosstermBackend<io::Stdout>>,
    area: Rect,
    s: &SystemStats,
    state: &mut AppState,
) {
    // save area for mouse hit-testing
    state.proc_area = Some(area);

    // ── sort bar above the table ───────────────────────────────────────────────
    // layout: sort_bar(1) | table(rest)
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Min(0)])
        .split(area);

    draw_sort_bar(f, chunks[0], state);

    // ── process table ──────────────────────────────────────────────────────────
    let sort_label = sort_label(state.sort_by);
    let title = format!("◈ PROCESSES  ⟨sort: {}⟩", sort_label);
    let block = future_block(&title, C_ACCENT);
    let inner = block.inner(chunks[1]);
    f.render_widget(block, chunks[1]);

    // header row y for mouse hit-testing
    state.proc_header_y = Some(inner.y);

    let header = Row::new(vec![
        Cell::from(" PID")  .style(Style::default().fg(C_ACCENT).add_modifier(Modifier::BOLD)),
        Cell::from("NAME")  .style(Style::default().fg(C_ACCENT).add_modifier(Modifier::BOLD)),
        Cell::from(sort_col_label("CPU%",  state.sort_by, SortBy::CpuDesc, SortBy::CpuAsc))
            .style(Style::default().fg(C_YELLOW).add_modifier(Modifier::BOLD)),
        Cell::from(sort_col_label("MEM",   state.sort_by, SortBy::MemDesc, SortBy::MemAsc))
            .style(Style::default().fg(C_YELLOW).add_modifier(Modifier::BOLD)),
    ]).height(1).bottom_margin(0);

    let rows: Vec<Row> = s.processes.iter().enumerate().map(|(i, p)| {
        let is_selected = state.selected_row == Some(i);
        let cpu_color = if p.cpu > 50.0 { C_RED }
        else if p.cpu > 15.0 { C_YELLOW }
        else { C_GREEN };

        let base_style = if is_selected {
            Style::default().fg(C_WHITE).bg(Color::Rgb(30, 30, 60)).add_modifier(Modifier::BOLD)
        } else if i == 0 {
            Style::default().fg(C_WHITE).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(C_DIM)
        };

        Row::new(vec![
            Cell::from(format!(" {}", p.pid)).style(base_style),
            Cell::from(clip(&p.name, 20)).style(base_style),
            Cell::from(format!("{:.1}%", p.cpu))
                .style(Style::default().fg(cpu_color).add_modifier(if is_selected { Modifier::BOLD } else { Modifier::empty() })),
            Cell::from(format!("{} MB", p.mem_mb)).style(base_style),
        ])
    }).collect();

    let table = Table::new(rows)
        .header(header)
        .widths(&[
            Constraint::Length(7),
            Constraint::Min(18),
            Constraint::Length(7),
            Constraint::Length(8),
        ]);

    f.render_widget(table, inner);
}

// ── Sort button bar ───────────────────────────────────────────────────────────

fn draw_sort_bar(f: &mut Frame<CrosstermBackend<io::Stdout>>, area: Rect, state: &mut AppState) {
    // define clickable sort buttons: label, sort mode
    let buttons: &[(&str, SortBy)] = &[
        ("CPU▼", SortBy::CpuDesc),
        ("CPU▲", SortBy::CpuAsc),
        ("MEM▼", SortBy::MemDesc),
        ("MEM▲", SortBy::MemAsc),
        ("PID",  SortBy::Pid),
        ("NAME", SortBy::Name),
    ];

    let mut x = area.x + 1;
    let y = area.y;

    for (label, sort) in buttons {
        let w = label.len() as u16 + 2;
        let btn_rect = Rect { x, y, width: w, height: 1 };

        let active = state.sort_by == *sort;
        let style = if active {
            Style::default().fg(Color::Black).bg(C_ACCENT).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(C_DIM)
        };

        let text = format!("[{}]", label);
        f.render_widget(Paragraph::new(Spans::from(Span::styled(text, style))), btn_rect);

        // register button for mouse hit-testing
        state.sort_buttons.push((btn_rect, *sort));

        x += w + 1;
        if x >= area.x + area.width { break; }
    }
}

// ── Disk ──────────────────────────────────────────────────────────────────────

fn draw_disk(f: &mut Frame<CrosstermBackend<io::Stdout>>, area: Rect, s: &SystemStats) {
    let title = format!("◈ DISK  ⟨{}⟩", clip(&s.disk_name, 12));
    let block = future_block(&title, C_GREEN);
    let inner = block.inner(area);
    f.render_widget(block, area);

    // layout: usage bar + size line + divider + dir rows
    let dir_count  = s.disk_dirs.len().min(6);
    let mut constraints = vec![
        Constraint::Length(1), // usage bar
        Constraint::Length(1), // used/total
        Constraint::Length(1), // ┄ dirs ┄
    ];
    for _ in 0..dir_count { constraints.push(Constraint::Length(1)); }
    constraints.push(Constraint::Min(0));

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(inner);

    // overall disk bar
    let bar_w = inner.width.saturating_sub(14) as usize;
    f.render_widget(cpu_bar(s.disk_percent as u16, bar_w, "/", C_GREEN, true), rows[0]);
    f.render_widget(
        stat_line("  ◇", &format!("{:.1} GB / {:.1} GB", s.disk_used_gb, s.disk_total_gb), C_GREEN),
        rows[1],
    );

    // divider
    f.render_widget(
        Paragraph::new(Spans::from(Span::styled(
            " ┄ top dirs ┄──────────────────────────────────",
            Style::default().fg(C_DIM),
        ))),
        rows[2],
    );

    #[allow(unused_variables)]
    let max_gb = s.disk_dirs.iter().map(|d| d.size_gb as u64).max().unwrap_or(1).max(1);

    // unique color per dir so you can tell them apart at a glance
    #[allow(unused_variables)]
    let dir_colors = [
        Color::Green,                   // /home
        Color::Cyan,                    // /usr
        Color::Magenta,                 // /var
        Color::Yellow,                  // /opt
        Color::Blue,                    // /boot
        Color::Red,                     // /tmp
        Color::LightGreen,              // /srv
        Color::LightCyan,               // /root
    ];
    // directory rows
    for (i, dir) in s.disk_dirs.iter().take(6).enumerate() {
        let bar_w = inner.width.saturating_sub(16) as usize;
        let color = match i % 3 { 0 => C_GREEN, 1 => C_ACCENT, _ => C_PURPLE };
        let label = clip_path(&dir.path, 6);

        let filled = (bar_w * dir.percent as usize / 100).min(bar_w);
        let empty  = bar_w.saturating_sub(filled);

        f.render_widget(
            Paragraph::new(Spans::from(vec![
                Span::styled(format!(" {:<7}", label), Style::default().fg(color).add_modifier(Modifier::BOLD)),
                Span::styled("▐".to_string(), Style::default().fg(color)),
                Span::styled("█".repeat(filled), Style::default().fg(color)),
                Span::styled("░".repeat(empty),  Style::default().fg(Color::Rgb(20, 20, 20))),
                Span::styled("▌".to_string(), Style::default().fg(color)),
                Span::styled(
                    format!(" {:.1}G", dir.size_gb),
                    Style::default().fg(C_WHITE).add_modifier(Modifier::BOLD),
                ),
            ])),
            rows[i + 3],
        );
    }

    // if dirs not yet loaded
    if s.disk_dirs.is_empty() {
        f.render_widget(
            Paragraph::new(Spans::from(Span::styled(
                "  scanning...",
                Style::default().fg(C_DIM),
            ))),
            rows[3],
        );
    }
}

// Clip path to last N chars with leading slash
fn clip_path(path: &str, max: usize) -> String {
    let name = path.trim_end_matches('/');
    let last = name.rsplit('/').next().unwrap_or(name);
    if last.len() <= max { format!("/{}", last) }
    else { format!("/{}…", &last[..max.saturating_sub(1)]) }
}

// ── Network ───────────────────────────────────────────────────────────────────

fn draw_network(f: &mut Frame<CrosstermBackend<io::Stdout>>, area: Rect, s: &SystemStats) {
    let title = format!("◈ NET  ⟨{}⟩", clip(&s.net_iface, 10));
    let block = future_block(&title, C_YELLOW);
    let inner = block.inner(area);
    f.render_widget(block, area);

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Length(1), Constraint::Min(0)])
        .split(inner);

    f.render_widget(
        Paragraph::new(Spans::from(vec![
            Span::styled("  ▼ ", Style::default().fg(C_GREEN).add_modifier(Modifier::BOLD)),
            Span::styled(format!("{:>6} KB/s", s.net_rx_kbs), Style::default().fg(C_WHITE)),
            Span::styled("  ▲ ", Style::default().fg(C_YELLOW).add_modifier(Modifier::BOLD)),
            Span::styled(format!("{:>6} KB/s", s.net_tx_kbs), Style::default().fg(C_WHITE)),
        ])),
        rows[0],
    );
    f.render_widget(
        Paragraph::new(Spans::from(vec![
            Span::styled("  ◇ ↓ ", Style::default().fg(C_DIM)),
            Span::styled(format!("{} MB", s.net_rx_total_mb), Style::default().fg(C_WHITE)),
            Span::styled("  ↑ ", Style::default().fg(C_DIM)),
            Span::styled(format!("{} MB", s.net_tx_total_mb), Style::default().fg(C_WHITE)),
        ])),
        rows[1],
    );
}

// ── Footer ────────────────────────────────────────────────────────────────────

fn draw_footer(f: &mut Frame<CrosstermBackend<io::Stdout>>, area: Rect, state: &AppState) {
    let sort = sort_label(state.sort_by);
    f.render_widget(
        Paragraph::new(Spans::from(vec![
            Span::styled(" ◈ ", Style::default().fg(C_PURPLE)),
            Span::styled("q", Style::default().fg(C_ACCENT).add_modifier(Modifier::BOLD)),
            Span::styled(" quit  ", Style::default().fg(C_DIM)),
            Span::styled("c", Style::default().fg(C_ACCENT).add_modifier(Modifier::BOLD)),
            Span::styled("/", Style::default().fg(C_DIM)),
            Span::styled("C", Style::default().fg(C_ACCENT).add_modifier(Modifier::BOLD)),
            Span::styled(" cpu  ", Style::default().fg(C_DIM)),
            Span::styled("m", Style::default().fg(C_ACCENT).add_modifier(Modifier::BOLD)),
            Span::styled("/", Style::default().fg(C_DIM)),
            Span::styled("M", Style::default().fg(C_ACCENT).add_modifier(Modifier::BOLD)),
            Span::styled(" mem  ", Style::default().fg(C_DIM)),
            Span::styled("p", Style::default().fg(C_ACCENT).add_modifier(Modifier::BOLD)),
            Span::styled(" pid  ", Style::default().fg(C_DIM)),
            Span::styled("n", Style::default().fg(C_ACCENT).add_modifier(Modifier::BOLD)),
            Span::styled(" name  ", Style::default().fg(C_DIM)),
            Span::styled(format!("· sort: {}", sort), Style::default().fg(C_DIM)),
        ])),
        area,
    );
}

// ── Widget helpers ────────────────────────────────────────────────────────────

fn fancy_gauge(percent: u16, label: &str, color: Color) -> Gauge<'_> {
    let pct = percent.min(100);
    let bar_color = if pct >= 85 { C_RED }
    else if pct >= 60 { C_YELLOW }
    else { color };

    Gauge::default()
        .block(Block::default())
        .gauge_style(Style::default().fg(bar_color).bg(Color::DarkGray))
        .percent(pct)
        .label(Span::styled(
            format!("{} {:3}%", label, pct),
            Style::default().fg(C_WHITE).add_modifier(Modifier::BOLD),
        ))
}

fn future_block(title: &str, color: Color) -> Block<'_> {
    Block::default()
        .title(Spans::from(vec![
            Span::styled("╸", Style::default().fg(color)),
            Span::styled(format!(" {} ", title), Style::default().fg(color).add_modifier(Modifier::BOLD)),
            Span::styled("╺", Style::default().fg(color)),
        ]))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(color))
}

fn stat_line<'a>(prefix: &'a str, value: &'a str, color: Color) -> Paragraph<'a> {
    Paragraph::new(Spans::from(vec![
        Span::styled(prefix.to_string(), Style::default().fg(color)),
        Span::styled(" ", Style::default()),
        Span::styled(value.to_string(), Style::default().fg(C_WHITE)),
    ]))
}

// Show sort indicator arrow in column header
fn sort_col_label(base: &str, current: SortBy, desc: SortBy, asc: SortBy) -> String {
    if current == desc      { format!("{} ▼", base) }
    else if current == asc  { format!("{} ▲", base) }
    else                    { base.to_string() }
}

fn sort_label(s: SortBy) -> &'static str {
    match s {
        SortBy::CpuDesc => "cpu ▼",
        SortBy::CpuAsc  => "cpu ▲",
        SortBy::MemDesc => "mem ▼",
        SortBy::MemAsc  => "mem ▲",
        SortBy::Pid     => "pid",
        SortBy::Name    => "name",
    }
}

fn clip(s: &str, max: usize) -> String {
    if s.len() <= max { s.to_string() }
    else { format!("{}…", &s[..max.saturating_sub(1)]) }
}

fn wall_time() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    format!("{:02}:{:02}:{:02}", (secs % 86400) / 3600, (secs % 3600) / 60, secs % 60)
}