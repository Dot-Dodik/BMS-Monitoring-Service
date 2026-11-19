mod collectors;
mod enroll;
mod identity;
mod proxy;
mod ring_buffer;

use collectors::{
    collect_fast_tier, collect_medium_tier, collect_slow_tier_inventory, CpuDeltaTracker,
};
use enroll::AgentTransportClient;
use identity::detect_device_identity;
use monitoring_proto::checkmk_fmt::format_checkmk_output;
use monitoring_proto::model::IngestPayload;
use proxy::{collect_branch_proxy_checks, BranchProxyConfig};
use ring_buffer::InMemoryRingBuffer;
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// Menghasilkan nilai pseudo-random deterministik ringan [-5000..=+5000] ms tanpa crate berat `rand`
fn compute_jitter_millis(host_uuid: &str, cycle: u64) -> i64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325 ^ cycle.wrapping_mul(0x9e37_79b9_7f4a_7c15);
    for b in host_uuid.bytes() {
        hash ^= u64::from(b);
        hash = hash.wrapping_mul(0x0100_0000_01b3);
    }
    ((hash % 10_001) as i64) - 5_000
}

fn current_epoch_sec() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let dump_only = args.iter().any(|a| a == "--dump" || a == "dump");
    let proxy_flag = args.iter().any(|a| a == "--branch-proxy")
        || std::env::var("AGENT_BRANCH_PROXY_ENABLED")
            .map(|v| v.eq_ignore_ascii_case("true") || v == "1")
            .unwrap_or(false);

    let server_url =
        std::env::var("AGENT_SERVER_URL").unwrap_or_else(|_| "http://127.0.0.1:8080".to_string());
    let branch_code =
        std::env::var("AGENT_BRANCH_CODE").unwrap_or_else(|_| "KDR-HQ".to_string());
    let enrollment_token = std::env::var("AGENT_ENROLLMENT_TOKEN")
        .unwrap_or_else(|_| "enroll-kdr-hq-secret-token-2026".to_string());
    let secret_file = std::env::var("AGENT_SECRET_FILE")
        .unwrap_or_else(|_| "/var/lib/cmk-agent/agent.secret".to_string());
    let local_scripts_dir = std::env::var("AGENT_LOCAL_SCRIPTS_DIR")
        .unwrap_or_else(|_| "/usr/lib/check_mk_agent/local".to_string());

    let mut cpu_tracker = CpuDeltaTracker::default();
    let _ = cpu_tracker.sample_cpu_percent();
    thread::sleep(Duration::from_millis(250));

    let id = detect_device_identity();
    let proxy_cfg = BranchProxyConfig::from_env();

    if dump_only {
        let mut checks = collect_fast_tier(&mut cpu_tracker);
        checks.extend(collect_medium_tier(&local_scripts_dir));
        if proxy_flag {
            checks.extend(collect_branch_proxy_checks(&proxy_cfg));
        }
        let raw = format_checkmk_output(
            env!("CARGO_PKG_VERSION"),
            id.host_type.as_db_str(),
            &id.hostname,
            &checks,
        );
        print!("{}", raw);
        return;
    }

    eprintln!(
        "[cmk-agent] Starting Ultra-Light Agent v{} | UUID={} | Host={} | Branch={} | Proxy={}",
        env!("CARGO_PKG_VERSION"),
        id.host_uuid,
        id.hostname,
        branch_code,
        proxy_flag
    );

    let mut client =
        AgentTransportClient::new(server_url, branch_code, enrollment_token, secret_file);
    let mut ring_buffer = InMemoryRingBuffer::default();
    let mut cycle: u64 = 0;

    loop {
        let id_now = detect_device_identity();
        let mut services = collect_fast_tier(&mut cpu_tracker);

        // Medium Tier: setiap 5 siklus (5 menit) atau saat startup (cycle == 0)
        if cycle % 5 == 0 {
            services.extend(collect_medium_tier(&local_scripts_dir));
        }

        // Branch Proxy Poller (jika diaktifkan)
        if proxy_flag {
            services.extend(collect_branch_proxy_checks(&proxy_cfg));
        }

        // Slow Tier Inventory: setiap 360 siklus (6 jam) atau saat startup (cycle == 0)
        let inventory = if cycle % 360 == 0 {
            Some(collect_slow_tier_inventory())
        } else {
            None
        };

        let checkmk_raw = Some(format_checkmk_output(
            env!("CARGO_PKG_VERSION"),
            id_now.host_type.as_db_str(),
            &id_now.hostname,
            &services,
        ));

        let payload = IngestPayload {
            host_uuid: id_now.host_uuid.clone(),
            hostname: id_now.hostname.clone(),
            ip_address: id_now.ip_address.clone(),
            mac_address: id_now.mac_address.clone(),
            host_type: id_now.host_type.clone(),
            agent_version: env!("CARGO_PKG_VERSION").to_string(),
            timestamp_sec: current_epoch_sec(),
            services,
            inventory,
            checkmk_raw,
        };

        // 1. Jika ada antrean paket di Ring Buffer dari saat koneksi putus, kirim dulu berurutan
        while let Some(buffered) = ring_buffer.peek_front().cloned() {
            if client.send_ingest(&id_now, &buffered).is_ok() {
                ring_buffer.pop_front();
            } else {
                break;
            }
        }

        // 2. Kirim paket siklus saat ini; jika gagal, masukkan ke Ring Buffer RAM (maks 50 paket)
        if let Err(err) = client.send_ingest(&id_now, &payload) {
            ring_buffer.push(payload);
            eprintln!(
                "[cmk-agent] Push gagal ({}), paket disimpan di RAM RingBuffer ({}/50, dropped={})",
                err,
                ring_buffer.len(),
                ring_buffer.dropped_count()
            );
        }

        cycle = cycle.wrapping_add(1);

        // Fast Tier 60 detik + Random Jitter (±5 detik) untuk mencegah Thundering Herd 3.000 device
        let jitter_ms = compute_jitter_millis(&id_now.host_uuid, cycle);
        let sleep_ms = (60_000i64 + jitter_ms).clamp(55_000, 65_000) as u64;
        thread::sleep(Duration::from_millis(sleep_ms));
    }
}
