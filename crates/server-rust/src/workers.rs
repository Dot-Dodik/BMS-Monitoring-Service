use crate::state::{AlertEvent, AlertEventType, AppState, CachedServiceState};
use chrono::Utc;
use monitoring_proto::checkmk_fmt::parse_checkmk_local_lines;
use monitoring_proto::model::MetricTier;
use sqlx::{MySql, QueryBuilder};
use std::sync::Arc;
use std::time::Duration;
use tokio::io::AsyncReadExt;
use tokio::net::TcpStream;
use tokio::sync::mpsc;

/// Menulis perubahan service secara instan (Immediate Diff-Only Write) beserta log insiden
pub async fn persist_immediate_service_transitions(
    state: &AppState,
    host_id: u64,
    host_status: &str,
    hostname: &str,
    ip_address: &str,
    transitions: &[CachedServiceState],
) {
    let _ = sqlx::query(
        "UPDATE hosts SET hostname = ?, ip_address = ?, status = ?, last_seen_at = UTC_TIMESTAMP(), last_state_change_at = UTC_TIMESTAMP() WHERE id = ?",
    )
    .bind(hostname)
    .bind(ip_address)
    .bind(host_status)
    .bind(host_id)
    .execute(&state.db)
    .await;

    for svc in transitions {
        let _ = sqlx::query(
            r#"INSERT INTO services
               (host_id, service_name, tier, state, prev_state, consecutive_non_ok, summary, metrics, last_check_at, last_state_change_at)
               VALUES (?, ?, ?, ?, ?, ?, ?, ?, UTC_TIMESTAMP(), UTC_TIMESTAMP())
               ON DUPLICATE KEY UPDATE
                 tier = VALUES(tier),
                 prev_state = VALUES(prev_state),
                 state = VALUES(state),
                 consecutive_non_ok = VALUES(consecutive_non_ok),
                 summary = VALUES(summary),
                 metrics = VALUES(metrics),
                 last_check_at = UTC_TIMESTAMP(),
                 last_state_change_at = IF(state <> VALUES(state), UTC_TIMESTAMP(), last_state_change_at)"#,
        )
        .bind(host_id)
        .bind(&svc.service_name)
        .bind(svc.tier.as_db_str())
        .bind(svc.state)
        .bind(svc.prev_state)
        .bind(svc.consecutive_non_ok)
        .bind(&svc.summary)
        .bind(&svc.metrics)
        .execute(&state.db)
        .await;

        if svc.prev_state != svc.state {
            let _ = sqlx::query(
                "INSERT INTO incident_logs (host_id, service_name, from_state, to_state, summary) VALUES (?, ?, ?, ?, ?)",
            )
            .bind(host_id)
            .bind(&svc.service_name)
            .bind(svc.prev_state)
            .bind(svc.state)
            .bind(&svc.summary)
            .execute(&state.db)
            .await;
        }
    }
}

/// WORKER 1: Batch Flusher (Setiap 3 Menit)
/// Mem-flush state yang tidak berubah (`OK -> OK`) dari RAM `DashMap` ke MariaDB per chunk 500 baris.
pub async fn batch_flusher_worker(state: Arc<AppState>) {
    let interval_sec = state.cfg.batch_flush_interval_sec.max(30);
    let mut ticker = tokio::time::interval(Duration::from_secs(interval_sec));

    loop {
        ticker.tick().await;
        flush_dirty_cache_to_db(&state).await;
    }
}

