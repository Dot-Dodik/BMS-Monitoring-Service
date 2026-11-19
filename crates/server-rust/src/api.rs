use crate::state::{AppState, CachedHostState, RuntimeSettings};
use crate::workers::{persist_immediate_service_transitions, send_smtp_email_with_settings};
use argon2::password_hash::SaltString;
use argon2::{Argon2, PasswordHasher};
use axum::body::Bytes;
use axum::extract::{Path, Query, State};
use axum::http::{header, HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, patch, post};
use axum::{Json, Router};
use chrono::Utc;
use monitoring_proto::model::{
    ApiEnvelope, EnrollRequest, EnrollResponse, IngestPayload, InventoryPayload,
};
use monitoring_proto::security::{derive_agent_secret, sha256_hex, verify_ingest_signature};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

pub fn build_router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/healthz", get(healthz_handler))
        .route("/api/v1/register", post(register_agent_handler))
        .route("/api/v1/ingest", post(ingest_metrics_handler))
        .route("/api/v1/branches/summary", get(branches_summary_handler))
        .route("/api/v1/branches", post(create_branch_handler))
        .route("/api/v1/branches/:id", patch(update_branch_telegram_handler))
        .route(
            "/api/v1/hosts",
            get(list_hosts_paginated_handler).post(create_host_manual_handler),
        )
        .route("/api/v1/hosts/:id/services", get(list_host_services_handler))
        .route(
            "/api/v1/hosts/:id/services.csv",
            get(export_host_services_csv_handler),
        )
        .route(
            "/api/v1/hosts/:id/services/:service_name/ack",
            post(acknowledge_service_handler),
        )
        .route("/api/v1/hosts/:id/downtime", post(toggle_host_downtime_handler))
        .route(
            "/api/v1/hosts/:id/metadata",
            patch(update_host_metadata_handler),
        )
        .route("/api/v1/inventory", get(list_inventory_handler))
        .route("/api/v1/inventory/export.csv", get(export_inventory_csv_handler))
        .route("/api/v1/incidents", get(list_incidents_handler))
        .route("/api/v1/outbox", get(list_outbox_handler))
        .route(
            "/api/v1/users",
            get(list_users_handler).post(create_user_handler),
        )
        .route("/api/v1/users/:id", patch(update_user_handler))
        .route("/api/v1/reports/hosts.csv", get(export_hosts_csv_handler))
        .route(
            "/api/v1/reports/incidents.csv",
            get(export_incidents_csv_handler),
        )
        .route(
            "/api/v1/device-types",
            get(list_device_types_handler).post(create_device_type_handler),
        )
        .route(
            "/api/v1/settings",
            get(get_settings_handler).put(update_settings_handler),
        )
        .route("/api/v1/settings/test-email", post(test_email_handler))
        .route("/api/v1/downloads", get(list_agent_downloads_handler))
        .route(
            "/api/v1/downloads/file/:filename",
            get(download_agent_file_handler),
        )
        .route(
            "/api/v1/downloads/install-linux.sh",
            get(download_linux_installer_handler),
        )
        .route(
            "/api/v1/downloads/install-windows.ps1",
            get(download_windows_installer_handler),
        )
        .with_state(state)
}

async fn healthz_handler(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let db_ok = sqlx::query("SELECT 1").execute(&state.db).await.is_ok();
    let cached_hosts = state.hosts_cache.len();
    let status = if db_ok {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };
    (
        status,
        Json(serde_json::json!({
            "status": if db_ok { "ok" } else { "degraded" },
            "database": db_ok,
            "cached_hosts_in_ram": cached_hosts,
            "timestamp": Utc::now().to_rfc3339()
        })),
    )
}

/// FASE 1 AUTH: Auto-Registration menggunakan `ENROLLMENT_TOKEN` cabang & deteksi konflik UUID
async fn register_agent_handler(
    State(state): State<Arc<AppState>>,
    Json(req): Json<EnrollRequest>,
) -> impl IntoResponse {
    let token_hash = sha256_hex(req.enrollment_token.trim());

    let branch_row: Option<(u32, String, Option<String>, Option<i32>)> = sqlx::query_as(
        "SELECT id, branch_code, telegram_chat_id, telegram_topic_id FROM branches WHERE branch_code = ? AND enrollment_token_hash = ? AND is_active = 1",
    )
    .bind(req.branch_code.trim())
    .bind(&token_hash)
    .fetch_optional(&state.db)
    .await
    .ok()
    .flatten();

    let Some((branch_id, branch_code, tg_chat, tg_topic)) = branch_row else {
        return (
            StatusCode::UNAUTHORIZED,
            Json(ApiEnvelope::<EnrollResponse>::err(
                "ENROLLMENT_TOKEN atau kode cabang tidak valid",
            )),
        );
    };

    let secret = derive_agent_secret(
        &state.cfg.hmac_master_pepper,
        req.host_uuid.trim(),
        &branch_code,
    );

    // Cek apakah host_uuid sudah terdaftar namun dari perangkat berbeda (deteksi disk cloning)
    let existing: Option<(u64, String, Option<String>, String)> = sqlx::query_as(
        "SELECT id, hostname, mac_address, approval_status FROM hosts WHERE host_uuid = ?",
    )
    .bind(req.host_uuid.trim())
    .fetch_optional(&state.db)
    .await
    .ok()
    .flatten();

    let mut approval_status = "APPROVED".to_string();
    let mut conflict_reason: Option<String> = None;

    if let Some((_, old_hostname, old_mac, cur_approval)) = &existing {
        let mac_changed = match (&old_mac, &req.mac_address) {
            (Some(a), Some(b)) if !a.is_empty() && !b.is_empty() => !a.eq_ignore_ascii_case(b),
            _ => false,
        };
        if !old_hostname.eq_ignore_ascii_case(req.hostname.trim()) && mac_changed {
            approval_status = "CONFLICT".to_string();
            conflict_reason = Some(format!(
                "Duplikat UUID terdeteksi: sebelumnya {} ({:?}), sekarang {} ({:?})",
                old_hostname, old_mac, req.hostname, req.mac_address
            ));
        } else {
            approval_status = cur_approval.clone();
        }
    }

    let res = sqlx::query(
        r#"INSERT INTO hosts
           (host_uuid, branch_id, hostname, ip_address, mac_address, host_type, agent_version, agent_secret_hmac, approval_status, conflict_reason, status, last_seen_at)
           VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, 'UP', UTC_TIMESTAMP())
           ON DUPLICATE KEY UPDATE
             hostname = VALUES(hostname),
             ip_address = VALUES(ip_address),
             mac_address = COALESCE(VALUES(mac_address), mac_address),
             agent_version = VALUES(agent_version),
             approval_status = VALUES(approval_status),
             conflict_reason = VALUES(conflict_reason),
             last_seen_at = UTC_TIMESTAMP()"#,
    )
    .bind(req.host_uuid.trim())
    .bind(branch_id)
    .bind(req.hostname.trim())
    .bind(req.ip_address.trim())
    .bind(&req.mac_address)
    .bind(req.host_type.as_db_str())
    .bind(req.agent_version.trim())
    .bind(&secret)
    .bind(&approval_status)
    .bind(&conflict_reason)
    .execute(&state.db)
    .await;

    let Ok(exec) = res else {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiEnvelope::err("Gagal menyimpan registrasi host ke database")),
        );
    };

    let host_id = existing
        .map(|e| e.0)
        .unwrap_or_else(|| exec.last_insert_id());

    state.hosts_cache.insert(
        req.host_uuid.trim().to_string(),
        CachedHostState {
            host_id,
            host_uuid: req.host_uuid.trim().to_string(),
            branch_id,
            branch_code,
            hostname: req.hostname.trim().to_string(),
            ip_address: req.ip_address.trim().to_string(),
            mac_address: req.mac_address.clone(),
            host_type: req.host_type.as_db_str().to_string(),
            agent_secret_hmac: secret.clone(),
            approval_status: approval_status.clone(),
            status: "UP".to_string(),
            is_in_downtime: false,
            owner_name: None,
            owner_wa: None,
            owner_email: None,
            user_alert_services: "Health_Battery,Storage_Usage,Printer_Paper".to_string(),
            telegram_chat_id: tg_chat,
            telegram_topic_id: tg_topic,
            last_seen_epoch: Utc::now().timestamp(),
            dirty_heartbeat: false,
            services: HashMap::new(),
        },
    );

    (
        StatusCode::OK,
        Json(ApiEnvelope::ok(EnrollResponse {
            host_uuid: req.host_uuid,
            agent_secret: secret,
            approval_status,
        })),
    )
}

