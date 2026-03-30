use sysinfo::{System, Cpu};
use std::{thread, time};

fn main() {
    let mut sys = System::new_all();

    loop{
        sys.refresh_all();
        
    }
}
