use std::env;

#[derive(Debug, Clone)]
pub struct ServerConfig {
    pub bind_addr: String,
    pub database_url: String,
    pub db_max_connections: u32,
    pub jwt_secret: String,
    pub hmac_master_pepper: String,
    pub default_branch_code: String,
    pub default_branch_name: String,
    pub default_enrollment_token: String,
    pub batch_flush_interval_sec: u64,
    pub stale_check_interval_sec: u64,
    pub host_stale_threshold_sec: i64,
    pub legacy_pull_enabled: bool,
    pub legacy_pull_interval_sec: u64,
    pub legacy_pull_timeout_sec: u64,
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
}

impl ServerConfig {
    pub fn from_env() -> Result<Self, String> {
        let _ = dotenvy::dotenv();

        let database_url = env::var("DATABASE_URL")
            .map_err(|_| "Variabel lingkungan DATABASE_URL wajib diisi di .env".to_string())?;
        let jwt_secret = env::var("JWT_SECRET")
            .map_err(|_| "Variabel lingkungan JWT_SECRET wajib diisi di .env".to_string())?;
        let hmac_master_pepper = env::var("HMAC_MASTER_PEPPER").map_err(|_| {
            "Variabel lingkungan HMAC_MASTER_PEPPER wajib diisi di .env".to_string()
        })?;

        Ok(Self {
            bind_addr: env::var("SERVER_BIND_ADDR")
                .unwrap_or_else(|_| "0.0.0.0:8080".to_string()),
            database_url,
            db_max_connections: env::var("DB_MAX_CONNECTIONS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(30),
            jwt_secret,
            hmac_master_pepper,
            default_branch_code: env::var("DEFAULT_BRANCH_CODE").unwrap_or_default(),
            default_branch_name: env::var("DEFAULT_BRANCH_NAME").unwrap_or_default(),
            default_enrollment_token: env::var("DEFAULT_ENROLLMENT_TOKEN").unwrap_or_default(),
            batch_flush_interval_sec: env::var("BATCH_FLUSH_INTERVAL_SEC")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(180),
            stale_check_interval_sec: env::var("STALE_CHECK_INTERVAL_SEC")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(30),
            host_stale_threshold_sec: env::var("HOST_STALE_THRESHOLD_SEC")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(180),
            legacy_pull_enabled: env::var("LEGACY_PULL_ENABLED")
                .map(|v| v.eq_ignore_ascii_case("true") || v == "1")
                .unwrap_or(true),
            legacy_pull_interval_sec: env::var("LEGACY_PULL_INTERVAL_SEC")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(60),
            legacy_pull_timeout_sec: env::var("LEGACY_PULL_TIMEOUT_SEC")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(5),
            flapping_threshold_cycles: env::var("FLAPPING_THRESHOLD_CYCLES")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(2),
            telegram_bot_token: env::var("TELEGRAM_BOT_TOKEN").unwrap_or_default(),
            telegram_default_chat_id: env::var("TELEGRAM_DEFAULT_CHAT_ID").unwrap_or_default(),
            wa_webhook_url: env::var("WA_WEBHOOK_URL").unwrap_or_default(),
            wa_webhook_token: env::var("WA_WEBHOOK_TOKEN").unwrap_or_default(),
            smtp_host: env::var("SMTP_HOST").unwrap_or_default(),
            smtp_port: env::var("SMTP_PORT")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(587),
            smtp_username: env::var("SMTP_USERNAME").unwrap_or_default(),
            smtp_password: env::var("SMTP_PASSWORD").unwrap_or_default(),
            smtp_from: env::var("SMTP_FROM")
                .unwrap_or_else(|_| "Monitoring IT Ops <alert@company.co.id>".to_string()),
        })
    }
}
