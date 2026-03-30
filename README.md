

---

```markdown
# ktop 🔥

A **simple system monitor** written in **Rust** with a **TUI interface**, inspired by `btop`.  
Displays **CPU**, **RAM**, and **top processes** in a clean, colorful terminal UI.

---

## Features

- CPU usage with colored gauge
- RAM usage in GB with colored gauge
- Top 10 processes sorted by CPU usage
- Smooth terminal UI using `tui` crate
- Quit easily by pressing `q`


````
---
## Screenshot
```bash

==== ktop ====
CPU: [#####-----] 33%
RAM: [####------] 6.87 / 15.37 GB

Top Processes:
PID     | Name                 | CPU
76428   | rustrover            | 72.91%
85212   | TerminalEmulato      | 27.59%
...

---
```
---
## Installation

1. Clone the repository:

```bash
git clone https://github.com/USERNAME/ktop.git
cd ktop
````

2. Build and run:

```bash
cargo run
```

---

## Dependencies

* [Rust](https://www.rust-lang.org/)
* [sysinfo](https://crates.io/crates/sysinfo)
* [tui](https://crates.io/crates/tui)
* [crossterm](https://crates.io/crates/crossterm)

---

## Usage

* Launch: `cargo run`
* Quit: Press `q`
* Displays CPU, RAM, and top processes
* Updates every 0.5 seconds

---

## Contributing

Pull requests are welcome!

If you want to add features like:

* Disk / Network usage
* Process scrolling
* Custom themes / colors

Feel free to fork and submit changes.

---

## License

MIT License © 2026 Mahir


