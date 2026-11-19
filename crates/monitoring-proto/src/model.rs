use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum HostType {
    #[serde(rename = "LINUX")]
    Linux,
    #[serde(rename = "WINDOWS")]
    Windows,
    #[serde(rename = "ANDROID", alias = "ANDROID_PDA")]
    Android,
    #[serde(rename = "ROUTER")]
    Router,
    #[serde(rename = "SWITCH", alias = "SWITCH_AP")]
    Switch,
    #[serde(rename = "FIREWALL")]
    Firewall,
    #[serde(rename = "PRINTER")]
    Printer,
    #[serde(untagged)]
    Custom(String),
}

impl HostType {
    pub fn as_db_str(&self) -> &str {
        match self {
            HostType::Linux => "LINUX",
            HostType::Windows => "WINDOWS",
            HostType::Android => "ANDROID",
            HostType::Router => "ROUTER",
            HostType::Switch => "SWITCH",
            HostType::Firewall => "FIREWALL",
            HostType::Printer => "PRINTER",
            HostType::Custom(s) => s.as_str(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum MetricTier {
    Fast,
    Medium,
    Slow,
    Proxy,
}

impl MetricTier {
    pub fn as_db_str(&self) -> &'static str {
        match self {
            MetricTier::Fast => "FAST",
            MetricTier::Medium => "MEDIUM",
            MetricTier::Slow => "SLOW",
            MetricTier::Proxy => "PROXY",
        }
    }
}

/// State service selaras dengan standar Nagios/Checkmk:
/// 0 = OK, 1 = WARN, 2 = CRIT, 3 = UNKNOWN
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceCheckPayload {
    pub service_name: String,
    pub tier: MetricTier,
    pub state: u8,
    pub summary: String,
    pub metrics: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct InventoryPayload {
    pub os_name: Option<String>,
    pub os_version: Option<String>,
    pub os_license_status: Option<String>,
    pub os_license_key_partial: Option<String>,
    pub office_product: Option<String>,
    pub office_license_status: Option<String>,
    pub cpu_model: Option<String>,
    pub cpu_cores_threads: Option<String>,
    pub ram_total_gb: Option<f64>,
    pub ram_slots_summary: Option<String>,
    pub disk_primary_model: Option<String>,
    pub disk_primary_type: Option<String>,
    pub disk_health_pct: Option<u8>,
    pub disk_tbw_written_tb: Option<f64>,
    pub disk_smart_status: Option<String>,
    pub battery_health_pct: Option<f64>,
    pub battery_cycles: Option<u32>,
    pub battery_design_mah: Option<u32>,
    pub battery_full_mah: Option<u32>,
    pub remote_anydesk_id: Option<String>,
    pub remote_rustdesk_id: Option<String>,
}

/// Request Fase 1: Pendaftaran otomatis agen baru menggunakan ENROLLMENT_TOKEN cabang
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnrollRequest {
    pub branch_code: String,
    pub enrollment_token: String,
    pub host_uuid: String,
    pub hostname: String,
    pub ip_address: String,
    pub mac_address: Option<String>,
    pub host_type: HostType,
    pub agent_version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnrollResponse {
    pub host_uuid: String,
    pub agent_secret: String,
    pub approval_status: String,
}

/// Request Fase 2: Pengiriman metrik periodik (ditandatangani dengan HMAC-SHA256)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IngestPayload {
    pub host_uuid: String,
    pub hostname: String,
    pub ip_address: String,
    pub mac_address: Option<String>,
    pub host_type: HostType,
    pub agent_version: String,
    pub timestamp_sec: i64,
    pub services: Vec<ServiceCheckPayload>,
    pub inventory: Option<InventoryPayload>,
    pub checkmk_raw: Option<String>,
}

/// Envelope standar untuk seluruh respons REST API
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiEnvelope<T> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<String>,
}

impl<T> ApiEnvelope<T> {
    pub fn ok(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
        }
    }

    pub fn err(msg: impl Into<String>) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(msg.into()),
        }
    }
}