pub async fn flush_dirty_cache_to_db(state: &AppState) {
    let mut dirty_hosts: Vec<(u64, String, String, String, i64)> = Vec::new();
    let mut dirty_services: Vec<(u64, CachedServiceState)> = Vec::new();

    for mut entry in state.hosts_cache.iter_mut() {
        let h = entry.value_mut();
        if h.dirty_heartbeat {
            dirty_hosts.push((
                h.host_id,
                h.hostname.clone(),
                h.ip_address.clone(),
                h.status.clone(),
                h.last_seen_epoch,
            ));
            h.dirty_heartbeat = false;
        }
        let host_id = h.host_id;
        for svc in h.services.values_mut() {
            if svc.dirty_for_batch {
                dirty_services.push((host_id, svc.clone()));
                svc.dirty_for_batch = false;
            }
        }
    }

    if dirty_hosts.is_empty() && dirty_services.is_empty() {
        return;
    }

    tracing::info!(
        hosts = dirty_hosts.len(),
        services = dirty_services.len(),
        "Executing 3-minute batch flush from DashMap to MariaDB"
    );

    // Update heartbeat host
    for (host_id, hostname, ip_addr, status, _epoch) in dirty_hosts {
        let _ = sqlx::query(
            "UPDATE hosts SET hostname = ?, ip_address = ?, status = ?, last_seen_at = UTC_TIMESTAMP() WHERE id = ?",
        )
        .bind(hostname)
        .bind(ip_addr)
        .bind(status)
        .bind(host_id)
        .execute(&state.db)
        .await;
    }

    // Multi-row bulk upsert untuk services dalam chunk 500 baris
    for chunk in dirty_services.chunks(500) {
        let mut qb: QueryBuilder<MySql> = QueryBuilder::new(
            "INSERT INTO services (host_id, service_name, tier, state, prev_state, consecutive_non_ok, summary, metrics, last_check_at) ",
        );
        qb.push_values(chunk, |mut b, (host_id, svc)| {
            b.push_bind(*host_id)
                .push_bind(&svc.service_name)
                .push_bind(svc.tier.as_db_str())
                .push_bind(svc.state)
                .push_bind(svc.prev_state)
                .push_bind(svc.consecutive_non_ok)
                .push_bind(&svc.summary)
                .push_bind(&svc.metrics)
                .push("UTC_TIMESTAMP()");
        });
        qb.push(
            " ON DUPLICATE KEY UPDATE summary = VALUES(summary), metrics = VALUES(metrics), last_check_at = UTC_TIMESTAMP()",
        );
        let _ = qb.build().execute(&state.db).await;
    }
}

/// WORKER 2: Stale Host Detector (Setiap 30 Detik)
/// Mengecek `last_seen_epoch` di RAM; jika lewat dari 180 detik, ubah status host menjadi `DOWN` & kirim alert.
pub async fn stale_host_detector_worker(state: Arc<AppState>) {
    let interval_sec = state.cfg.stale_check_interval_sec.max(10);
    let threshold_sec = state.cfg.host_stale_threshold_sec.max(60);
    let mut ticker = tokio::time::interval(Duration::from_secs(interval_sec));

    loop {
        ticker.tick().await;
        let now_sec = Utc::now().timestamp();
        let mut newly_down = Vec::new();

        for mut entry in state.hosts_cache.iter_mut() {
            let h = entry.value_mut();
            if h.approval_status != "APPROVED" || h.last_seen_epoch == 0 {
                continue;
            }
            if h.status != "DOWN" && (now_sec - h.last_seen_epoch) > threshold_sec {
                h.status = "DOWN".to_string();
                newly_down.push((
                    h.host_id,
                    h.branch_code.clone(),
                    h.hostname.clone(),
                    h.ip_address.clone(),
                    now_sec - h.last_seen_epoch,
                    h.is_in_downtime,
                    h.telegram_chat_id.clone(),
                    h.telegram_topic_id,
                ));
            }
        }

        for (host_id, branch_code, hostname, ip_address, silent_sec, in_downtime, chat_id, topic_id) in
            newly_down
        {
            let summary = format!(
                "Host {} ({}) OFFLINE / STALE (tidak mengirim heartbeat selama {} detik)",
                hostname, ip_address, silent_sec
            );
            let _ = sqlx::query(
                "UPDATE hosts SET status = 'DOWN', last_state_change_at = UTC_TIMESTAMP() WHERE id = ?",
            )
            .bind(host_id)
            .execute(&state.db)
            .await;

            let _ = sqlx::query(
                "INSERT INTO incident_logs (host_id, service_name, from_state, to_state, summary) VALUES (?, '_HOST_', 0, 2, ?)",
            )
            .bind(host_id)
            .bind(&summary)
            .execute(&state.db)
            .await;

            if !in_downtime {
                let _ = state
                    .alert_tx
                    .send(AlertEvent {
                        host_id,
                        branch_code,
                        hostname,
                        ip_address,
                        service_name: "_HOST_".to_string(),
                        state: 2,
                        event_type: AlertEventType::Problem,
                        summary,
                        telegram_chat_id: chat_id,
                        telegram_topic_id: topic_id,
                        owner_name: None,
                        owner_wa: None,
                        owner_email: None,
                        is_user_routable: false,
                    })
                    .await;
            }
        }
    }
}

