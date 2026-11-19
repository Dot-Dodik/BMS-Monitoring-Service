use crate::config::ServerConfig;
use dashmap::DashMap;
use monitoring_proto::model::{MetricTier, ServiceCheckPayload};
use serde::{Deserialize, Serialize};
use sqlx::MySqlPool;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};

#[derive(Debug, Clone)]
pub struct CachedServiceState {
    pub service_name: String,
    pub tier: MetricTier,
    pub state: u8,
    pub prev_state: u8,
    pub consecutive_non_ok: u8,
    pub notified_problem: bool,
    pub is_acknowledged: bool,
    pub summary: String,
    pub metrics: String,
    pub last_check_epoch: i64,
    pub last_state_change_epoch: i64,
    pub dirty_for_batch: bool,
}

#[derive(Debug, Clone)]
pub struct CachedHostState {
    pub host_id: u64,
    pub host_uuid: String,
    pub branch_id: u32,
    pub branch_code: String,
    pub hostname: String,
    pub ip_address: String,
    pub mac_address: Option<String>,
    pub host_type: String,
    pub agent_secret_hmac: String,
    pub approval_status: String,
    pub status: String, // UP, WARN, CRIT, DOWN, PENDING
    pub is_in_downtime: bool,
    pub owner_name: Option<String>,
    pub owner_wa: Option<String>,
    pub owner_email: Option<String>,
    pub user_alert_services: String,
    pub telegram_chat_id: Option<String>,
    pub telegram_topic_id: Option<i32>,
    pub last_seen_epoch: i64,
    pub dirty_heartbeat: bool,
    pub services: HashMap<String, CachedServiceState>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AlertEventType {
    Problem,
    Recovery,
}

impl AlertEventType {
    pub fn as_db_str(&self) -> &'static str {
        match self {
            AlertEventType::Problem => "PROBLEM",
            AlertEventType::Recovery => "RECOVERY",
        }
    }
}

#[derive(Debug, Clone)]
pub struct AlertEvent {
    pub host_id: u64,
    pub branch_code: String,
    pub hostname: String,
    pub ip_address: String,
    pub service_name: String,
    pub state: u8,
    pub event_type: AlertEventType,
    pub summary: String,
    pub telegram_chat_id: Option<String>,
    pub telegram_topic_id: Option<i32>,
    pub owner_name: Option<String>,
    pub owner_wa: Option<String>,
    pub owner_email: Option<String>,
    pub is_user_routable: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeSettings {
    pub rest_api_public_url: String,
    pub default_enrollment_token: String,
    pub batch_flush_interval_sec: u64,
    pub host_stale_threshold_sec: i64,
    pub flapping_threshold_cycles: u8,
    pub telegram_bot_token: String,
    pub telegram_default_chat_id: String,
    pub wa_webhook_url: String,
    pub wa_webhook_token: String,
    pub smtp_host: String,
    pub smtp_port: u16,
    pub smtp_username: String,
    pub smtp_password: String,
    pub smtp_from: String,
    pub smtp_use_tls: bool,
}

impl From<&ServerConfig> for RuntimeSettings {
    fn from(cfg: &ServerConfig) -> Self {
        Self {
            rest_api_public_url: format!("http://{}", cfg.bind_addr),
            default_enrollment_token: cfg.default_enrollment_token.clone(),
            batch_flush_interval_sec: cfg.batch_flush_interval_sec,
            host_stale_threshold_sec: cfg.host_stale_threshold_sec,
            flapping_threshold_cycles: cfg.flapping_threshold_cycles,
            telegram_bot_token: cfg.telegram_bot_token.clone(),
            telegram_default_chat_id: cfg.telegram_default_chat_id.clone(),
            wa_webhook_url: cfg.wa_webhook_url.clone(),
            wa_webhook_token: cfg.wa_webhook_token.clone(),
            smtp_host: cfg.smtp_host.clone(),
            smtp_port: cfg.smtp_port,
            smtp_username: cfg.smtp_username.clone(),
            smtp_password: cfg.smtp_password.clone(),
            smtp_from: cfg.smtp_from.clone(),
            smtp_use_tls: true,
        }
    }
}

pub struct AppState {
    pub cfg: ServerConfig,
    pub runtime_settings: RwLock<RuntimeSettings>,
    pub db: MySqlPool,
    /// Keyed by `host_uuid` untuk lookup O(1) tanpa menyentuh MariaDB pada jalur panas ingest
    pub hosts_cache: DashMap<String, CachedHostState>,
    pub alert_tx: mpsc::Sender<AlertEvent>,
}

impl AppState {
    pub fn new(cfg: ServerConfig, db: MySqlPool, alert_tx: mpsc::Sender<AlertEvent>) -> Arc<Self> {
        let rt = RuntimeSettings::from(&cfg);
        Arc::new(Self {
            cfg,
            runtime_settings: RwLock::new(rt),
            db,
            hosts_cache: DashMap::with_capacity(4096),
            alert_tx,
        })
    }