/// FASE 2 INGEST: Verifikasi `HMAC-SHA256` + Anti-Replay ±5m + Diff-Only `DashMap` Update
async fn ingest_metrics_handler(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    body: Bytes,
) -> impl IntoResponse {
    let Some(host_uuid) = headers
        .get("X-Host-UUID")
        .and_then(|v| v.to_str().ok())
        .map(str::trim)
    else {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiEnvelope::<()>::err("Header X-Host-UUID wajib ada")),
        );
    };
    let Some(ts_sec) = headers
        .get("X-Timestamp")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.trim().parse::<i64>().ok())
    else {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiEnvelope::<()>::err("Header X-Timestamp tidak valid")),
        );
    };
    let Some(sig_hex) = headers
        .get("X-Signature")
        .and_then(|v| v.to_str().ok())
        .map(str::trim)
    else {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiEnvelope::<()>::err("Header X-Signature wajib ada")),
        );
    };

    let Some(host_meta) = state.hosts_cache.get(host_uuid).map(|h| {
        (
            h.host_id,
            h.agent_secret_hmac.clone(),
            h.approval_status.clone(),
        )
    }) else {
        return (
            StatusCode::UNAUTHORIZED,
            Json(ApiEnvelope::<()>::err(
                "HostUUID belum terdaftar di cache server (silakan enroll ulang)",
            )),
        );
    };

    let (host_id, secret, approval_status) = host_meta;
    if approval_status == "REVOKED" {
        return (
            StatusCode::FORBIDDEN,
            Json(ApiEnvelope::<()>::err("Akses agen ini telah dicabut (REVOKED)")),
        );
    }

    let now_sec = Utc::now().timestamp();
    if let Err(reason) =
        verify_ingest_signature(&secret, host_uuid, ts_sec, now_sec, &body, sig_hex)
    {
        return (
            StatusCode::UNAUTHORIZED,
            Json(ApiEnvelope::<()>::err(reason)),
        );
    }

    let Ok(payload) = serde_json::from_slice::<IngestPayload>(&body) else {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiEnvelope::<()>::err("Body JSON IngestPayload tidak valid")),
        );
    };

    if let Some((hid, host_status, immediate_writes, alerts)) = state.apply_ingest_in_memory(
        host_uuid,
        &payload.hostname,
        &payload.ip_address,
        now_sec,
        &payload.services,
    ) {
        if !immediate_writes.is_empty() {
            persist_immediate_service_transitions(
                &state,
                hid,
                &host_status,
                &payload.hostname,
                &payload.ip_address,
                &immediate_writes,
            )
            .await;
        }
        for alert in alerts {
            let _ = state.alert_tx.send(alert).await;
        }
    }

    // Jika paket membawa Slow Tier Inventory (tiap 6 jam), perbarui tabel host_inventory
    if let Some(inv) = payload.inventory {
        upsert_host_inventory(&state, host_id, &inv).await;
    }

    (StatusCode::OK, Json(ApiEnvelope::ok(())))
}

async fn upsert_host_inventory(state: &AppState, host_id: u64, inv: &InventoryPayload) {
    let _ = sqlx::query(
        r#"INSERT INTO host_inventory
           (host_id, os_name, os_version, os_license_status, os_license_key_partial,
            office_product, office_license_status, cpu_model, cpu_cores_threads,
            ram_total_gb, ram_slots_summary, disk_primary_model, disk_primary_type,
            disk_health_pct, disk_tbw_written_tb, disk_smart_status,
            battery_health_pct, battery_cycles, battery_design_mah, battery_full_mah,
            remote_anydesk_id, remote_rustdesk_id, updated_at)
           VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, UTC_TIMESTAMP())
           ON DUPLICATE KEY UPDATE
            os_name = COALESCE(VALUES(os_name), os_name),
            os_version = COALESCE(VALUES(os_version), os_version),
            os_license_status = COALESCE(VALUES(os_license_status), os_license_status),
            office_product = COALESCE(VALUES(office_product), office_product),
            cpu_model = COALESCE(VALUES(cpu_model), cpu_model),
            cpu_cores_threads = COALESCE(VALUES(cpu_cores_threads), cpu_cores_threads),
            ram_total_gb = COALESCE(VALUES(ram_total_gb), ram_total_gb),
            disk_primary_model = COALESCE(VALUES(disk_primary_model), disk_primary_model),
            disk_primary_type = COALESCE(VALUES(disk_primary_type), disk_primary_type),
            disk_health_pct = COALESCE(VALUES(disk_health_pct), disk_health_pct),
            disk_smart_status = COALESCE(VALUES(disk_smart_status), disk_smart_status),
            battery_health_pct = COALESCE(VALUES(battery_health_pct), battery_health_pct),
            battery_cycles = COALESCE(VALUES(battery_cycles), battery_cycles),
            battery_design_mah = COALESCE(VALUES(battery_design_mah), battery_design_mah),
            battery_full_mah = COALESCE(VALUES(battery_full_mah), battery_full_mah),
            remote_anydesk_id = COALESCE(VALUES(remote_anydesk_id), remote_anydesk_id),
            remote_rustdesk_id = COALESCE(VALUES(remote_rustdesk_id), remote_rustdesk_id),
            updated_at = UTC_TIMESTAMP()"#,
    )
    .bind(host_id)
    .bind(&inv.os_name)
    .bind(&inv.os_version)
    .bind(&inv.os_license_status)
    .bind(&inv.os_license_key_partial)
    .bind(&inv.office_product)
    .bind(&inv.office_license_status)
    .bind(&inv.cpu_model)
    .bind(&inv.cpu_cores_threads)
    .bind(inv.ram_total_gb)
    .bind(&inv.ram_slots_summary)
    .bind(&inv.disk_primary_model)
    .bind(&inv.disk_primary_type)
    .bind(inv.disk_health_pct)
    .bind(inv.disk_tbw_written_tb)
    .bind(&inv.disk_smart_status)
    .bind(inv.battery_health_pct)
    .bind(inv.battery_cycles)
    .bind(inv.battery_design_mah)
    .bind(inv.battery_full_mah)
    .bind(&inv.remote_anydesk_id)
    .bind(&inv.remote_rustdesk_id)
    .execute(&state.db)
    .await;
}

#[derive(Debug, Serialize)]
pub struct BranchSummaryCard {
    pub id: u32,
    pub branch_code: String,
    pub branch_name: String,
    pub telegram_chat_id: Option<String>,
    pub telegram_topic_id: Option<i32>,
    pub total_hosts: usize,
    pub up_count: usize,
    pub warn_count: usize,
    pub crit_count: usize,
    pub down_count: usize,
    pub pending_or_conflict: usize,
}

