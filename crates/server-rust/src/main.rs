mod api;
mod config;
mod state;
mod workers;

use chrono::Utc;
use config::ServerConfig;
use monitoring_proto::security::sha256_hex;
use sqlx::mysql::MySqlPoolOptions;
use state::{AppState, CachedHostState};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::sync::mpsc;
use tower_http::cors::CorsLayer;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,server_rust=debug".into()),
        )
        .json()
        .init();

    let cfg = ServerConfig::from_env()?;

    let pool = MySqlPoolOptions::new()
        .max_connections(cfg.db_max_connections)
        .connect(&cfg.database_url)
        .await?;

    // Pastikan cabang default tersedia saat bootstrap pertama kali
    seed_default_branch(&pool, &cfg).await;

    let (alert_tx, alert_rx) = mpsc::channel(4096);
    let app_state = AppState::new(cfg.clone(), pool, alert_tx);

    // Muat pengaturan runtime (SMTP/WA/Telegram/REST API) yang tersimpan di DB (jika ada)
    load_persisted_runtime_settings(&app_state).await;

    // Muat state host awal dari MariaDB ke dalam RAM DashMap
    warmup_dashmap_from_db(&app_state).await;

    // Jalankan 4 Background Worker Asinkron
    tokio::spawn(workers::batch_flusher_worker(Arc::clone(&app_state)));
    tokio::spawn(workers::stale_host_detector_worker(Arc::clone(&app_state)));
    tokio::spawn(workers::legacy_tcp_puller_worker(Arc::clone(&app_state)));
    tokio::spawn(workers::notification_worker(Arc::clone(&app_state), alert_rx));

    let app = api::build_router(Arc::clone(&app_state)).layer(CorsLayer::permissive());

    let listener = TcpListener::bind(&cfg.bind_addr).await?;
    tracing::info!(bind_addr = %cfg.bind_addr, "cmk-server listening and ready for 3,000+ devices");

    let state_for_shutdown = Arc::clone(&app_state);
    axum::serve(listener, app)
        .with_graceful_shutdown(async move {
            let _ = tokio::signal::ctrl_c().await;
            tracing::info!("SIGTERM/SIGINT received: flushing DashMap RAM state to MariaDB before exit...");
            workers::flush_dirty_cache_to_db(&state_for_shutdown).await;
        })
        .await?;

    Ok(())
}

async fn load_persisted_runtime_settings(state: &AppState) {
    let row: Option<(String,)> = sqlx::query_as(
        "SELECT setting_value FROM system_settings WHERE setting_key = 'runtime_settings'",
    )
    .fetch_optional(&state.db)
    .await
    .ok()
    .flatten();

    if let Some((raw,)) = row {
        if let Ok(parsed) = serde_json::from_str::<state::RuntimeSettings>(&raw) {
            let mut guard = state.runtime_settings.write().await;
            *guard = parsed;
        }
    }
}

async fn seed_default_branch(pool: &sqlx::MySqlPool, cfg: &ServerConfig) {
    if cfg.default_branch_code.trim().is_empty() || cfg.default_branch_name.trim().is_empty() {
        return;
    }
    let token_hash = sha256_hex(&cfg.default_enrollment_token);
    let _ = sqlx::query(
        r#"INSERT IGNORE INTO branches (branch_code, branch_name, enrollment_token_hash, is_active)
           VALUES (?, ?, ?, 1)"#,
    )
    .bind(&cfg.default_branch_code)
    .bind(&cfg.default_branch_name)
    .bind(&token_hash)
    .execute(pool)
    .await;
}

async fn warmup_dashmap_from_db(state: &AppState) {
    #[derive(sqlx::FromRow)]
    struct HostWarmRow {
        id: u64,
        host_uuid: String,
        branch_id: u32,
        branch_code: String,
        hostname: String,
        ip_address: String,
        mac_address: Option<String>,
        host_type: String,
        agent_secret_hmac: String,
        approval_status: String,
        status: String,
        is_in_downtime: bool,
        owner_name: Option<String>,
        owner_wa: Option<String>,
        owner_email: Option<String>,
        user_alert_services: String,
        telegram_chat_id: Option<String>,
        telegram_topic_id: Option<i32>,
    }

    let rows: Vec<HostWarmRow> = sqlx::query_as(
        r#"SELECT
             h.id, h.host_uuid, h.branch_id, b.branch_code, h.hostname, h.ip_address,
             h.mac_address, h.host_type, h.agent_secret_hmac, h.approval_status, h.status,
             h.is_in_downtime, h.owner_name, h.owner_wa, h.owner_email, h.user_alert_services,
             b.telegram_chat_id, b.telegram_topic_id
           FROM hosts h
           JOIN branches b ON b.id = h.branch_id"#,
    )
    .fetch_all(&state.db)
    .await
    .unwrap_or_default();

    let now_sec = Utc::now().timestamp();
    for r in rows {
        state.hosts_cache.insert(
            r.host_uuid.clone(),
            CachedHostState {
                host_id: r.id,
                host_uuid: r.host_uuid,
                branch_id: r.branch_id,
                branch_code: r.branch_code,
                hostname: r.hostname,
                ip_address: r.ip_address,
                mac_address: r.mac_address,
                host_type: r.host_type,
                agent_secret_hmac: r.agent_secret_hmac,
                approval_status: r.approval_status,
                status: r.status,
                is_in_downtime: r.is_in_downtime,
                owner_name: r.owner_name,
                owner_wa: r.owner_wa,
                owner_email: r.owner_email,
                user_alert_services: r.user_alert_services,
                telegram_chat_id: r.telegram_chat_id,
                telegram_topic_id: r.telegram_topic_id,
                last_seen_epoch: now_sec,
                dirty_heartbeat: false,
                services: HashMap::new(),
            },
        );
    }
    tracing::info!(
        loaded_hosts = state.hosts_cache.len(),
        "Warmed up In-Memory DashMap state cache from MariaDB"
    );
}
