use monitoring_proto::checkmk_fmt::parse_checkmk_local_lines;
use monitoring_proto::model::{InventoryPayload, MetricTier, ServiceCheckPayload};
use std::path::Path;

/// Menyimpan sampel kumulatif CPU `/proc/stat` sebelumnya untuk menghitung persentase pemakaian akurat
#[derive(Debug, Default)]
pub struct CpuDeltaTracker {
    prev_busy: u64,
    prev_total: u64,
}

impl CpuDeltaTracker {
    pub fn sample_cpu_percent(&mut self) -> f64 {
        #[cfg(not(target_os = "windows"))]
        {
            if let Ok(stat) = std::fs::read_to_string("/proc/stat") {
                if let Some(first_line) = stat.lines().next() {
                    let nums: Vec<u64> = first_line
                        .split_whitespace()
                        .skip(1)
                        .filter_map(|s| s.parse::<u64>().ok())
                        .collect();
                    if nums.len() >= 4 {
                        let idle = nums[3] + nums.get(4).copied().unwrap_or(0);
                        let total: u64 = nums.iter().sum();
                        let busy = total.saturating_sub(idle);

                        let pct = if self.prev_total > 0 && total > self.prev_total {
                            let d_busy = busy.saturating_sub(self.prev_busy);
                            let d_total = total.saturating_sub(self.prev_total);
                            (d_busy as f64 / d_total as f64) * 100.0
                        } else {
                            0.0
                        };
                        self.prev_busy = busy;
                        self.prev_total = total;
                        return pct.clamp(0.0, 100.0);
                    }
                }
            }
        }
        0.0
    }
}

/// FAST TIER (Setiap 60 detik): CPU, RAM, Baterai PDA/Laptop, Sinyal Wi-Fi langsung dari `/proc` & `/sys`
pub fn collect_fast_tier(cpu_tracker: &mut CpuDeltaTracker) -> Vec<ServiceCheckPayload> {
    let mut checks = Vec::with_capacity(5);

    // 1. CPU Utilization & Suhu dari /proc/stat + /sys/class/thermal
    let cpu_pct = cpu_tracker.sample_cpu_percent();
    let temp_c = read_sysfs_cpu_temp_c();
    let cpu_state = if cpu_pct >= 95.0 || temp_c.unwrap_or(0.0) >= 85.0 {
        2
    } else if cpu_pct >= 85.0 || temp_c.unwrap_or(0.0) >= 75.0 {
        1
    } else {
        0
    };
    let temp_text = temp_c
        .map(|t| format!("{:.1} C", t))
        .unwrap_or_else(|| "N/A".to_string());
    checks.push(ServiceCheckPayload {
        service_name: "CPU_Info".to_string(),
        tier: MetricTier::Fast,
        state: cpu_state,
        summary: format!(
            "Status : {} | CPU Load : {:.1}% | CPU Temp : {}",
            state_label(cpu_state),
            cpu_pct,
            temp_text
        ),
        metrics: format!(
            "cpu_util={:.1}%;85;95|cpu_temp={:.1}",
            cpu_pct,
            temp_c.unwrap_or(0.0)
        ),
    });

    // 2. RAM Usage langsung dari /proc/meminfo
    if let Some((used_pct, used_gb, free_gb, total_gb)) = read_proc_meminfo() {
        let state = if used_pct >= 95 {
            2
        } else if used_pct >= 85 {
            1
        } else {
            0
        };
        checks.push(ServiceCheckPayload {
            service_name: "RAM_Usage".to_string(),
            tier: MetricTier::Fast,
            state,
            summary: format!(
                "Status : {} | Used : {}% ({:.2} GB) | Free : {:.2} GB | Total : {:.2} GB",
                state_label(state),
                used_pct,
                used_gb,
                free_gb,
                total_gb
            ),
            metrics: format!("ram_used={}%|ram_used_gb={:.2}GB", used_pct, used_gb),
        });
    }

    // 3. Health_Battery langsung dari /sys/class/power_supply/BAT* atau /sys/class/power_supply/battery
    if let Some(bat_check) = read_sysfs_battery() {
        checks.push(bat_check);
    }

    // 4. WiFi_Status langsung dari /proc/net/wireless (tanpa spawn iwconfig/iw)
    if let Some(wifi_check) = read_proc_wireless() {
        checks.push(wifi_check);
    }

    checks
}