/// Endpoint Agregasi Kartu Kesehatan per Cabang (Dihitung dari `DashMap` RAM dalam < 1 ms)
async fn branches_summary_handler(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let rows: Vec<(u32, String, String, Option<String>, Option<i32>)> = sqlx::query_as(
        "SELECT id, branch_code, branch_name, telegram_chat_id, telegram_topic_id FROM branches WHERE is_active = 1 ORDER BY branch_code",
    )
    .fetch_all(&state.db)
    .await
    .unwrap_or_default();

    let mut cards: Vec<BranchSummaryCard> = rows
        .into_iter()
        .map(|(id, code, name, chat, topic)| BranchSummaryCard {
            id,
            branch_code: code,
            branch_name: name,
            telegram_chat_id: chat,
            telegram_topic_id: topic,
            total_hosts: 0,
            up_count: 0,
            warn_count: 0,
            crit_count: 0,
            down_count: 0,
            pending_or_conflict: 0,
        })
        .collect();

    for entry in state.hosts_cache.iter() {
        let h = entry.value();
        if let Some(card) = cards.iter_mut().find(|c| c.id == h.branch_id) {
            card.total_hosts += 1;
            if h.approval_status == "PENDING" || h.approval_status == "CONFLICT" {
                card.pending_or_conflict += 1;
            }
            match h.status.as_str() {
                "UP" => card.up_count += 1,
                "WARN" => card.warn_count += 1,
                "CRIT" => card.crit_count += 1,
                "DOWN" => card.down_count += 1,
                _ => {}
            }
        }
    }

    (StatusCode::OK, Json(ApiEnvelope::ok(cards)))
}

#[derive(Debug, Deserialize)]
pub struct UpdateBranchTelegramReq {
    pub telegram_chat_id: Option<String>,
    pub telegram_topic_id: Option<i32>,
}

async fn update_branch_telegram_handler(
    State(state): State<Arc<AppState>>,
    Path(branch_id): Path<u32>,
    Json(req): Json<UpdateBranchTelegramReq>,
) -> impl IntoResponse {
    let _ = sqlx::query(
        "UPDATE branches SET telegram_chat_id = ?, telegram_topic_id = ? WHERE id = ?",
    )
    .bind(&req.telegram_chat_id)
    .bind(req.telegram_topic_id)
    .bind(branch_id)
    .execute(&state.db)
    .await;

    for mut entry in state.hosts_cache.iter_mut() {
        if entry.value().branch_id == branch_id {
            entry.value_mut().telegram_chat_id = req.telegram_chat_id.clone();
            entry.value_mut().telegram_topic_id = req.telegram_topic_id;
        }
    }

    (StatusCode::OK, Json(ApiEnvelope::ok("Updated")))
}

