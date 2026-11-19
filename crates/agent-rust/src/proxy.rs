use monitoring_proto::model::{MetricTier, ServiceCheckPayload};
use std::net::{SocketAddr, TcpStream};
use std::process::Command;
use std::time::{Duration, Instant};

/// Konfigurasi target Branch Proxy yang dibaca dari environment variable di 1 host per cabang
#[derive(Debug, Clone)]
pub struct BranchProxyConfig {
    pub mikrotik_host: Option<String>,
    pub ruijie_host: Option<String>,
    pub unifi_controller_url: Option<String>,
    pub printer_hosts: Vec<String>,
    pub inter_branch_ping_targets: Vec<String>,
}

impl BranchProxyConfig {
    pub fn from_env() -> Self {
        let mikrotik_host = std::env::var("PROXY_MIKROTIK_HOST")
            .ok()
            .filter(|s| !s.trim().is_empty());
        let ruijie_host = std::env::var("PROXY_RUIJIE_HOST")
            .ok()
            .filter(|s| !s.trim().is_empty());
        let unifi_controller_url = std::env::var("PROXY_UNIFI_URL")
            .ok()
            .filter(|s| !s.trim().is_empty());
        let printer_hosts = std::env::var("PROXY_PRINTER_HOSTS")
            .unwrap_or_default()
            .split(',')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(String::from)
            .collect();
        let inter_branch_ping_targets = std::env::var("PROXY_PING_TARGETS")
            .unwrap_or_else(|_| "8.8.8.8,192.168.122.10".to_string())
            .split(',')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(String::from)
            .collect();

        Self {
            mikrotik_host,
            ruijie_host,
            unifi_controller_url,
            printer_hosts,
            inter_branch_ping_targets,
        }
    }
}

/// Menjalankan poller jaringan cabang (MikroTik, Ruijie, UniFi, Printer, & ICMP Ping)
/// menggunakan single-threaded runtime jika fitur `branch-proxy` diaktifkan.
pub fn collect_branch_proxy_checks(cfg: &BranchProxyConfig) -> Vec<ServiceCheckPayload> {
    #[cfg(feature = "branch-proxy")]
    {
        if let Ok(rt) = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
        {
            return rt.block_on(async { run_proxy_probes(cfg) });
        }
    }
    run_proxy_probes(cfg)
}

fn run_proxy_probes(cfg: &BranchProxyConfig) -> Vec<ServiceCheckPayload> {
    let mut out = Vec::new();

    if let Some(mt) = &cfg.mikrotik_host {
        out.push(probe_tcp_device("Router_MikroTik", mt, &[8728, 80, 443]));
    }
    if let Some(rj) = &cfg.ruijie_host {
        out.push(probe_tcp_device("Gateway_Ruijie_EG", rj, &[161, 80, 443]));
    }
    if let Some(unifi) = &cfg.unifi_controller_url {
        out.push(probe_tcp_device("Controller_UniFi", unifi, &[8443, 443, 8080]));
    }
    for prn in &cfg.printer_hosts {
        let clean = prn.replace('.', "_");
        out.push(probe_tcp_device(
            &format!("Printer_{}", clean),
            prn,
            &[9100, 631, 80],
        ));
    }
    for target in &cfg.inter_branch_ping_targets {
        out.push(probe_icmp_target(target));
    }

    out
}

fn probe_tcp_device(service_name: &str, host: &str, ports: &[u16]) -> ServiceCheckPayload {
    let host_only = host
        .trim_start_matches("http://")
        .trim_start_matches("https://")
        .split(':')
        .next()
        .unwrap_or(host);

    for &port in ports {
        let addr_str = format!("{}:{}", host_only, port);
        if let Ok(addr) = addr_str.parse::<SocketAddr>() {
            let start = Instant::now();
            if TcpStream::connect_timeout(&addr, Duration::from_millis(1500)).is_ok() {
                let rtt_ms = start.elapsed().as_secs_f64() * 1000.0;
                return ServiceCheckPayload {
                    service_name: service_name.to_string(),
                    tier: MetricTier::Proxy,
                    state: 0,
                    summary: format!(
                        "Status : OK | Target : {}:{} reachable ({:.1} ms)",
                        host_only, port, rtt_ms
                    ),
                    metrics: format!("rtt={:.1}ms", rtt_ms),
                };
            }
        }
    }

    ServiceCheckPayload {
        service_name: service_name.to_string(),
        tier: MetricTier::Proxy,
        state: 2,
        summary: format!("Status : CRITICAL | Perangkat {} tidak merespons", host_only),
        metrics: "rtt=0ms".to_string(),
    }
}

fn probe_icmp_target(target_ip: &str) -> ServiceCheckPayload {
    let args = if cfg!(target_os = "windows") {
        vec!["-n", "2", "-w", "1000", target_ip]
    } else {
        vec!["-c", "2", "-W", "1", target_ip]
    };
    let start = Instant::now();
    let ok = Command::new("ping")
        .args(&args)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);
    let elapsed_ms = start.elapsed().as_secs_f64() * 1000.0 / 2.0;

    let svc = format!("Ping_{}", target_ip.replace('.', "_"));
    if ok {
        ServiceCheckPayload {
            service_name: svc,
            tier: MetricTier::Proxy,
            state: 0,
            summary: format!(
                "Status : OK | Target : {} | Avg RTT : {:.1} ms",
                target_ip, elapsed_ms
            ),
            metrics: format!("rta={:.1}ms|pl=0%", elapsed_ms),
        }
    } else {
        ServiceCheckPayload {
            service_name: svc,
            tier: MetricTier::Proxy,
            state: 2,
            summary: format!("Status : CRITICAL | Target : {} Timeout (100% loss)", target_ip),
            metrics: "rta=0ms|pl=100%".to_string(),
        }
    }
}
