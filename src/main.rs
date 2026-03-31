// main.rs — ktop entry point
// wires the collector and TUI together in a simple loop

mod system;
mod ui;

use std::{thread, time::Duration};
use system::Collector;

fn main() -> std::io::Result<()> {
    let mut terminal = ui::setup()?;
    let mut collector = Collector::new();

    // first collect so we have real data on frame 1
    let mut stats = collector.collect();

    loop {
        terminal.draw(|f| ui::draw(f, &stats))?;

        // sleep 1s total but check for 'q' every 50ms
        for _ in 0..20 {
            if ui::should_quit() {
                ui::teardown(&mut terminal)?;
                return Ok(());
            }
            thread::sleep(Duration::from_millis(50));
        }

        stats = collector.collect();
    }
}