#[derive(Debug, Deserialize)]
pub struct HostQueryParams {
    pub page: Option<usize>,
    pub per_page: Option<usize>,
    pub branch_code: Option<String>,
    pub host_type: Option<String>,
    pub search: Option<String>,
    pub action_needed: Option<bool>,
    pub approval_filter: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct HostRowDto {
    pub host_id: u64,
    pub host_uuid: String,
    pub branch_code: String,
    pub hostname: String,
    pub ip_address: String,
    pub host_type: String,
    pub status: String,
    pub approval_status: String,
    pub is_in_downtime: bool,
    pub ok_services: usize,
    pub warn_services: usize,
    pub crit_services: usize,
    pub total_services: usize,
    pub owner_name: Option<String>,
    pub owner_wa: Option<String>,
    pub owner_email: Option<String>,
    pub user_alert_services: String,
    pub last_seen_epoch: i64,
    pub problem_services: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct PaginatedHostsResponse {
    pub page: usize,
    pub per_page: usize,
    pub total_items: usize,
    pub total_pages: usize,
    pub items: Vec<HostRowDto>,
}

/// Server-Side Pagination (50 baris/halaman) + Filter Instan "Action Needed" dari RAM `DashMap`
async fn list_hosts_paginated_handler(
    State(state): State<Arc<AppState>>,
    Query(q): Query<HostQueryParams>,
) -> impl IntoResponse {
    let page = q.page.unwrap_or(1).max(1);
    let per_page = q.per_page.unwrap_or(50).clamp(10, 100);
    let action_only = q.action_needed.unwrap_or(false);
    let search_q = q.search.unwrap_or_default().trim().to_ascii_lowercase();

    let mut filtered: Vec<HostRowDto> = state
        .hosts_cache
        .iter()
        .filter_map(|entry| {
            let h = entry.value();
            if let Some(bc) = &q.branch_code {
                if !bc.is_empty() && !h.branch_code.eq_ignore_ascii_case(bc) {
                    return None;
                }
            }
            if let Some(ht) = &q.host_type {
                if !ht.is_empty() && !h.host_type.eq_ignore_ascii_case(ht) {
                    return None;
                }
            }
            if let Some(af) = &q.approval_filter {
                if !af.is_empty() && !h.approval_status.eq_ignore_ascii_case(af) {
                    return None;
                }
            }
            if action_only
                && h.status == "UP"
                && h.approval_status == "APPROVED"
            {
                return None;
            }
            if !search_q.is_empty()
                && !h.hostname.to_ascii_lowercase().contains(&search_q)
                && !h.ip_address.to_ascii_lowercase().contains(&search_q)
                && !h.branch_code.to_ascii_lowercase().contains(&search_q)
                && !h.host_uuid.to_ascii_lowercase().contains(&search_q)
            {
                return None;
            }

            let mut ok_services = 0usize;
            let mut warn_services = 0usize;
            let mut crit_services = 0usize;
            for s in h.services.values() {
                match s.state {
                    0 => ok_services += 1,
                    1 => warn_services += 1,
                    _ => crit_services += 1,
                }
            }
            let total_services = h.services.len();

            let problem_services: Vec<String> = h
                .services
                .values()
                .filter(|s| s.state > 0)
                .map(|s| format!("{}: {}", s.service_name, s.summary))
                .collect();

            Some(HostRowDto {
                host_id: h.host_id,
                host_uuid: h.host_uuid.clone(),
                branch_code: h.branch_code.clone(),
                hostname: h.hostname.clone(),
                ip_address: h.ip_address.clone(),
                host_type: h.host_type.clone(),
                status: h.status.clone(),
                approval_status: h.approval_status.clone(),
                is_in_downtime: h.is_in_downtime,
                ok_services,
                warn_services,
                crit_services,
                total_services,
                owner_name: h.owner_name.clone(),
                owner_wa: h.owner_wa.clone(),
                owner_email: h.owner_email.clone(),
                user_alert_services: h.user_alert_services.clone(),
                last_seen_epoch: h.last_seen_epoch,
                problem_services,
            })
        })
        .collect();

    // Urutkan prioritas: DOWN -> CRIT -> WARN -> UP, lalu berdasarkan hostname
    filtered.sort_by(|a, b| {
        let rank = |st: &str| match st {
            "DOWN" => 0,
            "CRIT" => 1,
            "WARN" => 2,
            _ => 3,
        };
        rank(&a.status)
            .cmp(&rank(&b.status))
            .then_with(|| a.hostname.cmp(&b.hostname))
    });

    let total_items = filtered.len();
    let total_pages = (total_items + per_page - 1).max(1) / per_page;
    let start = (page - 1) * per_page;
    let items = filtered
        .into_iter()
        .skip(start)
        .take(per_page)
        .collect();

    (
        StatusCode::OK,
        Json(ApiEnvelope::ok(PaginatedHostsResponse {
            page,
            per_page,
            total_items,
            total_pages,
            items,
        })),
    )
}

#[derive(Debug, Serialize)]
pub struct ServiceRowDto {
    pub service_name: String,
    pub tier: String,
    pub state: u8,
    pub is_acknowledged: bool,
    pub summary: String,
    pub metrics: String,
    pub last_check_epoch: i64,
    pub last_state_change_epoch: i64,
}

async fn list_host_services_handler(
    State(state): State<Arc<AppState>>,
    Path(host_id): Path<u64>,
) -> impl IntoResponse {
    for entry in state.hosts_cache.iter() {
        if entry.value().host_id == host_id {
            let mut list: Vec<ServiceRowDto> = entry
                .value()
                .services
                .values()
                .map(|s| ServiceRowDto {
                    service_name: s.service_name.clone(),
                    tier: s.tier.as_db_str().to_string(),
                    state: s.state,
                    is_acknowledged: s.is_acknowledged,
                    summary: s.summary.clone(),
                    metrics: s.metrics.clone(),
                    last_check_epoch: s.last_check_epoch,
                    last_state_change_epoch: s.last_state_change_epoch,
                })
                .collect();
            list.sort_by(|a, b| b.state.cmp(&a.state).then_with(|| a.service_name.cmp(&b.service_name)));
            return (StatusCode::OK, Json(ApiEnvelope::ok(list)));
        }
    }
    (StatusCode::OK, Json(ApiEnvelope::ok(Vec::<ServiceRowDto>::new())))
}

#[derive(Debug, Deserialize)]
pub struct UpdateHostMetadataReq {
    pub owner_name: Option<String>,
    pub owner_wa: Option<String>,
    pub owner_email: Option<String>,
    pub user_alert_services: Option<String>,
    pub approval_status: Option<String>,
}

async fn update_host_metadata_handler(
    State(state): State<Arc<AppState>>,
    Path(host_id): Path<u64>,
    Json(req): Json<UpdateHostMetadataReq>,
) -> impl IntoResponse {
    let services_filter = req
        .user_alert_services
        .clone()
        .unwrap_or_else(|| "Health_Battery,Storage_Usage,Printer_Paper".to_string());
    let approval = req
        .approval_status
        .clone()
        .unwrap_or_else(|| "APPROVED".to_string());

    let _ = sqlx::query(
        r#"UPDATE hosts
           SET owner_name = ?, owner_wa = ?, owner_email = ?, user_alert_services = ?, approval_status = ?
           WHERE id = ?"#,
    )
    .bind(&req.owner_name)
    .bind(&req.owner_wa)
    .bind(&req.owner_email)
    .bind(&services_filter)
    .bind(&approval)
    .bind(host_id)
    .execute(&state.db)
    .await;

    for mut entry in state.hosts_cache.iter_mut() {
        if entry.value().host_id == host_id {
            let h = entry.value_mut();
            h.owner_name = req.owner_name.clone();
            h.owner_wa = req.owner_wa.clone();
            h.owner_email = req.owner_email.clone();
            h.user_alert_services = services_filter;
            h.approval_status = approval;
            break;
        }
    }

    (StatusCode::OK, Json(ApiEnvelope::ok("Metadata host diperbarui")))
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct InventoryItemDto {
    pub host_id: u64,
    pub branch_code: String,
    pub hostname: String,
    pub ip_address: String,
    pub host_type: String,
    pub os_name: Option<String>,
    pub os_license_status: Option<String>,
    pub office_product: Option<String>,
    pub cpu_model: Option<String>,
    pub ram_total_gb: Option<f64>,
    pub disk_primary_model: Option<String>,
    pub disk_primary_type: Option<String>,
    pub disk_health_pct: Option<u8>,
    pub battery_health_pct: Option<f64>,
    pub battery_cycles: Option<u32>,
    pub remote_anydesk_id: Option<String>,
    pub remote_rustdesk_id: Option<String>,
}

async fn query_inventory_rows(state: &AppState) -> Vec<InventoryItemDto> {
    sqlx::query_as::<_, InventoryItemDto>(
        r#"SELECT
             h.id AS host_id,
             b.branch_code,
             h.hostname,
             h.ip_address,
             h.host_type,
             i.os_name,
             i.os_license_status,
             i.office_product,
             i.cpu_model,
             CAST(i.ram_total_gb AS DOUBLE) AS ram_total_gb,
             i.disk_primary_model,
             i.disk_primary_type,
             i.disk_health_pct,
             CAST(i.battery_health_pct AS DOUBLE) AS battery_health_pct,
             i.battery_cycles,
             i.remote_anydesk_id,
             i.remote_rustdesk_id
           FROM hosts h
           JOIN branches b ON b.id = h.branch_id
           LEFT JOIN host_inventory i ON i.host_id = h.id
           ORDER BY b.branch_code, h.hostname
           LIMIT 3000"#,
    )
    .fetch_all(&state.db)
    .await
    .unwrap_or_default()
}

async fn list_inventory_handler(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let rows = query_inventory_rows(&state).await;
    (StatusCode::OK, Json(ApiEnvelope::ok(rows)))
}

/// Endpoint Export CSV Sekali Klik untuk Laporan Audit Aset Lintas Cabang
async fn export_inventory_csv_handler(State(state): State<Arc<AppState>>) -> Response {
    let rows = query_inventory_rows(&state).await;
    let mut csv = String::with_capacity(4096 + rows.len() * 160);
    csv.push_str("Branch,Hostname,IP_Address,Host_Type,OS_Name,OS_License,Office_Product,CPU_Model,RAM_GB,Disk_Model,Disk_Type,Disk_Health_Pct,Battery_Health_Pct,Battery_Cycles,AnyDesk_ID,RustDesk_ID\n");

    let esc = |opt: &Option<String>| -> String {
        let s = opt.as_deref().unwrap_or("-");
        format!("\"{}\"", s.replace('"', "\"\""))
    };

    for r in rows {
        csv.push_str(&format!(
            "{},{},{},{},{},{},{},{},{:.1},{},{},{},{},{},{},{}\n",
            r.branch_code,
            r.hostname,
            r.ip_address,
            r.host_type,
            esc(&r.os_name),
            esc(&r.os_license_status),
            esc(&r.office_product),
            esc(&r.cpu_model),
            r.ram_total_gb.unwrap_or(0.0),
            esc(&r.disk_primary_model),
            esc(&r.disk_primary_type),
            r.disk_health_pct.map(|v| v.to_string()).unwrap_or_else(|| "-".to_string()),
            r.battery_health_pct
                .map(|v| format!("{:.1}", v))
                .unwrap_or_else(|| "-".to_string()),
            r.battery_cycles
                .map(|v| v.to_string())
                .unwrap_or_else(|| "-".to_string()),
            esc(&r.remote_anydesk_id),
            esc(&r.remote_rustdesk_id),
        ));
    }

    (
        StatusCode::OK,
        [
            (header::CONTENT_TYPE, "text/csv; charset=utf-8"),
            (
                header::CONTENT_DISPOSITION,
                "attachment; filename=\"cmk_asset_inventory_export.csv\"",
            ),
        ],
        csv,
    )
        .into_response()
}

#[derive(Debug, Deserialize)]
pub struct CreateBranchReq {
    pub branch_code: String,
    pub branch_name: String,
    pub enrollment_token: String,
    pub subnet_cidr: Option<String>,
    pub telegram_chat_id: Option<String>,
    pub telegram_topic_id: Option<i32>,
}

async fn create_branch_handler(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreateBranchReq>,
) -> impl IntoResponse {
    let code = req.branch_code.trim().to_uppercase();
    if code.is_empty() || req.branch_name.trim().is_empty() || req.enrollment_token.trim().is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiEnvelope::<()>::err("Kode cabang, nama cabang, dan enrollment token wajib diisi")),
        );
    }
    let token_hash = sha256_hex(req.enrollment_token.trim());
    let res = sqlx::query(
        r#"INSERT INTO branches (branch_code, branch_name, enrollment_token_hash, subnet_cidr, telegram_chat_id, telegram_topic_id, is_active)
           VALUES (?, ?, ?, ?, ?, ?, 1)"#,
    )
    .bind(&code)
    .bind(req.branch_name.trim())
    .bind(&token_hash)
    .bind(&req.subnet_cidr)
    .bind(&req.telegram_chat_id)
    .bind(req.telegram_topic_id)
    .execute(&state.db)
    .await;

