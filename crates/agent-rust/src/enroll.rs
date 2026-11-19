use crate::identity::DeviceIdentity;
use monitoring_proto::model::{ApiEnvelope, EnrollRequest, EnrollResponse, IngestPayload};
use monitoring_proto::security::sign_ingest_payload;
use std::path::Path;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

pub struct AgentTransportClient {
    pub server_url: String,
    pub branch_code: String,
    pub enrollment_token: String,
    pub secret_file: String,
    agent_secret: Option<String>,
    http: ureq::Agent,
}

impl AgentTransportClient {
    pub fn new(
        server_url: String,
        branch_code: String,
        enrollment_token: String,
        secret_file: String,
    ) -> Self {
        let http = ureq::AgentBuilder::new()
            .timeout_connect(Duration::from_secs(8))
            .timeout_read(Duration::from_secs(12))
            .timeout_write(Duration::from_secs(12))
            .build();

        let existing_secret = std::fs::read_to_string(&secret_file)
            .ok()
            .map(|s| s.trim().to_string())
            .filter(|s| s.len() >= 16);

        Self {
            server_url: server_url.trim_end_matches('/').to_string(),
            branch_code,
            enrollment_token,
            secret_file,
            agent_secret: existing_secret,
            http,
        }
    }

    /// Fase 1: Pastikan agen sudah memiliki `AgentSecret` unik. Jika belum, daftar via `/api/v1/register`.
    pub fn ensure_enrolled(&mut self, id: &DeviceIdentity) -> Result<String, String> {
        if let Some(sec) = &self.agent_secret {
            return Ok(sec.clone());
        }
        if let Ok(env_sec) = std::env::var("AGENT_SECRET") {
            let trimmed = env_sec.trim();
            if trimmed.len() >= 16 {
                self.agent_secret = Some(trimmed.to_string());
                return Ok(trimmed.to_string());
            }
        }

        let req_body = EnrollRequest {
            branch_code: self.branch_code.clone(),
            enrollment_token: self.enrollment_token.clone(),
            host_uuid: id.host_uuid.clone(),
            hostname: id.hostname.clone(),
            ip_address: id.ip_address.clone(),
            mac_address: id.mac_address.clone(),
            host_type: id.host_type.clone(),
            agent_version: env!("CARGO_PKG_VERSION").to_string(),
        };

        let url = format!("{}/api/v1/register", self.server_url);
        let resp = self
            .http
            .post(&url)
            .set("Content-Type", "application/json")
            .send_json(&req_body)
            .map_err(|e| format!("Gagal enroll ke {}: {}", url, e))?;

        let env_resp: ApiEnvelope<EnrollResponse> = resp
            .into_json()
            .map_err(|e| format!("Respons JSON enroll tidak valid: {}", e))?;

        let Some(data) = env_resp.data else {
            return Err(env_resp
                .error
                .unwrap_or_else(|| "Server menolak pendaftaran".to_string()));
        };

        if let Some(parent) = Path::new(&self.secret_file).parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let _ = std::fs::write(&self.secret_file, &data.agent_secret);
        self.agent_secret = Some(data.agent_secret.clone());
        Ok(data.agent_secret)
    }

    /// Fase 2: Kirim payload dengan tanda tangan `HMAC-SHA256` & timestamp segar (anti-replay ±5 menit)
    pub fn send_ingest(&mut self, id: &DeviceIdentity, payload: &IngestPayload) -> Result<(), String> {
        let secret = self.ensure_enrolled(id)?;

        // Gunakan timestamp saat pengiriman berlangsung agar paket dari ring buffer tetap lolos anti-replay
        let now_sec = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(payload.timestamp_sec);

        let body_bytes =
            serde_json::to_vec(payload).map_err(|e| format!("Gagal serialisasi JSON: {}", e))?;
        let signature = sign_ingest_payload(&secret, &payload.host_uuid, now_sec, &body_bytes);

        let url = format!("{}/api/v1/ingest", self.server_url);
        let res = self
            .http
            .post(&url)
            .set("Content-Type", "application/json")
            .set("X-Host-UUID", &payload.host_uuid)
            .set("X-Timestamp", &now_sec.to_string())
            .set("X-Signature", &signature)
            .send_bytes(&body_bytes);

        match res {
            Ok(r) if r.status() >= 200 && r.status() < 300 => Ok(()),
            Ok(r) => Err(format!("HTTP status {}", r.status())),
            Err(ureq::Error::Status(401, _)) => {
                // Jika secret di-rotasi oleh admin, kosongkan agar re-enroll pada siklus berikutnya
                self.agent_secret = None;
                Err("HTTP 401 Unauthorized (HMAC ditolak)".to_string())
            }
            Err(e) => Err(format!("Transport error: {}", e)),
        }
    }
}