/// WORKER 3: Legacy TCP/6556 Puller (Untuk agen Checkmk legacy yang belum memakai HTTP Push)
pub async fn legacy_tcp_puller_worker(state: Arc<AppState>) {
    if !state.cfg.legacy_pull_enabled {
        return;
    }
    let interval_sec = state.cfg.legacy_pull_interval_sec.max(30);
    let timeout_dur = Duration::from_secs(state.cfg.legacy_pull_timeout_sec.max(2));
    let mut ticker = tokio::time::interval(Duration::from_secs(interval_sec));

    loop {
        ticker.tick().await;

        let rows: Vec<(String, String, String, u16)> = sqlx::query_as(
            "SELECT host_uuid, hostname, ip_address, pull_legacy_port FROM hosts WHERE pull_legacy_enabled = 1 AND approval_status = 'APPROVED'",
        )
        .fetch_all(&state.db)
        .await
        .unwrap_or_default();

        for (host_uuid, hostname, ip_address, port) in rows {
            if ip_address == "0.0.0.0" || ip_address.is_empty() {
                continue;
            }
            let addr = format!("{}:{}", ip_address, port);
            if let Ok(Ok(mut stream)) =
                tokio::time::timeout(timeout_dur, TcpStream::connect(&addr)).await
            {
                let mut buf = Vec::with_capacity(16384);
                if tokio::time::timeout(timeout_dur, stream.read_to_end(&mut buf))
                    .await
                    .is_ok()
                {
                    let raw = String::from_utf8_lossy(&buf);
                    let checks = parse_checkmk_local_lines(&raw, MetricTier::Fast);
                    let now_sec = Utc::now().timestamp();
                    if let Some((host_id, host_status, immediate, alerts)) = state
                        .apply_ingest_in_memory(&host_uuid, &hostname, &ip_address, now_sec, &checks)
                    {
                        if !immediate.is_empty() {
                            persist_immediate_service_transitions(
                                &state,
                                host_id,
                                &host_status,
                                &hostname,
                                &ip_address,
                                &immediate,
                            )
                            .await;
                        }
                        for alert in alerts {
                            let _ = state.alert_tx.send(alert).await;
                        }
                    }
                }
            }
        }
    }
}