    match res {
        Ok(_) => (StatusCode::CREATED, Json(ApiEnvelope::ok(()))),
        Err(e) => (
            StatusCode::CONFLICT,
            Json(ApiEnvelope::err(format!("Gagal menambah cabang: {}", e))),
        ),
    }
}

#[derive(Debug, Deserialize)]
pub struct CreateHostManualReq {
    pub branch_code: String,
    pub hostname: String,
    pub ip_address: String,
    pub host_type: String,
    pub pull_legacy_enabled: Option<bool>,
    pub pull_legacy_port: Option<u16>,
    pub owner_name: Option<String>,
    pub owner_wa: Option<String>,
    pub owner_email: Option<String>,
}

async fn create_host_manual_handler(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreateHostManualReq>,
) -> impl IntoResponse {
    let branch: Option<(u32, String, Option<String>, Option<i32>)> = sqlx::query_as(
        "SELECT id, branch_code, telegram_chat_id, telegram_topic_id FROM branches WHERE branch_code = ? AND is_active = 1",
    )
    .bind(req.branch_code.trim())
    .fetch_optional(&state.db)
    .await
    .ok()
    .flatten();

    let Some((branch_id, branch_code, tg_chat, tg_topic)) = branch else {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiEnvelope::<u64>::err("Cabang tidak ditemukan")),
        );
    };

    let host_uuid = format!(
        "manual-{}-{}",
        req.hostname.trim().to_ascii_lowercase(),
        Utc::now().timestamp()
    );
    let secret = derive_agent_secret(&state.cfg.hmac_master_pepper, &host_uuid, &branch_code);
    let pull_enabled = req.pull_legacy_enabled.unwrap_or(true);
    let pull_port = req.pull_legacy_port.unwrap_or(6556);

    let res = sqlx::query(
        r#"INSERT INTO hosts
           (host_uuid, branch_id, hostname, ip_address, host_type, agent_version, agent_secret_hmac,
            pull_legacy_enabled, pull_legacy_port, approval_status, status, owner_name, owner_wa, owner_email, last_seen_at)
           VALUES (?, ?, ?, ?, ?, 'manual', ?, ?, ?, 'APPROVED', 'PENDING', ?, ?, ?, UTC_TIMESTAMP())"#,
    )
    .bind(&host_uuid)
    .bind(branch_id)
    .bind(req.hostname.trim())
    .bind(req.ip_address.trim())
    .bind(req.host_type.trim())
    .bind(&secret)
    .bind(pull_enabled)
    .bind(pull_port)
    .bind(&req.owner_name)
    .bind(&req.owner_wa)
    .bind(&req.owner_email)
    .execute(&state.db)
    .await;

    let Ok(exec) = res else {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiEnvelope::err("Gagal menambahkan host manual")),
        );
    };
    let host_id = exec.last_insert_id();

    state.hosts_cache.insert(
        host_uuid.clone(),
        CachedHostState {
            host_id,
            host_uuid,
            branch_id,
            branch_code,
            hostname: req.hostname.trim().to_string(),
            ip_address: req.ip_address.trim().to_string(),
            mac_address: None,
            host_type: req.host_type.trim().to_string(),
            agent_secret_hmac: secret,
            approval_status: "APPROVED".to_string(),
            status: "PENDING".to_string(),
            is_in_downtime: false,
            owner_name: req.owner_name,
            owner_wa: req.owner_wa,
            owner_email: req.owner_email,
            user_alert_services: "Health_Battery,Storage_Usage,Printer_Paper".to_string(),
            telegram_chat_id: tg_chat,
            telegram_topic_id: tg_topic,
            last_seen_epoch: Utc::now().timestamp(),
            dirty_heartbeat: false,
            services: HashMap::new(),
        },
    );

    (StatusCode::CREATED, Json(ApiEnvelope::ok(host_id)))
}

#[derive(Debug, Deserialize)]
pub struct AckReq {
    pub acknowledged: bool,
}

async fn acknowledge_service_handler(
    State(state): State<Arc<AppState>>,
    Path((host_id, service_name)): Path<(u64, String)>,
    Json(req): Json<AckReq>,
) -> impl IntoResponse {
    let _ = sqlx::query("UPDATE services SET is_acknowledged = ? WHERE host_id = ? AND service_name = ?")
        .bind(req.acknowledged)
        .bind(host_id)
        .bind(&service_name)
        .execute(&state.db)
        .await;

    for mut entry in state.hosts_cache.iter_mut() {
        if entry.value().host_id == host_id {
            if let Some(svc) = entry.value_mut().services.get_mut(&service_name) {
                svc.is_acknowledged = req.acknowledged;
            }
            break;
        }
    }

    (StatusCode::OK, Json(ApiEnvelope::ok("ACK diperbarui")))
}

#[derive(Debug, Deserialize)]
pub struct DowntimeReq {
    pub in_downtime: bool,
}

async fn toggle_host_downtime_handler(
    State(state): State<Arc<AppState>>,
    Path(host_id): Path<u64>,
    Json(req): Json<DowntimeReq>,
) -> impl IntoResponse {
    let _ = sqlx::query("UPDATE hosts SET is_in_downtime = ? WHERE id = ?")
        .bind(req.in_downtime)
        .bind(host_id)
        .execute(&state.db)
        .await;

    for mut entry in state.hosts_cache.iter_mut() {
        if entry.value().host_id == host_id {
            entry.value_mut().is_in_downtime = req.in_downtime;
            break;
        }
    }

    (StatusCode::OK, Json(ApiEnvelope::ok("Downtime diperbarui")))
}

async fn export_host_services_csv_handler(
    State(state): State<Arc<AppState>>,
    Path(host_id): Path<u64>,
) -> Response {
    let mut csv = String::from("Hostname,Service,Tier,State,Acknowledged,Metrics,Summary\n");
    for entry in state.hosts_cache.iter() {
        let h = entry.value();
        if h.host_id == host_id {
            for s in h.services.values() {
                csv.push_str(&format!(
                    "{},{},{},{},{},\"{}\",\"{}\"\n",
                    h.hostname,
                    s.service_name,
                    s.tier.as_db_str(),
                    s.state,
                    s.is_acknowledged,
                    s.metrics.replace('"', "\"\""),
                    s.summary.replace('"', "\"\""),
                ));
            }
            break;
        }
    }
    (
        StatusCode::OK,
        [
            (header::CONTENT_TYPE, "text/csv; charset=utf-8"),
            (
                header::CONTENT_DISPOSITION,
                "attachment; filename=\"host_services_export.csv\"",
            ),
        ],
        csv,
    )
        .into_response()
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct UserRowDto {
    pub id: u32,
    pub username: String,
    pub full_name: String,
    pub role: String,
    pub assigned_branch_id: Option<u32>,
    pub branch_code: Option<String>,
    pub is_active: bool,
}

async fn list_users_handler(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let rows: Vec<UserRowDto> = sqlx::query_as(
        r#"SELECT u.id, u.username, u.full_name, u.role, u.assigned_branch_id, b.branch_code, u.is_active
           FROM users u
           LEFT JOIN branches b ON b.id = u.assigned_branch_id
           ORDER BY u.id DESC"#,
    )
    .fetch_all(&state.db)
    .await
    .unwrap_or_default();
    (StatusCode::OK, Json(ApiEnvelope::ok(rows)))
}

#[derive(Debug, Deserialize)]
pub struct CreateUserReq {
    pub username: String,
    pub full_name: String,
    pub password: String,
    pub role: String,
    pub assigned_branch_id: Option<u32>,
}

async fn create_user_handler(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreateUserReq>,
) -> impl IntoResponse {
    if req.username.trim().is_empty() || req.password.len() < 6 {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiEnvelope::<()>::err("Username wajib diisi dan password minimal 6 karakter")),
        );
    }

    let salt = SaltString::generate(&mut argon2::password_hash::rand_core::OsRng);
    let Ok(hash) = Argon2::default().hash_password(req.password.as_bytes(), &salt) else {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiEnvelope::err("Gagal mengenkripsi password")),
        );
    };

    let res = sqlx::query(
        r#"INSERT INTO users (username, password_argon2, full_name, role, assigned_branch_id, is_active)
           VALUES (?, ?, ?, ?, ?, 1)"#,
    )
    .bind(req.username.trim())
    .bind(hash.to_string())
    .bind(req.full_name.trim())
    .bind(req.role.trim())
    .bind(req.assigned_branch_id)
    .execute(&state.db)
    .await;

    match res {
        Ok(_) => (StatusCode::CREATED, Json(ApiEnvelope::ok(()))),
        Err(e) => (
            StatusCode::CONFLICT,
            Json(ApiEnvelope::err(format!("Gagal membuat user: {}", e))),
        ),
    }
}

