<div align="center">
  <img src="assets/logo.png" height="256" alt="ktop logo">
  
  <h1 align="center" style="border:0;">ktop 🔥</h1>
  
  <p align="center">A minimalist, cross-platform system monitor for your terminal.</p>

  <p align="center">
   <img src="https://github.com/MahiroJV/ktop/actions/workflows/rust.yml/badge.svg" alt="CI">
    <a href="https://github.com/MahiroJV/ktop/stargazers">
      <img src="https://img.shields.io/github/stars/MahiroJV/ktop">
    </a> 
    <img src="https://img.shields.io/badge/Rust-orange?style=flat&logo=rust&logoColor=white" alt="Rust">
    <img src="https://img.shields.io/badge/Linux-E11837?style=flat&logo=linux&logoColor=white" alt="Linux">
    <img src="https://img.shields.io/github/repo-size/MahiroJV/ktop?style=tyle=flat&color=blue" alt="size">
    <img src="https://img.shields.io/badge/Status-Active-brightgreen?style=flat" alt="Status">
  </p>

  <br>

</div>

# Overview

**koktail's system monitor** — a btop-inspired TUI written in Rust.

<p align="center">
  <img src="assets/Screenshot.png" width="900" alt="ktop system monitor screenshot">
</p>

## Features

- **CPU** — overall usage, per-core bars, frequency
- **Memory** — RAM and swap with colored gauges
- **Disk** — usage % and GB used/total
- **Network** — live ↓↑ KB/s and total transferred
- **Processes** — top 10 by CPU, color-coded by load
- Gauges go green → yellow → red based on usage level
- Graceful handling of small terminals

## Requirements

- Rust (stable) — install from [rustup.rs](https://rustup.rs)
- Linux (reads from `/proc` and `/sys` via sysinfo)

## Install

```bash
git clone https://github.com/MahiroJV/ktop.git
cd ktop
chmod +x install.sh
./install.sh
```

This builds a release binary and copies it to `~/.local/bin/ktop`.

Make sure `~/.local/bin` is in your PATH:

```bash
echo 'export PATH="$HOME/.local/bin:$PATH"' >> ~/.bashrc
source ~/.bashrc
```

Then just run:

```bash
rtop
```

## Manual build

```bash
cargo build --release
./target/release/ktop
```

## Uninstall

```bash
rm ~/.local/bin/ktop
```

## Keybinds

| Key | Action |
|-----|--------|
| `q` | Quit   |
| `c/C`| Sort by CPU usage |
| `m/M`| Sort by Memory usage |
| `p` | Sort by ID |
| `n` | Sort by Name |

## Dependencies

| Crate | Version | Purpose |
|-------|---------|---------|
| [sysinfo](https://crates.io/crates/sysinfo) | 0.29 | System data (CPU, RAM, disk, net, processes) |
| [tui](https://crates.io/crates/tui) | 0.19 | Terminal UI framework |
| [crossterm](https://crates.io/crates/crossterm) | 0.27 | Cross-platform terminal control |

## Project structure

```
ktop/
├── src/
│   ├── main.rs      # event loop + wiring
│   ├── system.rs    # all data collection
│   └── ui.rs        # all TUI rendering
├── Cargo.toml
├── install.sh
└── README.md
```

## License

MIT License © 2026 Mahir
