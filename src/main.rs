mod system;
mod ui;

use std::{
    sync::{Arc, Mutex},
    thread,
    time::Duration,
};
use system::{Collector, DirInfo, SortBy};
use ui::AppState;

fn main() -> std::io::Result<()> {
    let mut terminal = ui::setup()?;
    let mut collector = Collector::new();
    let mut stats = collector.collect_fast();
    let mut state = AppState::default();
    let mut tick = 0u32;
    let mut slow_tick = 0u32;

    // shared buffer for disk dirs — updated by background thread
    // main thread only reads, bg thread writes
    let dir_buf: Arc<Mutex<Vec<DirInfo>>> = Arc::new(Mutex::new(vec![]));
    let dir_buf_bg = Arc::clone(&dir_buf);

    // spawn background thread for du — never blocks the TUI
    thread::spawn(move || {
        loop {
            let dirs = system::collect_disk_dirs();
            if let Ok(mut buf) = dir_buf_bg.lock() {
                *buf = dirs;
            }
            // re-scan every 30 seconds
            thread::sleep(Duration::from_secs(30));
        }
    });

    loop {
        // pull latest disk dirs from background thread (non-blocking try_lock)
        if let Ok(dirs) = dir_buf.try_lock() {
            if !dirs.is_empty() {
                stats.disk_dirs = dirs.clone();
            }
        }

        terminal.draw(|f| ui::draw(f, &stats, &mut state))?;

        match ui::poll_event(&mut state) {
            ui::Action::Quit => {
                ui::teardown(&mut terminal)?;
                return Ok(());
            }
            ui::Action::Refresh => {
                stats = collector.collect_fast();
                stats.sort_processes(state.sort_by);
                tick = 0;
            }
            ui::Action::None => {}
        }

        thread::sleep(Duration::from_millis(50));
        tick += 1;
        slow_tick += 1;

        // fast refresh every 500ms (10 × 50ms)
        if tick >= 10 {
            stats = collector.collect_fast();
            stats.sort_processes(state.sort_by);
            tick = 0;
        }
    }
}