#[derive(Debug, Deserialize)]
pub struct UpdateUserReq {
    pub is_active: Option<bool>,
    pub role: Option<String>,
}

async fn update_user_handler(
    State(state): State<Arc<AppState>>,
    Path(user_id): Path<u32>,
    Json(req): Json<UpdateUserReq>,
) -> impl IntoResponse {
    if let Some(active) = req.is_active {
        let _ = sqlx::query("UPDATE users SET is_active = ? WHERE id = ?")
            .bind(active)
            .bind(user_id)
            .execute(&state.db)
            .await;
    }
    if let Some(role) = req.role {
        let _ = sqlx::query("UPDATE users SET role = ? WHERE id = ?")
            .bind(role)
            .bind(user_id)
            .execute(&state.db)
            .await;
    }
    (StatusCode::OK, Json(ApiEnvelope::ok("User diperbarui")))
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct IncidentDto {
    pub id: u64,
    pub hostname: String,
    pub branch_code: String,
    pub service_name: String,
    pub from_state: u8,
    pub to_state: u8,
    pub summary: String,
    pub created_at: String,
}

async fn list_incidents_handler(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let rows: Vec<IncidentDto> = sqlx::query_as(
        r#"SELECT l.id, h.hostname, b.branch_code, l.service_name, l.from_state, l.to_state, l.summary,
                  DATE_FORMAT(l.created_at, '%Y-%m-%d %H:%i:%s') AS created_at
           FROM incident_logs l
           JOIN hosts h ON h.id = l.host_id
           JOIN branches b ON b.id = h.branch_id
           ORDER BY l.id DESC
           LIMIT 200"#,
    )
    .fetch_all(&state.db)
    .await
    .unwrap_or_default();
    (StatusCode::OK, Json(ApiEnvelope::ok(rows)))
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct OutboxDto {
    pub id: u64,
    pub hostname: String,
    pub service_name: String,
    pub target_role: String,
    pub destination: String,
    pub event_type: String,
    pub delivery_status: String,
    pub last_error: Option<String>,
    pub created_at: String,
}

async fn list_outbox_handler(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let rows: Vec<OutboxDto> = sqlx::query_as(
        r#"SELECT o.id, h.hostname, o.service_name, o.target_role, o.destination, o.event_type,
                  o.delivery_status, o.last_error, DATE_FORMAT(o.created_at, '%Y-%m-%d %H:%i:%s') AS created_at
           FROM notification_outbox o
           JOIN hosts h ON h.id = o.host_id
           ORDER BY o.id DESC
           LIMIT 200"#,
    )
    .fetch_all(&state.db)
    .await
    .unwrap_or_default();
    (StatusCode::OK, Json(ApiEnvelope::ok(rows)))
}

async fn export_hosts_csv_handler(State(state): State<Arc<AppState>>) -> Response {
    let mut csv = String::from("Branch,Hostname,IP_Address,Host_Type,Status,Approval,OK_Services,WARN_Services,CRIT_Services,Owner_Name,Owner_WA,Owner_Email\n");
    for entry in state.hosts_cache.iter() {
        let h = entry.value();
        let ok_c = h.services.values().filter(|s| s.state == 0).count();
        let warn_c = h.services.values().filter(|s| s.state == 1).count();
        let crit_c = h.services.values().filter(|s| s.state >= 2).count();
        csv.push_str(&format!(
            "{},{},{},{},{},{},{},{},{},\"{}\",\"{}\",\"{}\"\n",
            h.branch_code,
            h.hostname,
            h.ip_address,
            h.host_type,
            h.status,
            h.approval_status,
            ok_c,
            warn_c,
            crit_c,
            h.owner_name.as_deref().unwrap_or("-").replace('"', "\"\""),
            h.owner_wa.as_deref().unwrap_or("-").replace('"', "\"\""),
            h.owner_email.as_deref().unwrap_or("-").replace('"', "\"\""),
        ));
    }
    (
        StatusCode::OK,
        [
            (header::CONTENT_TYPE, "text/csv; charset=utf-8"),
            (
                header::CONTENT_DISPOSITION,
                "attachment; filename=\"cmk_hosts_status_report.csv\"",
            ),
        ],
        csv,
    )
        .into_response()
}

async fn export_incidents_csv_handler(State(state): State<Arc<AppState>>) -> Response {
    let rows: Vec<IncidentDto> = sqlx::query_as(
        r#"SELECT l.id, h.hostname, b.branch_code, l.service_name, l.from_state, l.to_state, l.summary,
                  DATE_FORMAT(l.created_at, '%Y-%m-%d %H:%i:%s') AS created_at
           FROM incident_logs l
           JOIN hosts h ON h.id = l.host_id
           JOIN branches b ON b.id = h.branch_id
           ORDER BY l.id DESC
           LIMIT 5000"#,
    )
    .fetch_all(&state.db)
    .await
    .unwrap_or_default();

    let mut csv = String::from("Timestamp,Branch,Hostname,Service,From_State,To_State,Summary\n");
    for r in rows {
        csv.push_str(&format!(
            "{},{},{},{},{},{},\"{}\"\n",
            r.created_at,
            r.branch_code,
            r.hostname,
            r.service_name,
            r.from_state,
            r.to_state,
            r.summary.replace('"', "\"\""),
        ));
    }
    (
        StatusCode::OK,
        [
            (header::CONTENT_TYPE, "text/csv; charset=utf-8"),
            (
                header::CONTENT_DISPOSITION,
                "attachment; filename=\"cmk_incidents_sla_report.csv\"",
            ),
        ],
        csv,
    )
        .into_response()
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct DeviceTypeDto {
    pub code: String,
    pub label: String,
    pub is_builtin: bool,
}

async fn list_device_types_handler(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let rows: Vec<DeviceTypeDto> = sqlx::query_as(
        "SELECT code, label, is_builtin FROM device_types ORDER BY is_builtin DESC, code ASC",
    )
    .fetch_all(&state.db)
    .await
    .unwrap_or_default();
    (StatusCode::OK, Json(ApiEnvelope::ok(rows)))
}

#[derive(Debug, Deserialize)]
pub struct CreateDeviceTypeReq {
    pub code: String,
    pub label: String,
}

async fn create_device_type_handler(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreateDeviceTypeReq>,
) -> impl IntoResponse {
    let code: String = req
        .code
        .trim()
        .to_ascii_uppercase()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .collect();
    let label = req.label.trim();
    if code.is_empty() || label.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiEnvelope::<DeviceTypeDto>::err("Kode dan label jenis perangkat wajib diisi")),
        );
    }

    let res = sqlx::query(
        "INSERT INTO device_types (code, label, is_builtin) VALUES (?, ?, 0) ON DUPLICATE KEY UPDATE label = VALUES(label)",
    )
    .bind(&code)
    .bind(label)
    .execute(&state.db)
    .await;

    match res {
        Ok(_) => (
            StatusCode::CREATED,
            Json(ApiEnvelope::ok(DeviceTypeDto {
                code,
                label: label.to_string(),
                is_builtin: false,
            })),
        ),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiEnvelope::err(format!("Gagal menambah jenis device: {}", e))),
        ),
    }
}

