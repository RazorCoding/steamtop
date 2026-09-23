use std::fs;
use std::path::Path;
use std::process::Command;

use glob::glob;

#[derive(Clone, Debug, Default)]
pub struct GpuInfo {
    pub backend: &'static str,
    pub name: Option<String>,
    pub utilization: Option<u8>,
    pub vram_used_mib: Option<u64>,
    pub vram_total_mib: Option<u64>,
}

impl GpuInfo {
    pub fn utilization(&self) -> Option<u8> {
        self.utilization.map(|u| u.min(100))
    }

    pub fn vram_ratio(&self) -> Option<f64> {
        match (self.vram_used_mib, self.vram_total_mib) {
            (Some(u), Some(t)) if t > 0 => Some((u as f64 / t as f64).clamp(0.0, 1.0)),
            _ => None,
        }
    }

    /// Probe order: nvidia-smi first (NVIDIA), then sysfs (AMD/Intel), then unsupported.
    pub fn probe() -> Self {
        detect_nvidia().or_else(sysfs_probe).unwrap_or(Self {
            backend: "unsupported",
            ..Default::default()
        })
    }
}

fn read_file(path: &Path) -> Option<String> {
    fs::read_to_string(path)
        .ok()
        .map(|s| s.trim().to_string())
}

fn to_mib(bytes: u64) -> u64 {
    bytes / 1024 / 1024
}

fn detect_nvidia() -> Option<GpuInfo> {
    let out = Command::new("nvidia-smi")
        .args([
            "--query-gpu=name,utilization.gpu,memory.used,memory.total",
            "--format=csv,noheader,nounits",
        ])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let line = String::from_utf8_lossy(&out.stdout).lines().next()?.trim().to_string();
    if line.is_empty() {
        return None;
    }
    let parts: Vec<&str> = line.split(',').map(str::trim).collect();
    if parts.len() < 4 {
        return None;
    }
    Some(GpuInfo {
        backend: "nvidia-smi",
        name: Some(parts[0].to_string()),
        utilization: parts[1].parse().ok(),
        vram_used_mib: parts[2].parse().ok(),
        vram_total_mib: parts[3].parse().ok(),
    })
}

fn sysfs_probe() -> Option<GpuInfo> {
    for entry in glob("/sys/class/drm/card*/device").ok()?.flatten() {
        let vendor = read_file(&entry.join("vendor")).unwrap_or_default();
        let utilization = read_file(&entry.join("gpu_busy_percent")).and_then(|s| s.parse().ok());
        let vram_total_mib = read_file(&entry.join("mem_info_vram_total"))
            .and_then(|s| s.parse::<u64>().ok())
            .map(to_mib);
        let vram_used_mib = read_file(&entry.join("mem_info_vram_used"))
            .and_then(|s| s.parse::<u64>().ok())
            .map(to_mib);
        if utilization.is_none() && vram_used_mib.is_none() {
            continue;
        }
        let name = match vendor.as_str() {
            "0x1002" => "AMD GPU",
            "0x8086" => "Intel GPU",
            "0x10de" => "NVIDIA GPU",
            _ => "GPU",
        };
        return Some(GpuInfo {
            backend: "sysfs",
            name: Some(name.to_string()),
            utilization,
            vram_used_mib,
            vram_total_mib,
        });
    }
    None
}