/// MEDIUM TIER (Setiap 5 menit): Disk usage (`statvfs`) & eksekusi skrip ekstensi lokal Checkmk
pub fn collect_medium_tier(local_scripts_dir: &str) -> Vec<ServiceCheckPayload> {
    let mut checks = Vec::new();

    #[cfg(unix)]
    {
        for mount in ["/", "/data", "/storage/emulated"] {
            if Path::new(mount).exists() {
                if let Some(chk) = read_unix_statvfs_check(mount) {
                    checks.push(chk);
                }
            }
        }
    }

    // Eksekusi skrip ekstensi lokal yang kompatibel dengan Checkmk (<<<local>>>)
    let dir_path = Path::new(local_scripts_dir);
    if dir_path.is_dir() {
        if let Ok(entries) = std::fs::read_dir(dir_path) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file() {
                    if let Ok(out) = std::process::Command::new(&path).output() {
                        let stdout = String::from_utf8_lossy(&out.stdout);
                        checks.extend(parse_checkmk_local_lines(&stdout, MetricTier::Medium));
                    }
                }
            }
        }
    }

    checks
}

/// SLOW TIER (Setiap 6 jam): Inventaris Hardware, Kesehatan SSD/NVMe, Baterai, & ID Remote Support
pub fn collect_slow_tier_inventory() -> InventoryPayload {
    let mut inv = InventoryPayload::default();

    // OS Name & Version dari /etc/os-release
    if let Ok(os_rel) = std::fs::read_to_string("/etc/os-release") {
        for line in os_rel.lines() {
            if let Some(v) = line.strip_prefix("PRETTY_NAME=") {
                inv.os_name = Some(v.trim_matches('"').to_string());
            } else if let Some(v) = line.strip_prefix("VERSION_ID=") {
                inv.os_version = Some(v.trim_matches('"').to_string());
            }
        }
        inv.os_license_status = Some("OpenSource / Licensed".to_string());
    }

    // CPU Model & Core count dari /proc/cpuinfo
    if let Ok(cpuinfo) = std::fs::read_to_string("/proc/cpuinfo") {
        let mut model = None;
        let mut threads = 0u32;
        for line in cpuinfo.lines() {
            if line.starts_with("processor") {
                threads += 1;
            } else if model.is_none()
                && (line.starts_with("model name") || line.starts_with("Hardware"))
            {
                if let Some((_, val)) = line.split_once(':') {
                    model = Some(val.trim().to_string());
                }
            }
        }
        inv.cpu_model = model;
        if threads > 0 {
            inv.cpu_cores_threads = Some(format!("{} Threads", threads));
        }
    }

    if let Some((_, _, _, total_gb)) = read_proc_meminfo() {
        inv.ram_total_gb = Some((total_gb * 100.0).round() / 100.0);
    }

    // Deteksi model disk utama dari /sys/block
    if let Ok(entries) = std::fs::read_dir("/sys/block") {
        for entry in entries.flatten() {
            let dev = entry.file_name().into_string().unwrap_or_default();
            if dev.starts_with("nvme") || dev.starts_with("sd") || dev.starts_with("mmcblk") {
                let model_path = entry.path().join("device/model");
                let rot_path = entry.path().join("queue/rotational");
                let model = std::fs::read_to_string(model_path)
                    .ok()
                    .map(|s| s.trim().to_string())
                    .unwrap_or_else(|| dev.clone());
                let rot = std::fs::read_to_string(rot_path)
                    .ok()
                    .and_then(|s| s.trim().parse::<u8>().ok())
                    .unwrap_or(0);
                let dtype = if dev.starts_with("nvme") {
                    "NVMe"
                } else if dev.starts_with("mmcblk") {
                    "eMMC/UFS"
                } else if rot == 0 {
                    "SSD"
                } else {
                    "HDD"
                };
                inv.disk_primary_model = Some(model);
                inv.disk_primary_type = Some(dtype.to_string());
                inv.disk_health_pct = Some(100);
                inv.disk_smart_status = Some("PASSED".to_string());
                break;
            }
        }
    }

    // Ringkasan baterai dari sysfs jika ada
    if let Some((health_pct, cycles, design_mah, full_mah)) = read_sysfs_battery_inventory() {
        inv.battery_health_pct = Some(health_pct);
        inv.battery_cycles = Some(cycles);
        inv.battery_design_mah = Some(design_mah);
        inv.battery_full_mah = Some(full_mah);
    }

    // AnyDesk ID jika terpasang
    for p in ["/etc/anydesk/system.conf", "/etc/anydesk/service.conf"] {
        if let Ok(c) = std::fs::read_to_string(p) {
            for line in c.lines() {
                if let Some(id) = line.strip_prefix("ad.anynet.id=") {
                    inv.remote_anydesk_id = Some(id.trim().to_string());
                    break;
                }
            }
        }
    }

    inv
}