#[derive(Debug, Serialize)]
pub struct SettingsViewDto {
    pub rest_api_public_url: String,
    pub default_enrollment_token: String,
    pub batch_flush_interval_sec: u64,
    pub host_stale_threshold_sec: i64,
    pub flapping_threshold_cycles: u8,
    pub telegram_default_chat_id: String,
    pub has_telegram_bot_token: bool,
    pub wa_webhook_url: String,
    pub has_wa_webhook_token: bool,
    pub smtp_host: String,
    pub smtp_port: u16,
    pub smtp_username: String,
    pub smtp_from: String,
    pub smtp_use_tls: bool,
    pub has_smtp_password: bool,
}

async fn get_settings_handler(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let rt = state.runtime_settings.read().await;
    (
        StatusCode::OK,
        Json(ApiEnvelope::ok(SettingsViewDto {
            rest_api_public_url: rt.rest_api_public_url.clone(),
            default_enrollment_token: rt.default_enrollment_token.clone(),
            batch_flush_interval_sec: rt.batch_flush_interval_sec,
            host_stale_threshold_sec: rt.host_stale_threshold_sec,
            flapping_threshold_cycles: rt.flapping_threshold_cycles,
            telegram_default_chat_id: rt.telegram_default_chat_id.clone(),
            has_telegram_bot_token: !rt.telegram_bot_token.trim().is_empty(),
            wa_webhook_url: rt.wa_webhook_url.clone(),
            has_wa_webhook_token: !rt.wa_webhook_token.trim().is_empty(),
            smtp_host: rt.smtp_host.clone(),
            smtp_port: rt.smtp_port,
            smtp_username: rt.smtp_username.clone(),
            smtp_from: rt.smtp_from.clone(),
            smtp_use_tls: rt.smtp_use_tls,
            has_smtp_password: !rt.smtp_password.trim().is_empty(),
        })),
    )
}

#[derive(Debug, Deserialize)]
pub struct UpdateSettingsReq {
    pub rest_api_public_url: String,
    pub default_enrollment_token: String,
    pub batch_flush_interval_sec: u64,
    pub host_stale_threshold_sec: i64,
    pub flapping_threshold_cycles: u8,
    pub telegram_bot_token: Option<String>,
    pub telegram_default_chat_id: String,
    pub wa_webhook_url: String,
    pub wa_webhook_token: Option<String>,
    pub smtp_host: String,
    pub smtp_port: u16,
    pub smtp_username: String,
    pub smtp_password: Option<String>,
    pub smtp_from: String,
    pub smtp_use_tls: bool,
}

async fn update_settings_handler(
    State(state): State<Arc<AppState>>,
    Json(req): Json<UpdateSettingsReq>,
) -> impl IntoResponse {
    let updated: RuntimeSettings = {
        let mut rt = state.runtime_settings.write().await;
        rt.rest_api_public_url = req.rest_api_public_url.trim().to_string();
        rt.default_enrollment_token = req.default_enrollment_token.trim().to_string();
        rt.batch_flush_interval_sec = req.batch_flush_interval_sec.max(30);
        rt.host_stale_threshold_sec = req.host_stale_threshold_sec.max(60);
        rt.flapping_threshold_cycles = req.flapping_threshold_cycles.max(1);
        rt.telegram_default_chat_id = req.telegram_default_chat_id.trim().to_string();
        rt.wa_webhook_url = req.wa_webhook_url.trim().to_string();
        rt.smtp_host = req.smtp_host.trim().to_string();
        rt.smtp_port = req.smtp_port;
        rt.smtp_username = req.smtp_username.trim().to_string();
        rt.smtp_from = req.smtp_from.trim().to_string();
        rt.smtp_use_tls = req.smtp_use_tls;

        if let Some(tok) = req.telegram_bot_token.as_deref().filter(|s| !s.trim().is_empty()) {
            rt.telegram_bot_token = tok.trim().to_string();
        }
        if let Some(wtok) = req.wa_webhook_token.as_deref().filter(|s| !s.trim().is_empty()) {
            rt.wa_webhook_token = wtok.trim().to_string();
        }
        if let Some(pw) = req.smtp_password.as_deref().filter(|s| !s.is_empty()) {
            rt.smtp_password = pw.to_string();
        }
        rt.clone()
    };

    if let Ok(json_val) = serde_json::to_string(&updated) {
        let _ = sqlx::query(
            "INSERT INTO system_settings (setting_key, setting_value, is_secret) VALUES ('runtime_settings', ?, 1) ON DUPLICATE KEY UPDATE setting_value = VALUES(setting_value)",
        )
        .bind(json_val)
        .execute(&state.db)
        .await;
    }

    (StatusCode::OK, Json(ApiEnvelope::ok("Pengaturan sistem & SMTP tersimpan")))
}

#[derive(Debug, Deserialize)]
pub struct TestEmailReq {
    pub to_email: String,
}

async fn test_email_handler(
    State(state): State<Arc<AppState>>,
    Json(req): Json<TestEmailReq>,
) -> impl IntoResponse {
    if req.to_email.trim().is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiEnvelope::<String>::err("Alamat email tujuan tes wajib diisi")),
        );
    }
    match send_smtp_email_with_settings(
        &state,
        req.to_email.trim(),
        "[BMS] Tes Konfigurasi Email Utama SMTP",
        "Halo,\n\nEmail ini menandakan konfigurasi SMTP Email Utama pada BMS (Borwita Monitoring Service) berhasil terhubung dan siap mengirim alert ke user.",
    )
    .await
    {
        Ok(()) => (
            StatusCode::OK,
            Json(ApiEnvelope::ok(format!("Email tes berhasil dikirim ke {}", req.to_email.trim()))),
        ),
        Err(err) => (StatusCode::BAD_REQUEST, Json(ApiEnvelope::err(err))),
    }
}

#[derive(Debug, Serialize)]
pub struct AgentPackageDto {
    pub id: String,
    pub os_label: String,
    pub arch: String,
    pub filename: String,
    pub description: String,
    pub available_locally: bool,
    pub size_bytes: u64,
    pub direct_download_url: String,
    pub github_release_url: String,
}

fn resolve_agent_file_path(filename: &str) -> Option<std::path::PathBuf> {
    // Validasi ketat mencegah path traversal: hanya izinkan karakter alfanumerik, '-', '_', '.'
    if filename.is_empty()
        || filename.contains("..")
        || !filename
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
    {
        return None;
    }
    let primary_dir =
        std::env::var("AGENT_DOWNLOAD_DIR").unwrap_or_else(|_| "/app/downloads".to_string());
    let candidates = [
        std::path::PathBuf::from(primary_dir).join(filename),
        std::path::PathBuf::from("/app/downloads-builtin").join(filename),
    ];
    candidates.into_iter().find(|p| p.is_file())
}

