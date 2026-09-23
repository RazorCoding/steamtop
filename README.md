# steamtop

htop/gtop-style system monitor for Steam games, built in Rust as a terminal UI.

Track CPU, memory, GPU utilization and VRAM of your running Steam games — auto-detected or hand-picked — in a single live dashboard.

![demo](https://img.shields.io/badge/platform-linux-lightgrey)
![Rust](https://img.shields.io/badge/rust-1.98%2B-orange)

## Features

- **htop-style TUI** — gauges for system CPU / MEM / GPU+VRAM, sorted process table, hotkey footer
- **Auto Detects Steam games** — processes living under a Steam library (`steamapps/`) or a Proton prefix are picked up automatically (`*`)
- **Manual pinning** — switch to the full process list and pin any process to watch it, even non-Steam games (`+`)
- **Per-process stats** — CPU%, MEM%, GPU% and VRAM% per row, sorted by any column
- **Best-effort GPU backend** — NVIDIA via `nvidia-smi`, AMD/Intel via sysfs (`/sys/class/drm`), graceful "unsupported" fallback
- **Adjustable refresh rate** (0.25s – 5s)

## Install

Requires Rust and a Linux system (process exe/cwd detection needs `/proc`).

```sh
git clone https://github.com/RazorCoding/steamtop.git
cd steamtop
cargo build --release
./target/release/steamtop
```

Or run the dev build:

```sh
cargo run
```

## Usage

Just run it. The **games** view opens by default — launch a game (Steam native or Proton) and it appears automatically.

Press `Tab` to browse **all processes** and pin anything you want to keep an eye on
(great for standalone / launcher game processes Steam doesn't auto-detect).

### Hotkeys

| Key      | Action                          |
|----------|---------------------------------|
| `q`      | Quit                            |
| `Tab`    | Toggle games / all-processes view |
| `Up/Down`| Move selection                  |
| `Space`  | Pin / unpin selected process    |
| `1`      | Sort by CPU                     |
| `2`      | Sort by memory                  |
| `3`      | Sort by GPU                     |
| `4`      | Sort by name                    |
| `+` / `-`| Faster / slower refresh rate    |

### Row markers

| Marker | Meaning                          |
|--------|----------------------------------|
| `*`    | Auto-detected Steam game         |
| `+`    | Manually pinned process          |
| ` `    | Unpinned process (all-procs view)|

## How it works

- **Metrics** — `sysinfo` samples system CPU%, used/total RAM, and per-process CPU%/memory (delta-based, like htop).
- **GPU** — probes in order: `nvidia-smi` (NVIDIA) → sysfs `gpu_busy_percent` + `mem_info_vram_*` (AMD/Intel). If neither exists, the GPU gauge shows its backend as "unsupported".
- **Game detection** — a process is a game if its executable or working directory lives under a Steam library dir or Proton prefix (`steamapps` / `steamlibrary` / `compatdata`). The Steam client and its bundled CEF/Chromium helper processes are excluded.

## Project layout

```
src/
├── main.rs     # event loop, hotkeys, state
├── metrics.rs  # sysinfo sampling
├── games.rs    # Steam game detection + pinning
├── gpu.rs      # best-effort GPU backend
└── ui.rs       # htop-style ratatui rendering
```

## License

MIT