    /// Mengecek apakah suatu `service_name` termasuk dalam filter `user_alert_services` milik host
    pub fn matches_user_alert_filter(filter_csv: &str, service_name: &str) -> bool {
        let svc_lower = service_name.to_ascii_lowercase();
        filter_csv
            .split(',')
            .map(str::trim)
            .filter(|t| !t.is_empty())
            .any(|pattern| {
                let pat = pattern.to_ascii_lowercase();
                svc_lower == pat || svc_lower.starts_with(&pat)
            })
    }

    /// Memproses pembaruan service di RAM (`DashMap`).
    /// Mengembalikan `(immediate_db_writes, alerts_to_dispatch)`:
    /// - Jika state tetap sama (`OK -> OK`), hanya set `dirty_for_batch = true` di RAM (0 query DB!).
    /// - Jika state berubah atau melewati ambang anti-flapping, kembalikan item untuk ditulis detik itu juga.
    pub fn apply_ingest_in_memory(
        &self,
        host_uuid: &str,
        hostname: &str,
        ip_address: &str,
        now_sec: i64,
        incoming: &[ServiceCheckPayload],
    ) -> Option<(u64, String, Vec<CachedServiceState>, Vec<AlertEvent>)> {
        let mut host_ref = self.hosts_cache.get_mut(host_uuid)?;
        let host_entry = host_ref.value_mut();
        host_entry.hostname = hostname.to_string();
        host_entry.ip_address = ip_address.to_string();
        host_entry.last_seen_epoch = now_sec;
        host_entry.dirty_heartbeat = true;

        let was_down = host_entry.status == "DOWN";
        let mut immediate_writes = Vec::new();
        let mut alerts = Vec::new();
        let flap_limit = self.cfg.flapping_threshold_cycles.max(1);

        for item in incoming {
            let new_state = item.state.min(3);
            let entry = host_entry
                .services
                .entry(item.service_name.clone())
                .or_insert_with(|| CachedServiceState {
                    service_name: item.service_name.clone(),
                    tier: item.tier,
                    state: 0,
                    prev_state: 0,
                    consecutive_non_ok: 0,
                    notified_problem: false,
                    is_acknowledged: false,
                    summary: String::new(),
                    metrics: String::new(),
                    last_check_epoch: now_sec,
                    last_state_change_epoch: now_sec,
                    dirty_for_batch: true,
                });

            let old_state = entry.state;
            entry.prev_state = old_state;
            entry.state = new_state;
            entry.tier = item.tier;
            entry.summary = item.summary.clone();
            entry.metrics = item.metrics.clone();
            entry.last_check_epoch = now_sec;
            if old_state != new_state {
                entry.last_state_change_epoch = now_sec;
                if new_state == 0 {
                    entry.is_acknowledged = false;
                }
            }

            if new_state == 0 {
                let needed_recovery = entry.notified_problem;
                let state_changed = old_state != 0;
                entry.consecutive_non_ok = 0;
                entry.notified_problem = false;

                if state_changed {
                    entry.dirty_for_batch = false;
                    immediate_writes.push(entry.clone());
                } else {
                    // Tetap OK -> tahan di RAM saja, flush tiap 3 menit
                    entry.dirty_for_batch = true;
                }

                if needed_recovery && !host_entry.is_in_downtime {
                    alerts.push(AlertEvent {
                        host_id: host_entry.host_id,
                        branch_code: host_entry.branch_code.clone(),
                        hostname: host_entry.hostname.clone(),
                        ip_address: host_entry.ip_address.clone(),
                        service_name: item.service_name.clone(),
                        state: 0,
                        event_type: AlertEventType::Recovery,
                        summary: item.summary.clone(),
                        telegram_chat_id: host_entry.telegram_chat_id.clone(),
                        telegram_topic_id: host_entry.telegram_topic_id,
                        owner_name: host_entry.owner_name.clone(),
                        owner_wa: host_entry.owner_wa.clone(),
                        owner_email: host_entry.owner_email.clone(),
                        is_user_routable: Self::matches_user_alert_filter(
                            &host_entry.user_alert_services,
                            &item.service_name,
                        ),
                    });
                }
            } else {
                entry.consecutive_non_ok = entry.consecutive_non_ok.saturating_add(1);
                let state_changed = old_state != new_state;
                let just_reached_flap =
                    entry.consecutive_non_ok >= flap_limit && !entry.notified_problem;

                if state_changed || just_reached_flap {
                    entry.dirty_for_batch = false;
                    immediate_writes.push(entry.clone());
                } else {
                    entry.dirty_for_batch = true;
                }

                if just_reached_flap && !host_entry.is_in_downtime {
                    entry.notified_problem = true;
                    alerts.push(AlertEvent {
                        host_id: host_entry.host_id,
                        branch_code: host_entry.branch_code.clone(),
                        hostname: host_entry.hostname.clone(),
                        ip_address: host_entry.ip_address.clone(),
                        service_name: item.service_name.clone(),
                        state: new_state,
                        event_type: AlertEventType::Problem,
                        summary: item.summary.clone(),
                        telegram_chat_id: host_entry.telegram_chat_id.clone(),
                        telegram_topic_id: host_entry.telegram_topic_id,
                        owner_name: host_entry.owner_name.clone(),
                        owner_wa: host_entry.owner_wa.clone(),
                        owner_email: host_entry.owner_email.clone(),
                        is_user_routable: Self::matches_user_alert_filter(
                            &host_entry.user_alert_services,
                            &item.service_name,
                        ),
                    });
                }
            }
        }

        // Hitung ulang status agregat host berdasarkan service terburuk
        let worst_svc = host_entry
            .services
            .values()
            .map(|s| s.state)
            .max()
            .unwrap_or(0);
        let new_host_status = match worst_svc {
            2 => "CRIT",
            1 => "WARN",
            _ => "UP",
        }
        .to_string();

        if was_down && !host_entry.is_in_downtime {
            alerts.push(AlertEvent {
                host_id: host_entry.host_id,
                branch_code: host_entry.branch_code.clone(),
                hostname: host_entry.hostname.clone(),
                ip_address: host_entry.ip_address.clone(),
                service_name: "_HOST_".to_string(),
                state: 0,
                event_type: AlertEventType::Recovery,
                summary: format!(
                    "Host {} ({}) kembali ONLINE",
                    host_entry.hostname, host_entry.ip_address
                ),
                telegram_chat_id: host_entry.telegram_chat_id.clone(),
                telegram_topic_id: host_entry.telegram_topic_id,
                owner_name: None,
                owner_wa: None,
                owner_email: None,
                is_user_routable: false,
            });
        }

        host_entry.status = new_host_status.clone();
        Some((
            host_entry.host_id,
            new_host_status,
            immediate_writes,
            alerts,
        ))
    }
}
