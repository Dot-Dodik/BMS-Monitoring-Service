use monitoring_proto::model::HostType;
use std::net::UdpSocket;
use std::path::Path;

#[derive(Debug, Clone)]
pub struct DeviceIdentity {
    pub host_uuid: String,
    pub hostname: String,
    pub ip_address: String,
    pub mac_address: Option<String>,
    pub host_type: HostType,
}

pub fn detect_device_identity() -> DeviceIdentity {
    let hostname = detect_hostname();
    let host_uuid = detect_hardware_uuid(&hostname);
    let ip_address = detect_outbound_ipv4();
    let mac_address = detect_primary_mac();
    let host_type = detect_host_type();

    DeviceIdentity {
        host_uuid,
        hostname,
        ip_address,
        mac_address,
        host_type,
    }
}

fn detect_host_type() -> HostType {
    if cfg!(target_os = "android") || Path::new("/system/build.prop").exists() {
        HostType::Android
    } else if cfg!(target_os = "windows") {
        HostType::Windows
    } else {
        HostType::Linux
    }
}

fn detect_hostname() -> String {
    if let Ok(env_host) = std::env::var("AGENT_HOSTNAME") {
        let trimmed = env_host.trim();
        if !trimmed.is_empty() {
            return sanitize_token(trimmed);
        }
    }
    #[cfg(not(target_os = "windows"))]
    {
        if let Ok(raw) = std::fs::read_to_string("/proc/sys/kernel/hostname") {
            let trimmed = raw.trim();
            if !trimmed.is_empty() {
                return sanitize_token(trimmed);
            }
        }
        if let Ok(raw) = std::fs::read_to_string("/etc/hostname") {
            let trimmed = raw.trim();
            if !trimmed.is_empty() {
                return sanitize_token(trimmed);
            }
        }
    }
    if let Ok(comp) = std::env::var("COMPUTERNAME") {
        let trimmed = comp.trim();
        if !trimmed.is_empty() {
            return sanitize_token(trimmed);
        }
    }
    "UNKNOWN-HOST".to_string()
}

fn detect_hardware_uuid(fallback_hostname: &str) -> String {
    if let Ok(explicit) = std::env::var("AGENT_HOST_UUID") {
        let t = explicit.trim();
        if !t.is_empty() {
            return sanitize_token(t);
        }
    }

    // 1. Linux: /etc/machine-id atau /var/lib/dbus/machine-id atau /sys/class/dmi/id/product_uuid
    #[cfg(not(target_os = "windows"))]
    {
        for path in [
            "/etc/machine-id",
            "/var/lib/dbus/machine-id",
            "/sys/class/dmi/id/product_uuid",
        ] {
            if let Ok(content) = std::fs::read_to_string(path) {
                let clean = sanitize_token(content.trim());
                if clean.len() >= 8 {
                    return clean.to_lowercase();
                }
            }
        }

        // 2. Android PDA: baca boot_id / serial / settings secure android_id jika tersedia
        if let Ok(out) = std::process::Command::new("settings")
            .args(["get", "secure", "android_id"])
            .output()
        {
            let aid = String::from_utf8_lossy(&out.stdout).trim().to_string();
            if aid.len() >= 8 && aid != "null" {
                return format!("and-{}", sanitize_token(&aid).to_lowercase());
            }
        }
    }

    // 3. Windows: HKLM\SOFTWARE\Microsoft\Cryptography\MachineGuid
    #[cfg(target_os = "windows")]
    {
        use winreg::enums::HKEY_LOCAL_MACHINE;
        use winreg::RegKey;
        let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
        if let Ok(crypto) = hklm.open_subkey(r"SOFTWARE\Microsoft\Cryptography") {
            if let Ok(guid) = crypto.get_value::<String, _>("MachineGuid") {
                let clean = sanitize_token(&guid);
                if clean.len() >= 8 {
                    return clean.to_lowercase();
                }
            }
        }
    }

    // Fallback deterministik berbasis MAC + Hostname
    let mac = detect_primary_mac().unwrap_or_else(|| "000000000000".to_string());
    format!(
        "fallback-{}-{}",
        sanitize_token(&mac).to_lowercase(),
        fallback_hostname.to_lowercase()
    )
}

pub fn detect_outbound_ipv4() -> String {
    // Trik UDP connect tanpa mengirim paket nyata untuk mengetahui IP interface utama
    if let Ok(sock) = UdpSocket::bind("0.0.0.0:0") {
        if sock.connect("192.168.122.10:80").is_ok() || sock.connect("8.8.8.8:53").is_ok() {
            if let Ok(local_addr) = sock.local_addr() {
                return local_addr.ip().to_string();
            }
        }
    }
    "127.0.0.1".to_string()
}

pub fn detect_primary_mac() -> Option<String> {
    #[cfg(not(target_os = "windows"))]
    {
        if let Ok(entries) = std::fs::read_dir("/sys/class/net") {
            let mut candidates: Vec<String> = entries
                .flatten()
                .filter_map(|e| e.file_name().into_string().ok())
                .filter(|name| {
                    name != "lo"
                        && !name.starts_with("docker")
                        && !name.starts_with("veth")
                        && !name.starts_with("br-")
                        && !name.starts_with("tun")
                })
                .collect();
            candidates.sort();
            for iface in candidates {
                let addr_path = format!("/sys/class/net/{}/address", iface);
                if let Ok(mac) = std::fs::read_to_string(addr_path) {
                    let clean = mac.trim().to_uppercase();
                    if clean.len() == 17 && clean != "00:00:00:00:00:00" {
                        return Some(clean);
                    }
                }
            }
        }
    }
    None
}

fn sanitize_token(raw: &str) -> String {
    raw.chars()
        .filter(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
        .collect()
}
