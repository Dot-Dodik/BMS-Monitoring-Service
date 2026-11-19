use hmac::{Hmac, Mac};
use sha2::{Digest, Sha256};

type HmacSha256 = Hmac<Sha256>;

/// Jendela waktu anti-replay maksimal (±300 detik / 5 menit)
pub const MAX_TIMESTAMP_SKEW_SEC: i64 = 300;

/// Menghasilkan SHA-256 hex dari token pendaftaran cabang (untuk disimpan/dicocokkan di DB)
pub fn sha256_hex(input: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(input.as_bytes());
    hex::encode(hasher.finalize())
}

/// Menghasilkan `AgentSecret` deterministik namun tidak dapat ditebak per `host_uuid`
pub fn derive_agent_secret(master_pepper: &str, host_uuid: &str, branch_code: &str) -> String {
    let mut mac =
        HmacSha256::new_from_slice(master_pepper.as_bytes()).expect("HMAC accepts any key length");
    mac.update(host_uuid.trim().as_bytes());
    mac.update(b":");
    mac.update(branch_code.trim().as_bytes());
    hex::encode(mac.finalize().into_bytes())
}

/// Menghitung signature HMAC-SHA256 atas kanonik `"{host_uuid}\n{timestamp_sec}\n"` + `body_bytes`
pub fn sign_ingest_payload(
    agent_secret: &str,
    host_uuid: &str,
    timestamp_sec: i64,
    body_bytes: &[u8],
) -> String {
    let mut mac =
        HmacSha256::new_from_slice(agent_secret.as_bytes()).expect("HMAC accepts any key length");
    let header_part = format!("{}\n{}\n", host_uuid.trim(), timestamp_sec);
    mac.update(header_part.as_bytes());
    mac.update(body_bytes);
    hex::encode(mac.finalize().into_bytes())
}

/// Memverifikasi jendela waktu anti-replay (±5 menit) dan kecocokan HMAC-SHA256 secara constant-time
pub fn verify_ingest_signature(
    agent_secret: &str,
    host_uuid: &str,
    timestamp_sec: i64,
    now_sec: i64,
    body_bytes: &[u8],
    provided_signature_hex: &str,
) -> Result<(), &'static str> {
    let diff = (now_sec - timestamp_sec).abs();
    if diff > MAX_TIMESTAMP_SKEW_SEC {
        return Err("Timestamp di luar batas anti-replay (±5 menit)");
    }

    let Ok(sig_bytes) = hex::decode(provided_signature_hex.trim()) else {
        return Err("Format X-Signature bukan hex valid");
    };

    let mut mac =
        HmacSha256::new_from_slice(agent_secret.as_bytes()).expect("HMAC accepts any key length");
    let header_part = format!("{}\n{}\n", host_uuid.trim(), timestamp_sec);
    mac.update(header_part.as_bytes());
    mac.update(body_bytes);

    mac.verify_slice(&sig_bytes)
        .map_err(|_| "Verifikasi HMAC-SHA256 gagal (AgentSecret tidak cocok)")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hmac_sign_and_verify_anti_replay() {
        let secret = derive_agent_secret("pepper-123", "uuid-abc-001", "KDR-HQ");
        let body = br#"{"hello":"world"}"#;
        let ts = 1_760_000_000;
        let sig = sign_ingest_payload(&secret, "uuid-abc-001", ts, body);

        // Valid within ±300s
        assert!(verify_ingest_signature(&secret, "uuid-abc-001", ts, ts + 120, body, &sig).is_ok());

        // Expired replay (> 300s)
        assert!(verify_ingest_signature(&secret, "uuid-abc-001", ts, ts + 305, body, &sig).is_err());

        // Tampered body
        assert!(verify_ingest_signature(
            &secret,
            "uuid-abc-001",
            ts,
            ts + 10,
            br#"{"hello":"tampered"}"#,
            &sig
        )
        .is_err());
    }
}