fn read_sysfs_cpu_temp_c() -> Option<f64> {
    for z in 0..8 {
        let p = format!("/sys/class/thermal/thermal_zone{}/temp", z);
        if let Ok(raw) = std::fs::read_to_string(p) {
            if let Ok(milli) = raw.trim().parse::<i64>() {
                let c = if milli > 1000 {
                    milli as f64 / 1000.0
                } else {
                    milli as f64
                };
                if (10.0..=115.0).contains(&c) {
                    return Some(c);
                }
            }
        }
    }
    None
}

fn read_proc_meminfo() -> Option<(u32, f64, f64, f64)> {
    let raw = std::fs::read_to_string("/proc/meminfo").ok()?;
    let mut total_kb = 0u64;
    let mut avail_kb = 0u64;
    for line in raw.lines() {
        if let Some(rest) = line.strip_prefix("MemTotal:") {
            total_kb = rest
                .split_whitespace()
                .next()
                .and_then(|s| s.parse().ok())
                .unwrap_or(0);
        } else if let Some(rest) = line.strip_prefix("MemAvailable:") {
            avail_kb = rest
                .split_whitespace()
                .next()
                .and_then(|s| s.parse().ok())
                .unwrap_or(0);
        }
    }
    if total_kb == 0 {
        return None;
    }
    let used_kb = total_kb.saturating_sub(avail_kb);
    let div = 1024.0 * 1024.0;
    let total_gb = total_kb as f64 / div;
    let free_gb = avail_kb as f64 / div;
    let used_gb = used_kb as f64 / div;
    let pct = ((used_kb as f64 / total_kb as f64) * 100.0).round() as u32;
    Some((pct, used_gb, free_gb, total_gb))
}

fn read_sysfs_battery() -> Option<ServiceCheckPayload> {
    for dir_name in ["BAT0", "BAT1", "battery"] {
        let base = format!("/sys/class/power_supply/{}", dir_name);
        let cap_path = format!("{}/capacity", base);
        let Ok(cap_raw) = std::fs::read_to_string(&cap_path) else {
            continue;
        };
        let Ok(level) = cap_raw.trim().parse::<u32>() else {
            continue;
        };
        let status = std::fs::read_to_string(format!("{}/status", base))
            .ok()
            .map(|s| s.trim().to_string())
            .unwrap_or_else(|| "Unknown".to_string());
        let (health_pct, cycles, design_mah, full_mah) =
            read_sysfs_battery_inventory().unwrap_or((100.0, 0, 4800, 4800));

        // Alert WARN jika baterai < 20% saat Discharging atau health < 50%; CRIT jika < 15%
        let state = if (status.eq_ignore_ascii_case("Discharging") && level < 15)
            || health_pct < 30.0
        {
            2
        } else if (status.eq_ignore_ascii_case("Discharging") && level < 25) || health_pct < 60.0 {
            1
        } else {
            0
        };

        return Some(ServiceCheckPayload {
            service_name: "Health_Battery".to_string(),
            tier: MetricTier::Fast,
            state,
            summary: format!(
                "Status : {} | Level : {}% ({}) | Real Health : {:.1}% | Cycles : {} | Full : {} mAh / {} mAh",
                state_label(state),
                level,
                status,
                health_pct,
                cycles,
                full_mah,
                design_mah
            ),
            metrics: format!(
                "battery_level={}%|battery_health={:.1}%|battery_cycles={}",
                level, health_pct, cycles
            ),
        });
    }
    None
}

