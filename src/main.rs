use sysinfo::{System, Cpu};
use std::{thread, time};

fn main() {
    let mut sys = System::new_all();

    loop{
        sys.refresh_all();

        print!("\x1B[2J\x1B[1;1H");//Clear screen

        println!("==== ktop ====\n");

        //CPU
        let cpu = sys.global_cpu_info().cpu_usage();
        println!("CPU usage: {:.2}%\n", cpu);

        //RAM
        let total = sys.total_memory();
        let used = sys.used_memory();
        let total_gb = total as f64 / 1_073_741_824.0;
        let used_gb = used as f64 / 1_073_741_824.0;
        println!("Ram Usage: {:.2} / {:.2} GB", used_gb, total_gb);

        //Top processes
        println!("\nTop Processes:");
        let mut processes: Vec<_> = sys.processes().iter().collect();
        processes.sort_by(|a, b| b.1.cpu_usage().partial_cmp(&a.1.cpu_usage()).unwrap());

        for (pid, proc) in processes.iter().take(5) {
            println!(
                "PID: {:<7} | {:<25} | CPU: {:>6.2}%",
                pid,
                proc.name(),
                proc.cpu_usage()
            );
        }
        thread::sleep(time::Duration::from_secs(1));
    }
}
