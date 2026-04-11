// system.rs — all data collection
// sysinfo 0.29 uses trait-based API: import the XxxExt trait to access methods

use sysinfo::{
    System, SystemExt, CpuExt, ProcessExt,
    NetworkExt, DiskExt, PidExt,
};

#[derive(Default, Clone)]
pub struct SystemStats {
    pub cpu_total:    f32,
    pub cpu_cores:    Vec<f32>,
    pub cpu_freq:     u64,
    pub cpu_name:     String,

    pub mem_used:     u64,
    pub mem_total:    u64,
    pub mem_percent:  u32,
    pub swap_used:    u64,
    pub swap_total:   u64,
    pub swap_percent: u32,

    pub disk_name:     String,
    pub disk_used_gb:  f64,
    pub disk_total_gb: f64,
    pub disk_percent:  u32,

    pub net_iface:       String,
    pub net_rx_kbs:      u64,
    pub net_tx_kbs:      u64,
    pub net_rx_total_mb: u64,
    pub net_tx_total_mb: u64,

    pub processes: Vec<ProcInfo>,
    pub disk_dirs: Vec<DirInfo>
}
#[derive(Clone, Default)]
pub struct DirInfo {
    pub path:    String,
    pub size_gb: f64,
    pub percent: u32,  // % of total disk
}
#[derive(Clone)]
pub struct ProcInfo {
    pub pid:    u32,
    pub name:   String,
    pub cpu:    f32,
    pub mem_mb: u64,
}

pub struct Collector {
    sys: System,
}

impl Collector {
    pub fn new() -> Self {
        let mut sys = System::new_all();
        sys.refresh_all();
        Self { sys }
    }

    #[allow(clippy::field_reassign_with_default)]
    pub fn collect_fast(&mut self) -> SystemStats {
        self.sys.refresh_all();
        self.sys.refresh_disks_list();
        self.sys.refresh_disks();
        self.sys.refresh_networks_list();
        self.sys.refresh_networks();

        let mut s = SystemStats::default();

        // ── CPU ───────────────────────────────────────────────────────────────
        // global_cpu_info() returns a virtual average CPU (via CpuExt)
        s.cpu_total = self.sys.global_cpu_info().cpu_usage();
        s.cpu_cores = self.sys.cpus().iter().map(|c| c.cpu_usage()).collect();
        if let Some(first) = self.sys.cpus().first() {
            s.cpu_freq = first.frequency();
            s.cpu_name = first.brand().to_string();
        }

        // ── Memory ────────────────────────────────────────────────────────────
        // sysinfo 0.29 returns KB — convert to MB
        s.mem_total   = self.sys.total_memory() / 1024;
        s.mem_used    = self.sys.used_memory()  / 1024;
        s.mem_percent = pct(s.mem_used, s.mem_total);

        s.swap_total   = self.sys.total_swap() / 1024;
        s.swap_used    = self.sys.used_swap()  / 1024;
        s.swap_percent = pct(s.swap_used, s.swap_total);

        // ── Disk ──────────────────────────────────────────────────────────────
        // pick the first real disk (>1 GB, skips tmpfs/devtmpfs)
        if let Some(disk) = self.sys.disks().iter().find(|d| d.total_space() > 1_000_000_000) {
            let total = disk.total_space();
            let used  = total.saturating_sub(disk.available_space());
            s.disk_name     = disk.name().to_string_lossy().to_string();
            s.disk_total_gb = total as f64 / 1_073_741_824.0;
            s.disk_used_gb  = used  as f64 / 1_073_741_824.0;
            s.disk_percent  = (used * 100 / total.max(1)) as u32;
        }


        // ── Network ───────────────────────────────────────────────────────────
        // pick non-loopback with most cumulative received bytes
        let mut best = 0u64;
        for (name, data) in self.sys.networks() {
            if name == "lo" { continue; }
            if data.total_received() > best {
                best               = data.total_received();
                s.net_iface        = name.clone();
                // bytes since last refresh ≈ per-second rate (refresh ~1s)
                s.net_rx_kbs       = data.received()          / 1024;
                s.net_tx_kbs       = data.transmitted()       / 1024;
                s.net_rx_total_mb  = data.total_received()    / 1024 / 1024;
                s.net_tx_total_mb  = data.total_transmitted() / 1024 / 1024;
            }
        }

        // ── Processes ─────────────────────────────────────────────────────────
        let mut procs: Vec<ProcInfo> = self.sys.processes()
            .iter()
            .map(|(pid, p)| ProcInfo {
                pid:    pid.as_u32(),   // requires PidExt in scope
                name:   p.name().to_string(),
                cpu:    p.cpu_usage(),
                mem_mb: p.memory() / 1024,  // KB → MB
            })
            .collect();

        procs.sort_by(|a, b| b.cpu.partial_cmp(&a.cpu).unwrap_or(std::cmp::Ordering::Equal));
        procs.truncate(10);
        s.processes = procs;

        s
    }
}

