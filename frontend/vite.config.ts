import { defineConfig, type Plugin } from 'vite';
import react from '@vitejs/plugin-react';

function demoApiMockPlugin(): Plugin {
  const nowSec = () => Math.floor(Date.now() / 1000);

  const branches = [
    {
      id: 1,
      branch_code: 'KDR-HQ',
      branch_name: 'Kediri Head Office',
      telegram_chat_id: '-1001987654321',
      telegram_topic_id: 12,
      total_hosts: 485,
      up_count: 472,
      warn_count: 8,
      crit_count: 3,
      down_count: 2,
      pending_or_conflict: 1,
    },
    {
      id: 2,
      branch_code: 'SBY-01',
      branch_name: 'Surabaya Distribusi Barat',
      telegram_chat_id: '-1001987654321',
      telegram_topic_id: 24,
      total_hosts: 620,
      up_count: 604,
      warn_count: 11,
      crit_count: 4,
      down_count: 1,
      pending_or_conflict: 0,
    },
    {
      id: 3,
      branch_code: 'MLG-02',
      branch_name: 'Malang Depo Selatan',
      telegram_chat_id: '-1001987654321',
      telegram_topic_id: 38,
      total_hosts: 410,
      up_count: 405,
      warn_count: 4,
      crit_count: 1,
      down_count: 0,
      pending_or_conflict: 0,
    },
    {
      id: 4,
      branch_code: 'JKT-01',
      branch_name: 'Jakarta DC & Gudang Timur',
      telegram_chat_id: '-1001987654321',
      telegram_topic_id: 51,
      total_hosts: 790,
      up_count: 774,
      warn_count: 10,
      crit_count: 4,
      down_count: 2,
      pending_or_conflict: 1,
    },
    {
      id: 5,
      branch_code: 'SMG-01',
      branch_name: 'Semarang Hub Tengah',
      telegram_chat_id: '-1001987654321',
      telegram_topic_id: 63,
      total_hosts: 380,
      up_count: 376,
      warn_count: 3,
      crit_count: 1,
      down_count: 0,
      pending_or_conflict: 0,
    },
    {
      id: 6,
      branch_code: 'DPS-01',
      branch_name: 'Denpasar Operasional',
      telegram_chat_id: '-1001987654321',
      telegram_topic_id: 79,
      total_hosts: 315,
      up_count: 309,
      warn_count: 4,
      crit_count: 1,
      down_count: 1,
      pending_or_conflict: 0,
    },
  ];

  const makeServices = (kind: string) => {
    const t = nowSec();
    if (kind === 'ANDROID_PDA') {
      return [
        {
          service_name: 'Health_Battery',
          tier: 'FAST',
          state: 1,
          is_acknowledged: false,
          summary: 'Status : WARNING | Level : 14% (Discharging) | Real Health : 94.3% | Cycles : 39 | Temp : 35.5 C | Voltage : 4.18 V',
          metrics: 'battery_level=14%|battery_health=94.3%',
          last_check_epoch: t - 18,
          last_state_change_epoch: t - 320,
        },
        {
          service_name: 'RAM_Usage',
          tier: 'FAST',
          state: 0,
          is_acknowledged: false,
          summary: 'Status : OK | Used : 47% (1.73 GB) | Free : 1.99 GB | Total : 3.71 GB',
          metrics: 'ram_used=47%',
          last_check_epoch: t - 18,
          last_state_change_epoch: t - 8600,
        },
        {
          service_name: 'Storage_Usage',
          tier: 'MEDIUM',
          state: 0,
          is_acknowledged: false,
          summary: 'Status : OK | Used : 11% (5.18 GB) | Free : 42.09 GB | Total : 47.27 GB',
          metrics: 'disk_used=11%',
          last_check_epoch: t - 42,
          last_state_change_epoch: t - 86400,
        },
        {
          service_name: 'WiFi_Status',
          tier: 'FAST',
          state: 0,
          is_acknowledged: false,
          summary: 'Status : OK | SSID : BCP-WMS | AP BSSID : D0:21:F9:5F:53:B8 (UniFi AP) | Band : 5GHz Ch 153 | Signal : -58 dBm | Speed : 300 Mbps',
          metrics: 'wifi_rssi=-58dBm',
          last_check_epoch: t - 18,
          last_state_change_epoch: t - 4200,
        },
        {
          service_name: 'Ping_8.8.8.8',
          tier: 'PROXY',
          state: 0,
          is_acknowledged: false,
          summary: 'Status : OK | Target : 8.8.8.8 | 1h Summary (60x4=240 pkt) | Loss : 0.0% | Avg RTT : 28.9 ms',
          metrics: 'rta=28.9ms|pl=0%',
          last_check_epoch: t - 25,
          last_state_change_epoch: t - 7200,
        },
        {
          service_name: 'Ping_Gateway',
          tier: 'PROXY',
          state: 0,
          is_acknowledged: false,
          summary: 'Status : OK | Target : 192.168.55.1 (Gateway Cabang) | Loss : 0.0% | Avg RTT : 4.2 ms',
          metrics: 'rta=4.2ms|pl=0%',
          last_check_epoch: t - 25,
          last_state_change_epoch: t - 7200,
        },
      ];
    }

    return [
      {
        service_name: 'CPU_Info',
        tier: 'FAST',
        state: 0,
        is_acknowledged: false,
        summary: 'Spesifikasi : Intel Core i3-14100 | Clock Speed : 3.5Ghz | Core/Thread : 4/8 | CPU Load : 7% | CPU Temp : 28 C',
        metrics: 'cpu_util=7.0%|cpu_temp=28',
        last_check_epoch: t - 24,
        last_state_change_epoch: t - 14000,
      },
      {
        service_name: 'FAN_Health',
        tier: 'FAST',
        state: 0,
        is_acknowledged: false,
        summary: 'Status : OK - FAN Speed: 1122 rpm | CPU Temp: 28C - Kondisi: Low / Silent (Normal)',
        metrics: 'fan_rpm=1122',
        last_check_epoch: t - 24,
        last_state_change_epoch: t - 14000,
      },
      {
        service_name: 'RAM_Usage',
        tier: 'FAST',
        state: 0,
        is_acknowledged: false,
        summary: 'Status : OK | Used: 55% | Used Space: 8.72 GB | Free: 6.97 GB | Total: 15.69 GB',
        metrics: 'ram_used=55%',
        last_check_epoch: t - 24,
        last_state_change_epoch: t - 14000,
      },
      {
        service_name: 'Health_RAM',
        tier: 'SLOW',
        state: 0,
        is_acknowledged: false,
        summary: 'Status : OK | Result: Passed | Used Slots: 2/4 (2 Empty) | Active Modules: [8GiB,8GiB]',
        metrics: '-',
        last_check_epoch: t - 3600,
        last_state_change_epoch: t - 86400,
      },
      {
        service_name: 'Info_Network_Ethernet',
        tier: 'FAST',
        state: 0,
        is_acknowledged: false,
        summary: 'OK - IP Address: 192.168.55.169 | Total Download: 6.18 GB | Total Upload: 6.08 GB | RX: 1.18 KB/s',
        metrics: 'net_rx=1.18KB',
        last_check_epoch: t - 24,
        last_state_change_epoch: t - 12000,
      },
      {
        service_name: 'Info_OS',
        tier: 'SLOW',
        state: 0,
        is_acknowledged: false,
        summary: 'OK - OS: Microsoft Windows 11 Pro | Build: 26200 | License: Licensed | Key: 3V66T',
        metrics: '-',
        last_check_epoch: t - 1200,
        last_state_change_epoch: t - 86400,
      },
      {
        service_name: 'Info_Office',
        tier: 'SLOW',
        state: 0,
        is_acknowledged: false,
        summary: 'OK - Product: Microsoft Excel 2024 LTSC LICENSED (Key: HCJB7)',
        metrics: '-',
        last_check_epoch: t - 1200,
        last_state_change_epoch: t - 86400,
      },
      {
        service_name: 'Remote_Access_ID',
        tier: 'SLOW',
        state: 0,
        is_acknowledged: false,
        summary: 'OK: AnyDesk: 1372251148 | RustDesk: 1250104221 | AnyViewer: Not Installed',
        metrics: '-',
        last_check_epoch: t - 1200,
        last_state_change_epoch: t - 86400,
      },
      {
        service_name: 'Storage_Health_NVMe',
        tier: 'SLOW',
        state: 0,
        is_acknowledged: false,
        summary: 'Status : OK | Drive: NVMe | Merk: V-GEN09MAP25MST512MTNV | Kesehatan: 99% | Suhu: 36 C | SMART: PASSED',
        metrics: 'disk_health=99%',
        last_check_epoch: t - 900,
        last_state_change_epoch: t - 86400,
      },
      {
        service_name: 'Storage_Health_HDD',
        tier: 'SLOW',
        state: kind === 'WINDOWS_CRIT' ? 2 : 0,
        is_acknowledged: false,
        summary:
          kind === 'WINDOWS_CRIT'
            ? 'Status : CRITICAL | Drive: HDD | Merk: WDC WD5000AUDX | WARNING (14 Bad Sector) | Prediksi: Segera Ganti Disk'
            : 'Status : OK | Drive: HDD | Merk: ST1000LM024 | Kesehatan: 100% (0 Bad Sector) | Suhu: 29 C | SMART: PASSED',
        metrics: kind === 'WINDOWS_CRIT' ? 'bad_sectors=14' : 'disk_health=100%',
        last_check_epoch: t - 900,
        last_state_change_epoch: t - 1800,
      },
      {
        service_name: 'Storage_Usage_C',
        tier: 'MEDIUM',
        state: 0,
        is_acknowledged: false,
        summary: 'Status : OK | Partition: C: (Local Disk) | Used: 36% | Free: 154.49 GB | Total: 243.27 GB',
        metrics: 'disk_used=36%',
        last_check_epoch: t - 120,
        last_state_change_epoch: t - 40000,
      },
      {
        service_name: 'Ping_Gateway',
        tier: 'PROXY',
        state: 0,
        is_acknowledged: false,
        summary: 'Status : OK | Target : 192.168.55.1 | Loss : 0.0% | Avg RTT : 0.5 ms',
        metrics: 'rta=0.5ms',
        last_check_epoch: t - 24,
        last_state_change_epoch: t - 12000,
      },
    ];
  };

  const hostServicesMap: Record<number, ReturnType<typeof makeServices>> = {
    101: makeServices('ANDROID_PDA'),
    102: makeServices('WINDOWS_CRIT'),
    103: makeServices('LINUX'),
    104: [
      {
        service_name: '_HOST_',
        tier: 'PROXY',
        state: 2,
        is_acknowledged: false,
        summary: 'Host MIKROTIK-GW-SBY01 (192.168.146.1) OFFLINE / STALE (245 detik tidak merespons)',
        metrics: 'rta=0ms|pl=100%',
        last_check_epoch: nowSec() - 245,
        last_state_change_epoch: nowSec() - 245,
      },
    ],
    105: makeServices('LINUX'),
  };

  const hosts = [
    {
      host_id: 101,
      host_uuid: 'and-84f2c910a8b34e21',
      branch_code: 'KDR-HQ',
      hostname: 'PDA-04-SUPRIADI',
      ip_address: '192.168.108.164',
      host_type: 'ANDROID',
      status: 'WARN',
      approval_status: 'APPROVED',
      is_in_downtime: false,
      owner_name: 'Supriadi (Checker Gudang)',
      owner_wa: '6281234567890',
      owner_email: 'supriadi@bcp.co.id',
      user_alert_services: 'Health_Battery,Storage_Usage,WiFi_Status',
      last_seen_epoch: nowSec() - 18,
    },
    {
      host_id: 102,
      host_uuid: 'win-6d2f8110-4b2c-11ee',
      branch_code: 'SBY-01',
      hostname: 'BCPKDR-CSRPHIL-GHIFARY',
      ip_address: '192.168.55.168',
      host_type: 'WINDOWS',
      status: 'CRIT',
      approval_status: 'APPROVED',
      is_in_downtime: false,
      owner_name: 'Ghifary (CSR Surabaya)',
      owner_wa: '6285678901234',
      owner_email: 'ghifary@bcp.co.id',
      user_alert_services: 'Storage_Usage,Storage_Health',
      last_seen_epoch: nowSec() - 24,
    },
    {
      host_id: 103,
      host_uuid: 'lnx-2b4e90128f44a001',
      branch_code: 'KDR-HQ',
      hostname: 'BCPKDR-ADMPROG-MITHA',
      ip_address: '192.168.55.169',
      host_type: 'WINDOWS',
      status: 'UP',
      approval_status: 'APPROVED',
      is_in_downtime: false,
      owner_name: 'Mitha (Admin Prog)',
      owner_wa: '6281122334455',
      owner_email: 'mitha@bcp.co.id',
      user_alert_services: 'Storage_Usage,Health_RAM',
      last_seen_epoch: nowSec() - 12,
    },
    {
      host_id: 104,
      host_uuid: 'prx-mt-sby-01',
      branch_code: 'SBY-01',
      hostname: 'MIKROTIK-GW-SBY01',
      ip_address: '192.168.146.1',
      host_type: 'ROUTER',
      status: 'DOWN',
      approval_status: 'APPROVED',
      is_in_downtime: false,
      owner_name: null,
      owner_wa: null,
      owner_email: null,
      user_alert_services: 'Printer_Paper',
      last_seen_epoch: nowSec() - 245,
    },
    {
      host_id: 105,
      host_uuid: 'win-clone-99aabbccdd',
      branch_code: 'JKT-01',
      hostname: 'JKT-KASIR-09',
      ip_address: '10.20.14.88',
      host_type: 'WINDOWS',
      status: 'PENDING',
      approval_status: 'CONFLICT',
      is_in_downtime: false,
      owner_name: 'Rina (Kasir Depo)',
      owner_wa: '6281998877665',
      owner_email: null,
      user_alert_services: 'Storage_Usage',
      last_seen_epoch: nowSec() - 9,
    },
  ];

  const users = [
    {
      id: 1,
      username: 'admin',
      full_name: 'Dodik (IT Infrastructure Lead)',
      role: 'SUPERADMIN',
      assigned_branch_id: null,
      branch_code: null,
      is_active: true,
    },
    {
      id: 2,
      username: 'it.surabaya',
      full_name: 'Rizky Pratama (IT Ops Surabaya)',
      role: 'IT_BRANCH',
      assigned_branch_id: 2,
      branch_code: 'SBY-01',
      is_active: true,
    },
    {
      id: 3,
      username: 'audit.ho',
      full_name: 'Tim Audit Internal Pusat',
      role: 'VIEWER',
      assigned_branch_id: null,
      branch_code: null,
      is_active: true,
    },
  ];

  const incidents = [
    {
      id: 501,
      hostname: 'MIKROTIK-GW-SBY01',
      branch_code: 'SBY-01',
      service_name: '_HOST_',
      from_state: 0,
      to_state: 2,
      summary: 'Host MIKROTIK-GW-SBY01 (192.168.146.1) OFFLINE / STALE (>180s)',
      created_at: '2026-11-16 09:27:15',
    },
    {
      id: 500,
      hostname: 'BCPKDR-CSRPHIL-GHIFARY',
      branch_code: 'SBY-01',
      service_name: 'Storage_Health_HDD',
      from_state: 1,
      to_state: 2,
      summary: 'WARNING -> CRITICAL (14 Bad Sector pada WDC WD5000AUDX)',
      created_at: '2026-11-16 09:14:02',
    },
    {
      id: 499,
      hostname: 'PDA-04-SUPRIADI',
      branch_code: 'KDR-HQ',
      service_name: 'Health_Battery',
      from_state: 0,
      to_state: 1,
      summary: 'Level baterai PDA turun menjadi 14% (Discharging)',
      created_at: '2026-11-16 09:05:44',
    },
  ];

  const outbox = [
    {
      id: 901,
      hostname: 'MIKROTIK-GW-SBY01',
      service_name: '_HOST_',
      target_role: 'IT_TELEGRAM',
      destination: '-1001987654321:24',
      event_type: 'PROBLEM',
      delivery_status: 'SENT',
      last_error: null,
      created_at: '2026-11-16 09:27:16',
    },
    {
      id: 900,
      hostname: 'PDA-04-SUPRIADI',
      service_name: 'Health_Battery',
      target_role: 'OWNER_WA',
      destination: '6281234567890',
      event_type: 'PROBLEM',
      delivery_status: 'SENT',
      last_error: null,
      created_at: '2026-11-16 09:06:10',
    },
  ];

  const inventory = [
    {
      host_id: 101,
      branch_code: 'KDR-HQ',
      hostname: 'PDA-04-SUPRIADI',
      ip_address: '192.168.108.164',
      host_type: 'ANDROID_PDA',
      os_name: 'Android 11 (Newland NLS-MT93)',
      os_license_status: 'Enterprise MDM',
      office_product: '-',
      cpu_model: 'MediaTek Helio P60 (8 Cores)',
      ram_total_gb: 4.0,
      disk_primary_model: 'eMMC 64GB',
      disk_primary_type: 'eMMC/UFS',
      disk_health_pct: 96,
      battery_health_pct: 94.3,
      battery_cycles: 39,
      remote_anydesk_id: '781029384',
      remote_rustdesk_id: '-',
    },
    {
      host_id: 102,
      branch_code: 'SBY-01',
      hostname: 'BCPKDR-CSRPHIL-GHIFARY',
      ip_address: '192.168.55.168',
      host_type: 'WINDOWS',
      os_name: 'Microsoft Windows 11 Pro (25H2)',
      os_license_status: 'Licensed (Key: KQBQB)',
      office_product: 'Microsoft Excel 2024 LTSC',
      cpu_model: 'Intel Core i3-14100 (4C/8T)',
      ram_total_gb: 16.0,
      disk_primary_model: 'V-GEN09MAP25MST512MTNV',
      disk_primary_type: 'NVMe',
      disk_health_pct: 98,
      battery_health_pct: null,
      battery_cycles: null,
      remote_anydesk_id: '161827264',
      remote_rustdesk_id: '38046685',
    },
    {
      host_id: 103,
      branch_code: 'KDR-HQ',
      hostname: 'BCPKDR-ADMPROG-MITHA',
      ip_address: '192.168.55.169',
      host_type: 'WINDOWS',
      os_name: 'Microsoft Windows 11 Pro (25H2)',
      os_license_status: 'Licensed (Key: 3V66T)',
      office_product: 'Microsoft Excel 2024 LTSC',
      cpu_model: 'Intel Core i3-14100 (4C/8T)',
      ram_total_gb: 16.0,
      disk_primary_model: 'V-GEN09MAP25MST512MTNV',
      disk_primary_type: 'NVMe',
      disk_health_pct: 99,
      battery_health_pct: null,
      battery_cycles: null,
      remote_anydesk_id: '1372251148',
      remote_rustdesk_id: '1250104221',
    },
  ];

  const deviceTypes = [
    { code: 'WINDOWS', label: 'Windows', is_builtin: true },
    { code: 'LINUX', label: 'Linux', is_builtin: true },
    { code: 'ANDROID', label: 'Android', is_builtin: true },
    { code: 'ROUTER', label: 'Router', is_builtin: true },
    { code: 'SWITCH', label: 'Switch', is_builtin: true },
    { code: 'FIREWALL', label: 'Firewall', is_builtin: true },
    { code: 'PRINTER', label: 'Printer', is_builtin: true },
  ];

  const runtimeSettings = {
    rest_api_public_url: 'http://192.168.55.112:8080',
    default_enrollment_token: 'enroll-kdr-hq-secret-token-2026',
    batch_flush_interval_sec: 180,
    host_stale_threshold_sec: 180,
    flapping_threshold_cycles: 2,
    telegram_default_chat_id: '-1001987654321',
    has_telegram_bot_token: true,
    wa_webhook_url: 'https://wa-gateway.internal/api/send',
    has_wa_webhook_token: true,
    smtp_host: 'smtp.office365.com',
    smtp_port: 587,
    smtp_username: 'monitoring@company.co.id',
    smtp_from: 'Monitoring IT Ops <monitoring@company.co.id>',
    smtp_use_tls: true,
    has_smtp_password: false,
  };

  const enrichHost = (h: (typeof hosts)[number]) => {
    const svcs = hostServicesMap[h.host_id] || [];
    return {
      ...h,
      ok_services: svcs.filter((s) => s.state === 0).length,
      warn_services: svcs.filter((s) => s.state === 1).length,
      crit_services: svcs.filter((s) => s.state >= 2).length,
      total_services: svcs.length,
      problem_services: svcs
        .filter((s) => s.state > 0)
        .map((s) => `${s.service_name}: ${s.summary}`),
    };
  };

  return {
    name: 'cmk-demo-api-mock',
    configureServer(server) {
      if (process.env.VITE_DEMO_MODE !== 'true') return;

      server.middlewares.use((req, res, next) => {
        const url = req.url || '';
        if (!url.startsWith('/api/v1/')) return next();

        const sendJson = (data: unknown, status = 200) => {
          res.statusCode = status;
          res.setHeader('Content-Type', 'application/json');
          res.end(JSON.stringify({ success: true, data, error: null }));
        };

        const sendCsv = (filename: string, content: string) => {
          res.statusCode = 200;
          res.setHeader('Content-Type', 'text/csv; charset=utf-8');
          res.setHeader('Content-Disposition', `attachment; filename="${filename}"`);
          res.end(content);
        };

        if (req.method === 'GET' && url.startsWith('/api/v1/branches/summary')) {
          return sendJson(branches);
        }

        if (req.method === 'GET' && url === '/api/v1/users') {
          return sendJson(users);
        }

        if (req.method === 'GET' && url === '/api/v1/incidents') {
          return sendJson(incidents);
        }

        if (req.method === 'GET' && url === '/api/v1/outbox') {
          return sendJson(outbox);
        }

        if (req.method === 'GET' && url === '/api/v1/inventory') {
          return sendJson(inventory);
        }

        if (req.method === 'GET' && url === '/api/v1/device-types') {
          return sendJson(deviceTypes);
        }

        if (req.method === 'GET' && url === '/api/v1/settings') {
          return sendJson(runtimeSettings);
        }

        if (req.method === 'GET' && url === '/api/v1/downloads') {
          return sendJson([
            {
              id: 'windows-x64',
              os_label: 'Windows 10 / 11 / Server',
              arch: 'x86_64 (.exe)',
              filename: 'bms-agent-windows-x86_64.exe',
              description: 'Binary agent ultra-ringan + installer Windows Service (MachineGuid UUID).',
              available_locally: true,
              size_bytes: 2_980_000,
              direct_download_url: '/api/v1/downloads/file/bms-agent-windows-x86_64.exe',
              github_release_url: 'https://github.com/Dot-Dodik/cmkagent-rust/releases/latest',
            },
            {
              id: 'linux-x64',
              os_label: 'Linux Desktop / Server',
              arch: 'x86_64 (ELF)',
              filename: 'bms-agent-linux-x86_64',
              description: 'Agen ringan untuk Ubuntu/Mint/Debian lawas (< 4 MB, RAM < 8 MB, /proc + /sys).',
              available_locally: true,
              size_bytes: 2_640_000,
              direct_download_url: '/api/v1/downloads/file/bms-agent-linux-x86_64',
              github_release_url: 'https://github.com/Dot-Dodik/cmkagent-rust/releases/latest',
            },
            {
              id: 'linux-arm64',
              os_label: 'Raspberry Pi / ARM64',
              arch: 'aarch64 (ELF)',
              filename: 'bms-agent-linux-aarch64',
              description: 'Agen khusus ARM64 / Raspberry Pi & mini server cabang.',
              available_locally: true,
              size_bytes: 2_510_000,
              direct_download_url: '/api/v1/downloads/file/bms-agent-linux-aarch64',
              github_release_url: 'https://github.com/Dot-Dodik/cmkagent-rust/releases/latest',
            },
            {
              id: 'android-apk',
              os_label: 'Android PDA & Tablet',
              arch: 'Universal (.apk)',
              filename: 'bms-agent-android-v2.0.0.apk',
              description: 'Aplikasi Android dengan ForegroundService, baterai real health, WiFi roaming & ping.',
              available_locally: true,
              size_bytes: 4_120_000,
              direct_download_url: '/api/v1/downloads/file/bms-agent-android-v2.0.0.apk',
              github_release_url: 'https://github.com/Dot-Dodik/cmkagent-rust/releases/latest',
            },
            {
              id: 'branch-proxy',
              os_label: 'Linux Branch Proxy Poller',
              arch: 'x86_64 (Proxy)',
              filename: 'bms-agent-proxy-linux-x86_64',
              description: 'Agen kolektor cabang (1 per cabang) untuk polling Router, Switch, Firewall, & Printer.',
              available_locally: true,
              size_bytes: 3_350_000,
              direct_download_url: '/api/v1/downloads/file/bms-agent-proxy-linux-x86_64',
              github_release_url: 'https://github.com/Dot-Dodik/cmkagent-rust/releases/latest',
            },
          ]);
        }

        if (req.method === 'GET' && url.startsWith('/api/v1/downloads/install-linux.sh')) {
          res.statusCode = 200;
          res.setHeader('Content-Type', 'text/x-shellscript; charset=utf-8');
          res.setHeader('Content-Disposition', 'attachment; filename="install-bms-linux.sh"');
          res.end('#!/usr/bin/env bash\necho "BMS Linux Agent Installer"\n');
          return;
        }

        if (req.method === 'GET' && url.startsWith('/api/v1/downloads/install-windows.ps1')) {
          res.statusCode = 200;
          res.setHeader('Content-Type', 'text/plain; charset=utf-8');
          res.setHeader('Content-Disposition', 'attachment; filename="install-bms-windows.ps1"');
          res.end('# BMS Windows Agent Installer\nWrite-Host "BMS Windows Installer"\n');
          return;
        }

        if (req.method === 'GET' && url === '/api/v1/reports/hosts.csv') {
          const lines = hosts
            .map(enrichHost)
            .map((h) => `${h.branch_code},${h.hostname},${h.ip_address},${h.host_type},${h.status},${h.ok_services},${h.warn_services},${h.crit_services},"${h.owner_name || '-'}"`)
            .join('\n');
          return sendCsv(
            'cmk_hosts_status_report.csv',
            'Branch,Hostname,IP_Address,Host_Type,Status,OK,WARN,CRIT,Owner\n' + lines + '\n'
          );
        }

        if (req.method === 'GET' && url === '/api/v1/reports/incidents.csv') {
          const lines = incidents
            .map((i) => `${i.created_at},${i.branch_code},${i.hostname},${i.service_name},${i.from_state},${i.to_state},"${i.summary}"`)
            .join('\n');
          return sendCsv(
            'cmk_incidents_sla_report.csv',
            'Timestamp,Branch,Hostname,Service,From,To,Summary\n' + lines + '\n'
          );
        }

        if (req.method === 'GET' && url === '/api/v1/inventory/export.csv') {
          const lines = inventory
            .map((r) => `${r.branch_code},${r.hostname},${r.ip_address},${r.host_type},"${r.os_name}","${r.office_product}",${r.disk_health_pct ?? '-'},${r.battery_health_pct ?? '-'}`)
            .join('\n');
          return sendCsv(
            'cmk_asset_inventory_export.csv',
            'Branch,Hostname,IP,Type,OS,Office,Disk_Health,Battery_Health\n' + lines + '\n'
          );
        }

        const svcCsvMatch = url.match(/^\/api\/v1\/hosts\/(\d+)\/services\.csv$/);
        if (req.method === 'GET' && svcCsvMatch) {
          const hid = Number(svcCsvMatch[1]);
          const h = hosts.find((x) => x.host_id === hid);
          const svcs = hostServicesMap[hid] || [];
          const lines = svcs
            .map((s) => `${h?.hostname || hid},${s.service_name},${s.tier},${s.state},"${s.metrics}","${s.summary}"`)
            .join('\n');
          return sendCsv(
            `host_${hid}_services.csv`,
            'Hostname,Service,Tier,State,Metrics,Summary\n' + lines + '\n'
          );
        }

        const svcMatch = url.match(/^\/api\/v1\/hosts\/(\d+)\/services$/);
        if (req.method === 'GET' && svcMatch) {
          const hid = Number(svcMatch[1]);
          return sendJson(hostServicesMap[hid] || makeServices('LINUX'));
        }

        if (req.method === 'GET' && url.startsWith('/api/v1/hosts')) {
          const parsedUrl = new URL(url, 'http://localhost');
          const bc = parsedUrl.searchParams.get('branch_code') || '';
          const ht = parsedUrl.searchParams.get('host_type') || '';
          const search = (parsedUrl.searchParams.get('search') || '').toLowerCase();
          const actionOnly = parsedUrl.searchParams.get('action_needed') === 'true';
          const approvalFilter = parsedUrl.searchParams.get('approval_filter') || '';

          const filtered = hosts
            .filter((h) => {
              if (bc && h.branch_code !== bc) return false;
              if (ht && h.host_type !== ht) return false;
              if (approvalFilter && h.approval_status !== approvalFilter) return false;
              if (actionOnly && h.status === 'UP' && h.approval_status === 'APPROVED') return false;
              if (
                search &&
                !h.hostname.toLowerCase().includes(search) &&
                !h.ip_address.toLowerCase().includes(search) &&
                !h.branch_code.toLowerCase().includes(search) &&
                !h.host_uuid.toLowerCase().includes(search)
              ) {
                return false;
              }
              return true;
            })
            .map(enrichHost);

          return sendJson({
            page: 1,
            per_page: 50,
            total_items: filtered.length,
            total_pages: 1,
            items: filtered,
          });
        }

        let body = '';
        req.on('data', (chunk) => {
          body += chunk;
        });
        req.on('end', () => {
          try {
            const payload = JSON.parse(body || '{}');

            if (req.method === 'POST' && url === '/api/v1/device-types') {
              const code = String(payload.code || '').toUpperCase();
              const label = String(payload.label || code);
              const dt = { code, label, is_builtin: false };
              if (code && !deviceTypes.some((d) => d.code === code)) {
                deviceTypes.push(dt);
              }
              return sendJson(dt, 201);
            }

            if (req.method === 'PUT' && url === '/api/v1/settings') {
              Object.assign(runtimeSettings, {
                rest_api_public_url: payload.rest_api_public_url ?? runtimeSettings.rest_api_public_url,
                default_enrollment_token:
                  payload.default_enrollment_token ?? runtimeSettings.default_enrollment_token,
                batch_flush_interval_sec:
                  payload.batch_flush_interval_sec ?? runtimeSettings.batch_flush_interval_sec,
                host_stale_threshold_sec:
                  payload.host_stale_threshold_sec ?? runtimeSettings.host_stale_threshold_sec,
                flapping_threshold_cycles:
                  payload.flapping_threshold_cycles ?? runtimeSettings.flapping_threshold_cycles,
                telegram_default_chat_id:
                  payload.telegram_default_chat_id ?? runtimeSettings.telegram_default_chat_id,
                wa_webhook_url: payload.wa_webhook_url ?? runtimeSettings.wa_webhook_url,
                smtp_host: payload.smtp_host ?? runtimeSettings.smtp_host,
                smtp_port: payload.smtp_port ?? runtimeSettings.smtp_port,
                smtp_username: payload.smtp_username ?? runtimeSettings.smtp_username,
                smtp_from: payload.smtp_from ?? runtimeSettings.smtp_from,
                smtp_use_tls: payload.smtp_use_tls ?? runtimeSettings.smtp_use_tls,
              });
              if (payload.smtp_password) runtimeSettings.has_smtp_password = true;
              if (payload.telegram_bot_token) runtimeSettings.has_telegram_bot_token = true;
              if (payload.wa_webhook_token) runtimeSettings.has_wa_webhook_token = true;
              return sendJson('Settings saved (demo)');
            }

            if (req.method === 'POST' && url === '/api/v1/settings/test-email') {
              return sendJson(
                `Simulasi email tes berhasil dikirim dari ${runtimeSettings.smtp_from} ke ${payload.to_email}`
              );
            }

            if (req.method === 'POST' && url === '/api/v1/hosts') {
              const newId = hosts.length + 101;
              hosts.unshift({
                host_id: newId,
                host_uuid: `manual-${Date.now()}`,
                branch_code: payload.branch_code || 'KDR-HQ',
                hostname: payload.hostname || 'NEW-HOST',
                ip_address: payload.ip_address || '192.168.1.1',
                host_type: payload.host_type || 'WINDOWS',
                status: 'UP',
                approval_status: 'APPROVED',
                is_in_downtime: false,
                owner_name: payload.owner_name ?? null,
                owner_wa: payload.owner_wa ?? null,
                owner_email: null,
                user_alert_services: 'Health_Battery,Storage_Usage,Printer_Paper',
                last_seen_epoch: nowSec(),
              });
              hostServicesMap[newId] = makeServices(payload.host_type || 'LINUX');
              return sendJson(newId, 201);
            }

            if (req.method === 'POST' && url === '/api/v1/branches') {
              const newId = branches.length + 1;
              branches.push({
                id: newId,
                branch_code: payload.branch_code,
                branch_name: payload.branch_name,
                telegram_chat_id: payload.telegram_chat_id ?? null,
                telegram_topic_id: payload.telegram_topic_id ?? null,
                total_hosts: 0,
                up_count: 0,
                warn_count: 0,
                crit_count: 0,
                down_count: 0,
                pending_or_conflict: 0,
              });
              return sendJson('Branch added', 201);
            }

            if (req.method === 'POST' && url === '/api/v1/users') {
              users.unshift({
                id: users.length + 1,
                username: payload.username,
                full_name: payload.full_name,
                role: payload.role || 'VIEWER',
                assigned_branch_id: payload.assigned_branch_id ?? null,
                branch_code:
                  branches.find((b) => b.id === payload.assigned_branch_id)?.branch_code || null,
                is_active: true,
              });
              return sendJson('User created', 201);
            }

            const userPatch = url.match(/^\/api\/v1\/users\/(\d+)$/);
            if (req.method === 'PATCH' && userPatch) {
              const uid = Number(userPatch[1]);
              const u = users.find((x) => x.id === uid);
              if (u && typeof payload.is_active === 'boolean') u.is_active = payload.is_active;
              return sendJson('User updated');
            }

            const ackMatch = url.match(/^\/api\/v1\/hosts\/(\d+)\/services\/(.+)\/ack$/);
            if (req.method === 'POST' && ackMatch) {
              const hid = Number(ackMatch[1]);
              const sname = decodeURIComponent(ackMatch[2]);
              const svc = (hostServicesMap[hid] || []).find((s) => s.service_name === sname);
              if (svc) svc.is_acknowledged = Boolean(payload.acknowledged);
              return sendJson('ACK updated');
            }

            const dtMatch = url.match(/^\/api\/v1\/hosts\/(\d+)\/downtime$/);
            if (req.method === 'POST' && dtMatch) {
              const hid = Number(dtMatch[1]);
              const h = hosts.find((x) => x.host_id === hid);
              if (h) h.is_in_downtime = Boolean(payload.in_downtime);
              return sendJson('Downtime updated');
            }

            const hostMatch = url.match(/^\/api\/v1\/hosts\/(\d+)\/metadata$/);
            if (req.method === 'PATCH' && hostMatch) {
              const hid = Number(hostMatch[1]);
              const target = hosts.find((h) => h.host_id === hid);
              if (target) {
                target.owner_name = payload.owner_name ?? null;
                target.owner_wa = payload.owner_wa ?? null;
                target.owner_email = payload.owner_email ?? null;
                target.user_alert_services =
                  payload.user_alert_services || target.user_alert_services;
                target.approval_status = payload.approval_status || target.approval_status;
              }
              return sendJson('Metadata updated');
            }

            const branchMatch = url.match(/^\/api\/v1\/branches\/(\d+)$/);
            if (req.method === 'PATCH' && branchMatch) {
              const bid = Number(branchMatch[1]);
              const b = branches.find((x) => x.id === bid);
              if (b) {
                b.telegram_chat_id = payload.telegram_chat_id ?? null;
                b.telegram_topic_id = payload.telegram_topic_id ?? null;
              }
              return sendJson('Branch updated');
            }
          } catch {
            return sendJson('Bad payload', 400);
          }
          return sendJson('OK');
        });
      });
    },
  };
}

export default defineConfig({
  plugins: [react(), demoApiMockPlugin()],
  server: {
    port: 5173,
    proxy:
      process.env.VITE_DEMO_MODE === 'true'
        ? undefined
        : {
            '/api': {
              target: 'http://127.0.0.1:8080',
              changeOrigin: true,
            },
          },
  },
});
