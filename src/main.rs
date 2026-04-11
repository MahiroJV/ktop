mod system;
mod ui;

use std::{thread, time::Duration};
use system::{Collector};
use ui::AppState;

fn main() -> std::io::Result<()> {
    let mut terminal = ui::setup()?;
    let mut collector = Collector::new();
    let mut stats = collector.collect();
    let mut state = AppState::default();
    let mut tick = 0u32;

    loop {
        terminal.draw(|f| ui::draw(f, &stats, &mut state))?;

        match ui::poll_event(&mut state) {
            ui::Action::Quit => {
                ui::teardown(&mut terminal)?;
                return Ok(());
            }
            ui::Action::Refresh => {
                stats = collector.collect();
                stats.sort_processes(state.sort_by);
                tick = 0;
            }
            ui::Action::None => {}
        }

        thread::sleep(Duration::from_millis(50));
        tick += 1;

        if tick >= 20 {
            stats = collector.collect();
            stats.sort_processes(state.sort_by);
            tick = 0;
        }
    }
}