/// WORKER 4: Decoupled Notification Queue Worker
/// Memisahkan pengiriman Telegram Group Topic (IT Ops Cabang) & WhatsApp/Email (Pemilik Perangkat)
pub async fn notification_worker(state: Arc<AppState>, mut rx: mpsc::Receiver<AlertEvent>) {
    let http = reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .unwrap_or_default();

    while let Some(ev) = rx.recv().await {
        let badge = match (ev.event_type, ev.state) {
            (AlertEventType::Recovery, _) => "✅ [RECOVERY]",
            (_, 2) => "🚨 [CRITICAL]",
            (_, 1) => "⚠️ [WARNING]",
            _ => "ℹ️ [INFO]",
        };

        // 1. Routing ke Telegram Group Topic IT Ops Cabang
        // Dikirim untuk insiden host DOWN/UP, service CRITICAL, atau infrastruktur cabang
        let should_notify_it = ev.service_name == "_HOST_"
            || ev.state >= 2
            || ev.event_type == AlertEventType::Recovery
            || !ev.is_user_routable;

        if should_notify_it {
            let rt = state.runtime_settings.read().await;
            let chat_id = ev
                .telegram_chat_id
                .clone()
                .filter(|s| !s.is_empty())
                .unwrap_or_else(|| rt.telegram_default_chat_id.clone());
            drop(rt);

            if !chat_id.is_empty() {
                let tg_msg = format!(
                    "{} *{}* — `{}` (`{}`)\n• Service: `{}`\n• Detail: {}",
                    badge, ev.branch_code, ev.hostname, ev.ip_address, ev.service_name, ev.summary
                );
                let dest = match ev.telegram_topic_id {
                    Some(tid) if tid > 0 => format!("{}:{}", chat_id, tid),
                    _ => chat_id.clone(),
                };
                dispatch_or_enqueue_outbox(
                    &state,
                    &http,
                    ev.host_id,
                    &ev.service_name,
                    "IT_TELEGRAM",
                    &dest,
                    ev.event_type,
                    &tg_msg,
                )
                .await;
            }
        }

        // 2. Routing ke End-User / Pemilik Perangkat (WhatsApp Webhook & Email)
        // Hanya dikirim jika service masuk daftar `user_alert_services` (misal Health_Battery, Storage_Usage, Printer_Paper)
        if ev.is_user_routable {
            let owner_greeting = ev.owner_name.as_deref().unwrap_or("Rekan Operasional");
            let user_msg = format!(
                "{} Halo {}, perangkat *{}* ({}) melaporkan kondisi pada *{}*:\n{}\n\nMohon dicek secara mandiri (misal: isi daya baterai / bersihkan ruang simpan / cek kertas printer). Terima kasih.",
                badge, owner_greeting, ev.hostname, ev.branch_code, ev.service_name, ev.summary
            );

            if let Some(wa) = ev.owner_wa.as_deref().filter(|s| !s.trim().is_empty()) {
                dispatch_or_enqueue_outbox(
                    &state,
                    &http,
                    ev.host_id,
                    &ev.service_name,
                    "OWNER_WA",
                    wa.trim(),
                    ev.event_type,
                    &user_msg,
                )
                .await;
            }
            if let Some(email) = ev.owner_email.as_deref().filter(|s| !s.trim().is_empty()) {
                dispatch_or_enqueue_outbox(
                    &state,
                    &http,
                    ev.host_id,
                    &ev.service_name,
                    "OWNER_EMAIL",
                    email.trim(),
                    ev.event_type,
                    &user_msg,
                )
                .await;
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
async fn dispatch_or_enqueue_outbox(
    state: &AppState,
    http: &reqwest::Client,
    host_id: u64,
    service_name: &str,
    target_role: &str,
    destination: &str,
    event_type: AlertEventType,
    message_body: &str,
) {
    let send_res = match target_role {
        "IT_TELEGRAM" => send_telegram_topic_msg(state, http, destination, message_body).await,
        "OWNER_WA" => send_whatsapp_webhook(state, http, destination, message_body).await,
        "OWNER_EMAIL" => {
            send_smtp_email_with_settings(
                state,
                destination,
                &format!("[BMS] Alert pada service {}", service_name),
                message_body,
            )
            .await
        }
        _ => Ok(()),
    };

    let (status, last_err) = match send_res {
        Ok(()) => ("SENT", None),
        Err(e) => ("PENDING", Some(e)),
    };

    let _ = sqlx::query(
        r#"INSERT INTO notification_outbox
           (host_id, service_name, target_role, destination, event_type, message_body, delivery_status, last_error, sent_at)
           VALUES (?, ?, ?, ?, ?, ?, ?, ?, IF(? = 'SENT', UTC_TIMESTAMP(), NULL))"#,
    )
    .bind(host_id)
    .bind(service_name)
    .bind(target_role)
    .bind(destination)
    .bind(event_type.as_db_str())
    .bind(message_body)
    .bind(status)
    .bind(last_err)
    .bind(status)
    .execute(&state.db)
    .await;
}

async fn send_telegram_topic_msg(
    state: &AppState,
    http: &reqwest::Client,
    dest: &str,
    text: &str,
) -> Result<(), String> {
    let token = {
        let rt = state.runtime_settings.read().await;
        rt.telegram_bot_token.trim().to_string()
    };
    if token.is_empty() {
        return Err("TELEGRAM_BOT_TOKEN belum dikonfigurasi di .env / Settings".to_string());
    }
    let (chat_id, topic_opt) = match dest.split_once(':') {
        Some((c, t)) => (c, t.parse::<i32>().ok()),
        None => (dest, None),
    };

    let url = format!("https://api.telegram.org/bot{}/sendMessage", token);
    let mut body = serde_json::json!({
        "chat_id": chat_id,
        "text": text,
        "parse_mode": "Markdown"
    });
    if let Some(tid) = topic_opt {
        body["message_thread_id"] = serde_json::json!(tid);
    }

    let resp = http
        .post(&url)
        .json(&body)
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if resp.status().is_success() {
        Ok(())
    } else {
        Err(format!("Telegram HTTP {}", resp.status()))
    }
}

async fn send_whatsapp_webhook(
    state: &AppState,
    http: &reqwest::Client,
    phone: &str,
    text: &str,
) -> Result<(), String> {
    let (wa_url, wa_token) = {
        let rt = state.runtime_settings.read().await;
        (
            rt.wa_webhook_url.trim().to_string(),
            rt.wa_webhook_token.trim().to_string(),
        )
    };
    if wa_url.is_empty() {
        return Err("WA_WEBHOOK_URL belum dikonfigurasi di .env / Settings".to_string());
    }
    let mut req = http.post(&wa_url).json(&serde_json::json!({
        "phone": phone,
        "message": text,
    }));
    if !wa_token.is_empty() {
        req = req.bearer_auth(&wa_token);
    }
    let resp = req.send().await.map_err(|e| e.to_string())?;
    if resp.status().is_success() {
        Ok(())
    } else {
        Err(format!("WA Webhook HTTP {}", resp.status()))
    }
}

/// Mengirim email via SMTP Utama (Gmail App Password, Office 365, atau Mail Server Internal)
pub async fn send_smtp_email_with_settings(
    state: &AppState,
    to_email: &str,
    subject: &str,
    body_text: &str,
) -> Result<(), String> {
    use lettre::transport::smtp::authentication::Credentials;
    use lettre::{AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor};

    let rt = state.runtime_settings.read().await.clone();
    if rt.smtp_host.trim().is_empty() || rt.smtp_from.trim().is_empty() {
        return Err("SMTP Host / Email Pengirim Utama belum dikonfigurasi".to_string());
    }

    let from_mb: lettre::message::Mailbox = rt
        .smtp_from
        .trim()
        .parse()
        .map_err(|e| format!("Format Email Pengirim (SMTP_FROM) tidak valid: {}", e))?;
    let to_mb: lettre::message::Mailbox = to_email
        .trim()
        .parse()
        .map_err(|e| format!("Format email tujuan tidak valid: {}", e))?;

    let email = Message::builder()
        .from(from_mb)
        .to(to_mb)
        .subject(subject)
        .body(body_text.to_string())
        .map_err(|e| format!("Gagal menyusun email: {}", e))?;

    let mut builder = if rt.smtp_use_tls {
        AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(rt.smtp_host.trim())
            .map_err(|e| format!("Gagal setup SMTP STARTTLS: {}", e))?
    } else {
        AsyncSmtpTransport::<Tokio1Executor>::builder_dangerous(rt.smtp_host.trim())
    };
    builder = builder.port(rt.smtp_port);

    if !rt.smtp_username.trim().is_empty() {
        builder = builder.credentials(Credentials::new(
            rt.smtp_username.trim().to_string(),
            rt.smtp_password.clone(),
        ));
    }

    let mailer = builder.build();
    mailer
        .send(email)
        .await
        .map(|_| ())
        .map_err(|e| format!("SMTP send gagal: {}", e))
}