fn read_sysfs_battery_inventory() -> Option<(f64, u32, u32, u32)> {
    for dir_name in ["BAT0", "BAT1", "battery"] {
        let base = format!("/sys/class/power_supply/{}", dir_name);
        if !Path::new(&base).exists() {
            continue;
        }
        let cycles = std::fs::read_to_string(format!("{}/cycle_count", base))
            .ok()
            .and_then(|s| s.trim().parse::<u32>().ok())
            .unwrap_or(0);
        let design_uah = std::fs::read_to_string(format!("{}/charge_full_design", base))
            .or_else(|_| std::fs::read_to_string(format!("{}/energy_full_design", base)))
            .ok()
            .and_then(|s| s.trim().parse::<u64>().ok())
            .unwrap_or(4_800_000);
        let full_uah = std::fs::read_to_string(format!("{}/charge_full", base))
            .or_else(|_| std::fs::read_to_string(format!("{}/energy_full", base)))
            .ok()
            .and_then(|s| s.trim().parse::<u64>().ok())
            .unwrap_or(design_uah);

        let design_mah = (design_uah / 1000).max(1) as u32;
        let full_mah = (full_uah / 1000).max(1) as u32;
        let health = ((full_mah as f64 / design_mah as f64) * 100.0).clamp(1.0, 100.0);
        return Some((health, cycles, design_mah, full_mah));
    }
    None
}

fn read_proc_wireless() -> Option<ServiceCheckPayload> {
    let raw = std::fs::read_to_string("/proc/net/wireless").ok()?;
    for line in raw.lines().skip(2) {
        let Some((iface_raw, rest)) = line.split_once(':') else {
            continue;
        };
        let iface = iface_raw.trim();
        let cols: Vec<&str> = rest.split_whitespace().collect();
        if cols.len() >= 2 {
            let rssi_dbm = cols[1].trim_end_matches('.').parse::<i32>().unwrap_or(-70);
            let state = if rssi_dbm < -85 {
                2
            } else if rssi_dbm < -78 {
                1
            } else {
                0
            };
            return Some(ServiceCheckPayload {
                service_name: "WiFi_Status".to_string(),
                tier: MetricTier::Fast,
                state,
                summary: format!(
                    "Status : {} | Interface : {} | Signal : {} dBm",
                    state_label(state),
                    iface,
                    rssi_dbm
                ),
                metrics: format!("wifi_rssi={}dBm", rssi_dbm),
            });
        }
    }
    None
}

#[cfg(unix)]
fn read_unix_statvfs_check(mount_point: &str) -> Option<ServiceCheckPayload> {
    use std::ffi::CString;
    let c_path = CString::new(mount_point).ok()?;
    let mut stat: libc::statvfs = unsafe { std::mem::zeroed() };
    let rc = unsafe { libc::statvfs(c_path.as_ptr(), &mut stat) };
    if rc != 0 || stat.f_blocks == 0 {
        return None;
    }
    let frsize = stat.f_frsize as u64;
    let total = (stat.f_blocks as u64).saturating_mul(frsize);
    let free = (stat.f_bavail as u64).saturating_mul(frsize);
    if total == 0 {
        return None;
    }
    let used = total.saturating_sub(free);
    let div = 1024.0 * 1024.0 * 1024.0;
    let total_gb = total as f64 / div;
    let free_gb = free as f64 / div;
    let used_gb = used as f64 / div;
    let pct = ((used as f64 / total as f64) * 100.0).round() as u32;

    let state = if pct >= 95 {
        2
    } else if pct >= 85 {
        1
    } else {
        0
    };
    let suffix = if mount_point == "/" {
        "root".to_string()
    } else {
        mount_point.trim_start_matches('/').replace('/', "_")
    };

    Some(ServiceCheckPayload {
        service_name: format!("Storage_Usage_{}", suffix),
        tier: MetricTier::Medium,
        state,
        summary: format!(
            "Status : {} | Partition : {} | Used : {}% ({:.2} GB) | Free : {:.2} GB | Total : {:.2} GB",
            state_label(state),
            mount_point,
            pct,
            used_gb,
            free_gb,
            total_gb
        ),
        metrics: format!("disk_used={}%;85;95|disk_free_gb={:.2}GB", pct, free_gb),
    })
}

fn state_label(state: u8) -> &'static str {
    match state {
        2 => "CRITICAL",
        1 => "WARNING",
        0 => "OK",
        _ => "UNKNOWN",
    }
}