async fn list_agent_downloads_handler() -> impl IntoResponse {
    let gh_base = std::env::var("GITHUB_RELEASES_URL")
        .unwrap_or_else(|_| "https://github.com/Dot-Dodik/cmkagent-rust/releases/latest".to_string());

    let catalog = [
        (
            "windows-x64",
            "Windows 10 / 11 / Server",
            "x86_64 (.exe)",
            "bms-agent-windows-x86_64.exe",
            "Binary agent ultra-ringan + installer Windows Service (MachineGuid UUID).",
        ),
        (
            "linux-x64",
            "Linux Desktop / Server",
            "x86_64 (ELF)",
            "bms-agent-linux-x86_64",
            "Agen ringan untuk Ubuntu/Mint/Debian lawas (< 4 MB, RAM < 8 MB, /proc + /sys).",
        ),
        (
            "linux-arm64",
            "Raspberry Pi / ARM64",
            "aarch64 (ELF)",
            "bms-agent-linux-aarch64",
            "Agen khusus ARM64 / Raspberry Pi & mini server cabang.",
        ),
        (
            "android-apk",
            "Android PDA & Tablet",
            "Universal (.apk)",
            "bms-agent-android-v2.0.0.apk",
            "Aplikasi Android dengan ForegroundService, baterai real health, WiFi roaming & ping.",
        ),
        (
            "branch-proxy",
            "Linux Branch Proxy Poller",
            "x86_64 (Proxy)",
            "bms-agent-proxy-linux-x86_64",
            "Agen kolektor cabang (1 per cabang) untuk polling Router, Switch, Firewall, & Printer.",
        ),
    ];

    let items: Vec<AgentPackageDto> = catalog
        .into_iter()
        .map(|(id, os_label, arch, filename, desc)| {
            let file_opt = resolve_agent_file_path(filename);
            let size_bytes = file_opt
                .as_ref()
                .and_then(|p| std::fs::metadata(p).ok())
                .map(|m| m.len())
                .unwrap_or(0);
            AgentPackageDto {
                id: id.to_string(),
                os_label: os_label.to_string(),
                arch: arch.to_string(),
                filename: filename.to_string(),
                description: desc.to_string(),
                available_locally: file_opt.is_some(),
                size_bytes,
                direct_download_url: format!("/api/v1/downloads/file/{}", filename),
                github_release_url: gh_base.clone(),
            }
        })
        .collect();

    (StatusCode::OK, Json(ApiEnvelope::ok(items)))
}

async fn download_agent_file_handler(Path(filename): Path<String>) -> Response {
    let Some(path) = resolve_agent_file_path(&filename) else {
        return (
            StatusCode::NOT_FOUND,
            Json(ApiEnvelope::<()>::err(
                "Berkas installer belum tersedia di folder /app/downloads server. Letakkan file di ./dist/agents/ atau unduh dari GitHub Releases.",
            )),
        )
            .into_response();
    };

    match tokio::fs::read(&path).await {
        Ok(bytes) => {
            let disp = format!("attachment; filename=\"{}\"", filename);
            (
                StatusCode::OK,
                [
                    (header::CONTENT_TYPE, "application/octet-stream".to_string()),
                    (header::CONTENT_DISPOSITION, disp),
                ],
                bytes,
            )
                .into_response()
        }
        Err(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiEnvelope::<()>::err("Gagal membaca berkas agent dari disk")),
        )
            .into_response(),
    }
}

#[derive(Debug, Deserialize)]
pub struct InstallerScriptQuery {
    pub branch: Option<String>,
    pub server_url: Option<String>,
}

async fn download_linux_installer_handler(
    State(state): State<Arc<AppState>>,
    Query(q): Query<InstallerScriptQuery>,
) -> Response {
    let rt = state.runtime_settings.read().await.clone();
    let branch = q.branch.unwrap_or_else(|| state.cfg.default_branch_code.clone());
    let server_url = q
        .server_url
        .filter(|s| !s.trim().is_empty())
        .unwrap_or(rt.rest_api_public_url);

    let script = format!(
        r#"#!/usr/bin/env bash
set -euo pipefail
SERVER_URL="{server_url}"
BRANCH_CODE="{branch}"
ENROLL_TOKEN="{token}"
ARCH="$(uname -m)"
BIN_NAME="bms-agent-linux-x86_64"
if [ "$ARCH" = "aarch64" ] || [ "$ARCH" = "arm64" ]; then
  BIN_NAME="bms-agent-linux-aarch64"
fi

echo "[BMS] Mengunduh $BIN_NAME dari $SERVER_URL..."
curl -fSL "$SERVER_URL/api/v1/downloads/file/$BIN_NAME" -o /usr/local/bin/bms-agent
chmod +x /usr/local/bin/bms-agent

mkdir -p /etc/bms-agent /var/lib/bms-agent
cat > /etc/bms-agent/agent.env <<EOF
AGENT_SERVER_URL=$SERVER_URL
AGENT_BRANCH_CODE=$BRANCH_CODE
AGENT_ENROLLMENT_TOKEN=$ENROLL_TOKEN
AGENT_SECRET_FILE=/var/lib/bms-agent/agent.secret
EOF

cat > /etc/systemd/system/bms-agent.service <<EOF
[Unit]
Description=BMS (Borwita Monitoring Service) Agent
After=network-online.target

[Service]
EnvironmentFile=/etc/bms-agent/agent.env
ExecStart=/usr/local/bin/bms-agent
Restart=always
RestartSec=10

[Install]
WantedBy=multi-user.target
EOF

systemctl daemon-reload
systemctl enable --now bms-agent.service
echo "[BMS] Agent aktif di cabang $BRANCH_CODE!"
"#,
        server_url = server_url.trim_end_matches('/'),
        branch = branch,
        token = rt.default_enrollment_token
    );

    (
        StatusCode::OK,
        [
            (header::CONTENT_TYPE, "text/x-shellscript; charset=utf-8"),
            (
                header::CONTENT_DISPOSITION,
                "attachment; filename=\"install-bms-linux.sh\"",
            ),
        ],
        script,
    )
        .into_response()
}

async fn download_windows_installer_handler(
    State(state): State<Arc<AppState>>,
    Query(q): Query<InstallerScriptQuery>,
) -> Response {
    let rt = state.runtime_settings.read().await.clone();
    let branch = q.branch.unwrap_or_else(|| state.cfg.default_branch_code.clone());
    let server_url = q
        .server_url
        .filter(|s| !s.trim().is_empty())
        .unwrap_or(rt.rest_api_public_url);

    let script = format!(
        r#"# BMS (Borwita Monitoring Service) Windows Installer
$ServerUrl = "{server_url}"
$BranchCode = "{branch}"
$EnrollToken = "{token}"
$InstallDir = "C:\Program Files\BMS-Agent"
New-Item -ItemType Directory -Force -Path $InstallDir | Out-Null
New-Item -ItemType Directory -Force -Path "C:\ProgramData\bms-agent" | Out-Null

Write-Host "[BMS] Mengunduh bms-agent-windows-x86_64.exe dari $ServerUrl..."
Invoke-WebRequest -Uri "$ServerUrl/api/v1/downloads/file/bms-agent-windows-x86_64.exe" -OutFile "$InstallDir\bms-agent.exe"

[Environment]::SetEnvironmentVariable("AGENT_SERVER_URL", $ServerUrl, "Machine")
[Environment]::SetEnvironmentVariable("AGENT_BRANCH_CODE", $BranchCode, "Machine")
[Environment]::SetEnvironmentVariable("AGENT_ENROLLMENT_TOKEN", $EnrollToken, "Machine")
[Environment]::SetEnvironmentVariable("AGENT_SECRET_FILE", "C:\ProgramData\bms-agent\agent.secret", "Machine")

sc.exe create bms-agent binPath= "`"$InstallDir\bms-agent.exe`"" start= auto | Out-Null
sc.exe start bms-agent | Out-Null
Write-Host "[BMS] Windows Agent terpasang & berjalan di cabang $BranchCode!"
"#,
        server_url = server_url.trim_end_matches('/'),
        branch = branch,
        token = rt.default_enrollment_token
    );

    (
        StatusCode::OK,
        [
            (header::CONTENT_TYPE, "text/plain; charset=utf-8"),
            (
                header::CONTENT_DISPOSITION,
                "attachment; filename=\"install-bms-windows.ps1\"",
            ),
        ],
        script,
    )
        .into_response()
}
