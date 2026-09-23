use std::collections::{HashMap, HashSet};
use std::path::Path;

use sysinfo::Process;

use crate::metrics::Metrics;

#[derive(Clone, Debug)]
pub struct Game {
    pub pid: u32,
    pub name: String,
    pub cpu: f32,
    pub mem_bytes: u64,
    pub pinned: bool,
    pub steam: bool,
}

/// A process is a real Steam game only if it lives under a Steam library dir
/// ("steamapps" for native games, or a Proton prefix inside "steamapps").
/// The Steam client itself and its bundled CEF/Chromium helper processes are
/// deliberately excluded (their paths contain "steam" too).
fn in_steamapps(path: &Path) -> bool {
    let s = path.to_string_lossy().to_lowercase();
    s.contains("steamapps") || s.contains("steamlibrary")
}

pub fn looks_like_game(p: &Process) -> bool {
    let name = p.name().to_string_lossy().to_lowercase();
    if name.contains("steam") {
        return false;
    }
    let in_game_dir = |path: Option<&Path>| path.map(in_steamapps).unwrap_or(false);
    in_game_dir(p.exe()) || in_game_dir(p.cwd())
}

/// Builds the combined game list: auto-detected Steam games plus user-pinned
/// processes from the full process view.
pub fn collect_games(
    metrics: &Metrics,
    pinned: &HashSet<u32>,
    watch_names: &HashMap<u32, String>,
) -> Vec<Game> {
    let mut games: Vec<Game> = metrics
        .processes()
        .iter()
        .filter_map(|(pid, p)| {
            let u = pid.as_u32();
            let is_pinned = pinned.contains(&u);
            let auto = !is_pinned && looks_like_game(p);
            if !is_pinned && !auto {
                return None;
            }
            Some(Game {
                pid: u,
                name: p.name().to_string_lossy().into_owned(),
                cpu: p.cpu_usage(),
                mem_bytes: p.memory(),
                pinned: is_pinned,
                steam: auto,
            })
        })
        .collect();

    // Add pinned entries whose process no longer exists (shown as stale).
    for (&pid, name) in watch_names {
        if pinned.contains(&pid) && !games.iter().any(|g| g.pid == pid) {
            games.push(Game {
                pid,
                name: format!("{name} (gone)"),
                cpu: 0.0,
                mem_bytes: 0,
                pinned: true,
                steam: false,
            });
        }
    }
    games
}
