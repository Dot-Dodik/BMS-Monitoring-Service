export interface ApiEnvelope<T> {
  success: boolean;
  data?: T;
  error?: string;
}

export interface BranchSummaryCard {
  id: number;
  branch_code: string;
  branch_name: string;
  telegram_chat_id: string | null;
  telegram_topic_id: number | null;
  total_hosts: number;
  up_count: number;
  warn_count: number;
  crit_count: number;
  down_count: number;
  pending_or_conflict: number;
}

export interface HostRowDto {
  host_id: number;
  host_uuid: string;
  branch_code: string;
  hostname: string;
  ip_address: string;
  host_type: string;
  status: 'UP' | 'WARN' | 'CRIT' | 'DOWN' | 'PENDING';
  approval_status: 'PENDING' | 'APPROVED' | 'CONFLICT' | 'REVOKED';
  is_in_downtime?: boolean;
  ok_services?: number;
  warn_services?: number;
  crit_services?: number;
  total_services?: number;
  owner_name: string | null;
  owner_wa: string | null;
  owner_email: string | null;
  user_alert_services: string;
  last_seen_epoch: number;
  problem_services: string[];
}

export interface PaginatedHostsResponse {
  page: number;
  per_page: number;
  total_items: number;
  total_pages: number;
  items: HostRowDto[];
}

export interface ServiceRowDto {
  service_name: string;
  tier: string;
  state: number; // 0=OK, 1=WARN, 2=CRIT, 3=UNKNOWN
  is_acknowledged: boolean;
  summary: string;
  metrics: string;
  last_check_epoch: number;
  last_state_change_epoch: number;
}

export interface InventoryItemDto {
  host_id: number;
  branch_code: string;
  hostname: string;
  ip_address: string;
  host_type: string;
  os_name: string | null;
  os_license_status: string | null;
  office_product: string | null;
  cpu_model: string | null;
  ram_total_gb: number | null;
  disk_primary_model: string | null;
  disk_primary_type: string | null;
  disk_health_pct: number | null;
  battery_health_pct: number | null;
  battery_cycles: number | null;
  remote_anydesk_id: string | null;
  remote_rustdesk_id: string | null;
}

export interface UserRowDto {
  id: number;
  username: string;
  full_name: string;
  role: 'SUPERADMIN' | 'IT_BRANCH' | 'VIEWER';
  assigned_branch_id: number | null;
  branch_code: string | null;
  is_active: boolean;
}

export interface IncidentDto {
  id: number;
  hostname: string;
  branch_code: string;
  service_name: string;
  from_state: number;
  to_state: number;
  summary: string;
  created_at: string;
}

export interface OutboxDto {
  id: number;
  hostname: string;
  service_name: string;
  target_role: 'IT_TELEGRAM' | 'OWNER_WA' | 'OWNER_EMAIL';
  destination: string;
  event_type: 'PROBLEM' | 'RECOVERY';
  delivery_status: 'PENDING' | 'SENT' | 'FAILED';
  last_error: string | null;
  created_at: string;
}

export interface DeviceTypeDto {
  code: string;
  label: string;
  is_builtin: boolean;
}

export interface SettingsViewDto {
  rest_api_public_url: string;
  default_enrollment_token: string;
  batch_flush_interval_sec: number;
  host_stale_threshold_sec: number;
  flapping_threshold_cycles: number;
  telegram_default_chat_id: string;
  has_telegram_bot_token: boolean;
  wa_webhook_url: string;
  has_wa_webhook_token: boolean;
  smtp_host: string;
  smtp_port: number;
  smtp_username: string;
  smtp_from: string;
  smtp_use_tls: boolean;
  has_smtp_password: boolean;
}

export interface AgentPackageDto {
  id: string;
  os_label: string;
  arch: string;
  filename: string;
  description: string;
  available_locally: boolean;
  size_bytes: number;
  direct_download_url: string;
  github_release_url: string;
}

const API_BASE = import.meta.env.VITE_API_BASE || '';

export async function apiFetch<T>(path: string, init?: RequestInit): Promise<T> {
  const controller = new AbortController();
  const timer = setTimeout(() => controller.abort(), 10000);
  try {
    const res = await fetch(`${API_BASE}${path}`, {
      ...init,
      signal: controller.signal,
      headers: {
        'Content-Type': 'application/json',
        ...(init?.headers || {}),
      },
    });
    const json: ApiEnvelope<T> = await res.json();
    if (!res.ok || !json.success || json.data === undefined) {
      throw new Error(json.error || `HTTP ${res.status}`);
    }
    return json.data;
  } finally {
    clearTimeout(timer);
  }
}

export const reportUrls = {
  inventoryCsv: () => `${API_BASE}/api/v1/inventory/export.csv`,
  hostsCsv: () => `${API_BASE}/api/v1/reports/hosts.csv`,
  incidentsCsv: () => `${API_BASE}/api/v1/reports/incidents.csv`,
  hostServicesCsv: (hostId: number) => `${API_BASE}/api/v1/hosts/${hostId}/services.csv`,
  linuxInstaller: (branchCode: string) =>
    `${API_BASE}/api/v1/downloads/install-linux.sh?branch=${encodeURIComponent(branchCode)}`,
  windowsInstaller: (branchCode: string) =>
    `${API_BASE}/api/v1/downloads/install-windows.ps1?branch=${encodeURIComponent(branchCode)}`,
};
