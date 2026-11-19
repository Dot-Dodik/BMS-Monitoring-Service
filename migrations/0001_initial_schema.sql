-- ============================================================================
-- CMK ENTERPRISE MONITOR - MARIADB 11 PRODUCTION SCHEMA (3,000+ DEVICES)
-- ============================================================================
SET NAMES utf8mb4;
SET FOREIGN_KEY_CHECKS = 0;

-- 1. Tabel Cabang & Konfigurasi Topik Telegram IT Ops
CREATE TABLE IF NOT EXISTS branches (
    id                    INT UNSIGNED AUTO_INCREMENT PRIMARY KEY,
    branch_code           VARCHAR(32)  NOT NULL COMMENT 'Contoh: KDR-HQ, SBY-01, MLG-02',
    branch_name           VARCHAR(128) NOT NULL,
    enrollment_token_hash VARCHAR(128) NOT NULL COMMENT 'SHA-256 hex dari ENROLLMENT_TOKEN cabang',
    subnet_cidr           VARCHAR(64)  DEFAULT NULL,
    telegram_chat_id      VARCHAR(64)  DEFAULT NULL COMMENT 'ID Supergroup Telegram IT Ops',
    telegram_topic_id     INT          DEFAULT NULL COMMENT 'message_thread_id untuk Forum Topic cabang',
    is_active             TINYINT(1)   NOT NULL DEFAULT 1,
    created_at            DATETIME     NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at            DATETIME     NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
    UNIQUE KEY uq_branches_code (branch_code)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

-- 2. Tabel Host / Perangkat (Identitas Zero-IP Lock + Metadata Notifikasi End-User)
CREATE TABLE IF NOT EXISTS hosts (
    id                    BIGINT UNSIGNED AUTO_INCREMENT PRIMARY KEY,
    host_uuid             VARCHAR(64)  NOT NULL COMMENT 'machine-id / MachineGuid / ANDROID_ID',
    branch_id             INT UNSIGNED NOT NULL,
    hostname              VARCHAR(128) NOT NULL,
    ip_address            VARCHAR(45)  NOT NULL DEFAULT '0.0.0.0',
    mac_address           VARCHAR(32)  DEFAULT NULL,
    host_type             VARCHAR(64)  NOT NULL DEFAULT 'LINUX' COMMENT 'WINDOWS, LINUX, ANDROID, ROUTER, SWITCH, FIREWALL, PRINTER, atau custom',
    agent_version         VARCHAR(32)  NOT NULL DEFAULT '2.0.0',
    agent_secret_hmac     VARCHAR(128) NOT NULL COMMENT 'Secret unik per host untuk HMAC-SHA256',
    pull_legacy_enabled   TINYINT(1)   NOT NULL DEFAULT 0,
    pull_legacy_port      SMALLINT UNSIGNED NOT NULL DEFAULT 6556,

    approval_status       ENUM('PENDING', 'APPROVED', 'CONFLICT', 'REVOKED') NOT NULL DEFAULT 'APPROVED',
    conflict_reason       VARCHAR(255) DEFAULT NULL,
    status                ENUM('UP', 'WARN', 'CRIT', 'DOWN', 'PENDING') NOT NULL DEFAULT 'PENDING',
    consecutive_failures  TINYINT UNSIGNED NOT NULL DEFAULT 0,
    is_in_downtime        TINYINT(1)   NOT NULL DEFAULT 0,

    owner_name            VARCHAR(128) DEFAULT NULL,
    owner_wa              VARCHAR(32)  DEFAULT NULL,
    owner_email           VARCHAR(128) DEFAULT NULL,
    user_alert_services   VARCHAR(512) NOT NULL DEFAULT 'Health_Battery,Storage_Usage,Printer_Paper',

    last_seen_at          DATETIME     DEFAULT NULL,
    last_state_change_at  DATETIME     DEFAULT NULL,
    created_at            DATETIME     NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at            DATETIME     NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,

    UNIQUE KEY uq_hosts_uuid (host_uuid),
    KEY idx_hosts_branch_status (branch_id, status, approval_status),
    KEY idx_hosts_search (hostname, ip_address),
    KEY idx_hosts_last_seen (last_seen_at),
    CONSTRAINT fk_hosts_branch FOREIGN KEY (branch_id) REFERENCES branches (id) ON DELETE RESTRICT
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

-- 3. Tabel Status Service Terkini per Host (Target Diff-Only Write & Batch Upsert 3 Menit)
CREATE TABLE IF NOT EXISTS services (
    id                    BIGINT UNSIGNED AUTO_INCREMENT PRIMARY KEY,
    host_id               BIGINT UNSIGNED NOT NULL,
    service_name          VARCHAR(128) NOT NULL,
    tier                  ENUM('FAST', 'MEDIUM', 'SLOW', 'PROXY') NOT NULL DEFAULT 'FAST',
    state                 TINYINT UNSIGNED NOT NULL DEFAULT 0 COMMENT '0=OK, 1=WARN, 2=CRIT, 3=UNKNOWN',
    prev_state            TINYINT UNSIGNED NOT NULL DEFAULT 0,
    consecutive_non_ok    TINYINT UNSIGNED NOT NULL DEFAULT 0,
    summary               VARCHAR(1024) NOT NULL DEFAULT '',
    metrics               VARCHAR(512)  NOT NULL DEFAULT '-',
    is_acknowledged       TINYINT(1)   NOT NULL DEFAULT 0,
    last_check_at         DATETIME     NOT NULL DEFAULT CURRENT_TIMESTAMP,
    last_state_change_at  DATETIME     NOT NULL DEFAULT CURRENT_TIMESTAMP,

    UNIQUE KEY uq_host_service (host_id, service_name),
    KEY idx_services_action_needed (state, is_acknowledged, host_id),
    CONSTRAINT fk_services_host FOREIGN KEY (host_id) REFERENCES hosts (id) ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

-- 4. Tabel Inventaris Aset & Kesehatan Hardware (Diperbarui oleh Slow Tier 6 Jam & Siap Export CSV)
CREATE TABLE IF NOT EXISTS host_inventory (
    host_id                BIGINT UNSIGNED PRIMARY KEY,
    os_name                VARCHAR(128) DEFAULT NULL,
    os_version             VARCHAR(64)  DEFAULT NULL,
    os_license_status      VARCHAR(64)  DEFAULT NULL,
    os_license_key_partial VARCHAR(32)  DEFAULT NULL,
    office_product         VARCHAR(128) DEFAULT NULL,
    office_license_status  VARCHAR(64)  DEFAULT NULL,
    cpu_model              VARCHAR(128) DEFAULT NULL,
    cpu_cores_threads      VARCHAR(32)  DEFAULT NULL,
    ram_total_gb           DECIMAL(6,2) DEFAULT NULL,
    ram_slots_summary      VARCHAR(128) DEFAULT NULL,
    disk_primary_model     VARCHAR(128) DEFAULT NULL,
    disk_primary_type      VARCHAR(32)  DEFAULT NULL,
    disk_health_pct        TINYINT UNSIGNED DEFAULT NULL,
    disk_tbw_written_tb    DECIMAL(8,2) DEFAULT NULL,
    disk_smart_status      VARCHAR(64)  DEFAULT NULL,
    battery_health_pct     DECIMAL(5,1) DEFAULT NULL,
    battery_cycles         INT UNSIGNED DEFAULT NULL,
    battery_design_mah     INT UNSIGNED DEFAULT NULL,
    battery_full_mah       INT UNSIGNED DEFAULT NULL,
    remote_anydesk_id      VARCHAR(64)  DEFAULT NULL,
    remote_rustdesk_id     VARCHAR(64)  DEFAULT NULL,
    raw_inventory_json     MEDIUMTEXT   DEFAULT NULL,
    updated_at             DATETIME     NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
    CONSTRAINT fk_inventory_host FOREIGN KEY (host_id) REFERENCES hosts (id) ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

-- 5. Tabel Riwayat Insiden (Hanya Ditulis Saat State Transition)
CREATE TABLE IF NOT EXISTS incident_logs (
    id                    BIGINT UNSIGNED AUTO_INCREMENT PRIMARY KEY,
    host_id               BIGINT UNSIGNED NOT NULL,
    service_name          VARCHAR(128) NOT NULL,
    from_state            TINYINT UNSIGNED NOT NULL,
    to_state              TINYINT UNSIGNED NOT NULL,
    summary               VARCHAR(1024) NOT NULL,
    created_at            DATETIME     NOT NULL DEFAULT CURRENT_TIMESTAMP,
    KEY idx_incidents_host_time (host_id, created_at DESC),
    KEY idx_incidents_created (created_at DESC),
    CONSTRAINT fk_incidents_host FOREIGN KEY (host_id) REFERENCES hosts (id) ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

-- 6. Tabel Antrean Notifikasi Persisten (Decoupled Outbox dengan Retry Backoff)
CREATE TABLE IF NOT EXISTS notification_outbox (
    id                    BIGINT UNSIGNED AUTO_INCREMENT PRIMARY KEY,
    host_id               BIGINT UNSIGNED NOT NULL,
    service_name          VARCHAR(128) NOT NULL,
    target_role           ENUM('IT_TELEGRAM', 'OWNER_WA', 'OWNER_EMAIL') NOT NULL,
    destination           VARCHAR(255) NOT NULL,
    event_type            ENUM('PROBLEM', 'RECOVERY') NOT NULL,
    message_body          TEXT         NOT NULL,
    delivery_status       ENUM('PENDING', 'SENT', 'FAILED') NOT NULL DEFAULT 'PENDING',
    retry_count           TINYINT UNSIGNED NOT NULL DEFAULT 0,
    next_retry_at         DATETIME     NOT NULL DEFAULT CURRENT_TIMESTAMP,
    last_error            VARCHAR(512) DEFAULT NULL,
    created_at            DATETIME     NOT NULL DEFAULT CURRENT_TIMESTAMP,
    sent_at               DATETIME     DEFAULT NULL,
    KEY idx_outbox_poll (delivery_status, next_retry_at)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

-- 7. Tabel Pengguna Dashboard (RBAC)
CREATE TABLE IF NOT EXISTS users (
    id                    INT UNSIGNED AUTO_INCREMENT PRIMARY KEY,
    username              VARCHAR(64)  NOT NULL,
    password_argon2       VARCHAR(255) NOT NULL,
    full_name             VARCHAR(128) NOT NULL,
    role                  ENUM('SUPERADMIN', 'IT_BRANCH', 'VIEWER') NOT NULL DEFAULT 'VIEWER',
    assigned_branch_id    INT UNSIGNED DEFAULT NULL,
    is_active             TINYINT(1)   NOT NULL DEFAULT 1,
    created_at            DATETIME     NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE KEY uq_users_username (username),
    CONSTRAINT fk_users_branch FOREIGN KEY (assigned_branch_id) REFERENCES branches (id) ON DELETE SET NULL
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

-- 8. Tabel Jenis Perangkat Dinamis (WINDOWS, LINUX, ANDROID, ROUTER, SWITCH, FIREWALL, PRINTER + Custom)
CREATE TABLE IF NOT EXISTS device_types (
    code                  VARCHAR(64)  PRIMARY KEY,
    label                 VARCHAR(128) NOT NULL,
    is_builtin            TINYINT(1)   NOT NULL DEFAULT 0,
    created_at            DATETIME     NOT NULL DEFAULT CURRENT_TIMESTAMP
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

INSERT IGNORE INTO device_types (code, label, is_builtin) VALUES
    ('WINDOWS', 'Windows', 1),
    ('LINUX', 'Linux', 1),
    ('ANDROID', 'Android', 1),
    ('ROUTER', 'Router', 1),
    ('SWITCH', 'Switch', 1),
    ('FIREWALL', 'Firewall', 1),
    ('PRINTER', 'Printer', 1);

-- 9. Tabel Pengaturan Sistem & Integrasi (SMTP Email Utama, WA Webhook, Telegram, REST API)
CREATE TABLE IF NOT EXISTS system_settings (
    setting_key           VARCHAR(64)  PRIMARY KEY,
    setting_value         TEXT         NOT NULL,
    is_secret             TINYINT(1)   NOT NULL DEFAULT 0,
    updated_at            DATETIME     NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

SET FOREIGN_KEY_CHECKS = 1;