pub fn collect_disk_dirs() -> Vec<DirInfo> {
    let disk_total_gb = 0.0_f64;
    top_dirs(disk_total_gb)
}


// Scan top-level directories and return sorted by size (biggest first)
fn top_dirs(_disk_total_gb: f64) -> Vec<DirInfo> {
    use std::process::Command;

    // only scan these specific dirs — instant, no hanging
    let targets = [
        "/home", "/usr", "/var", "/opt", "/tmp",
        "/boot", "/srv", "/root", "/snap",
    ];

    let mut dirs: Vec<(u64, String)> = Vec::new();
    for target in &targets {
        // check dir exists first
        if !std::path::Path::new(target).exists() { continue; }

        // du -sm = summary in MB, -x = same filesystem, 2s timeout via take
        let result = Command::new("du")
            .args(["-s", "-m", "-x", target])
            .output();

        let Ok(out) = result else { continue; };

        let text = String::from_utf8_lossy(&out.stdout);
        if let Some(line) = text.lines().next() {
            let mut parts = line.splitn(2, '\t');
            #[allow(clippy::collapsible_if)]
            if let (Some(size_str), Some(_path)) = (parts.next(), parts.next()) {
                if let Ok(size_mb) = size_str.trim().parse::<u64>() {
                    if size_mb > 0 {
                        dirs.push((size_mb, target.to_string()));
                    }
                }
            }
        }
    }
    // sort biggest first
    dirs.sort_by(|a, b| b.0.cmp(&a.0));

    dirs.iter().take(8).map(|(size_mb, path)| {
        let size_gb = *size_mb as f64 / 1024.0;
        let percent = 0u32; // percent calc done in ui based on total
        DirInfo {
            path: path.clone(),
            size_gb,
            percent,
        }
    }).collect()
}

fn pct(used: u64, total: u64) -> u32 {
    if total == 0 { 0 } else { (used * 100 / total) as u32 }
}

// ── Sorting ───────────────────────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq, Default)]
pub enum SortBy {
    #[default]
    CpuDesc,   // most CPU first (default)
    CpuAsc,    // least CPU first
    MemDesc,   // most memory first
    MemAsc,    // least memory first
    Pid,       // by PID ascending
    Name,      // alphabetical
}

impl SystemStats {
    pub fn sort_processes(&mut self, sort: SortBy) {
        match sort {
            SortBy::CpuDesc => self.processes.sort_by(|a, b|
                b.cpu.partial_cmp(&a.cpu).unwrap_or(std::cmp::Ordering::Equal)),
            SortBy::CpuAsc  => self.processes.sort_by(|a, b|
                a.cpu.partial_cmp(&b.cpu).unwrap_or(std::cmp::Ordering::Equal)),
            SortBy::MemDesc => self.processes.sort_by(|a, b| b.mem_mb.cmp(&a.mem_mb)),
            SortBy::MemAsc  => self.processes.sort_by(|a, b| a.mem_mb.cmp(&b.mem_mb)),
            SortBy::Pid     => self.processes.sort_by(|a, b| a.pid.cmp(&b.pid)),
            SortBy::Name    => self.processes.sort_by(|a, b| a.name.cmp(&b.name)),
        }
    }
}