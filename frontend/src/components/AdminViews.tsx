import { useEffect, useState } from 'react';
import {
  Bell,
  Building2,
  CheckCircle2,
  Copy,
  Download,
  ExternalLink,
  FileSpreadsheet,
  Globe,
  HardDriveDownload,
  Mail,
  Plus,
  Printer,
  Save,
  Send,
  Settings,
  Terminal,
  UserPlus,
  Users,
} from 'lucide-react';
import {
  AgentPackageDto,
  apiFetch,
  BranchSummaryCard,
  DeviceTypeDto,
  IncidentDto,
  OutboxDto,
  reportUrls,
  SettingsViewDto,
  UserRowDto,
} from '../api/client';

export function ReportsCenterView({
  onOpenInventory,
}: {
  onOpenInventory: () => void;
}) {
  return (
    <div className="space-y-6">
      <div>
        <h2 className="text-lg font-bold">Pusat Export Laporan &amp; Audit Infrastruktur</h2>
        <p className="text-xs text-slate-400">
          Unduh laporan kesiapan armada, inventaris hardware/lisensi, serta riwayat insiden lintas 3.000 perangkat dalam sekali klik.
        </p>
      </div>

      <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
        <ReportCard
          title="1. Laporan Status Seluruh Perangkat (All Hosts CSV)"
          desc="Ekspor status UP/WARN/CRIT/DOWN, jumlah service bermasalah per host, IP address, tipe perangkat, dan nama pemilik."
          href={reportUrls.hostsCsv()}
          badge="REAL-TIME RAM"
        />
        <ReportCard
          title="2. Laporan Inventaris Hardware, SSD/NVMe, Baterai PDA & Lisensi"
          desc="Data Slow Tier (6 jam): OS, Office license, model CPU/RAM, kesehatan SSD/NVMe %, cycle count baterai PDA, & ID AnyDesk/RustDesk."
          href={reportUrls.inventoryCsv()}
          badge="ASSET AUDIT"
          extraAction={
            <button
              onClick={onOpenInventory}
              className="px-3 py-1.5 rounded bg-slate-800 hover:bg-slate-700 text-xs font-medium"
            >
              Pratinjau Tabel
            </button>
          }
        />
        <ReportCard
          title="3. Laporan Riwayat Insiden & SLA Cabang (Incidents CSV)"
          desc="Catatan lengkap transisi status (OK <-> WARN/CRIT/DOWN) per cabang untuk audit uptime dan SLA ISP/perangkat."
          href={reportUrls.incidentsCsv()}
          badge="SLA HISTORY"
        />
        <div className="bg-[#11181c] border border-slate-800 rounded-xl p-5 flex flex-col justify-between space-y-4">
          <div>
            <span className="text-[11px] font-mono px-2 py-0.5 rounded bg-sky-950 text-sky-300 border border-sky-500/30">
              PRINT / PDF
            </span>
            <h3 className="text-base font-bold mt-2">4. Cetak Ringkasan Eksekutif (PDF / Print)</h3>
            <p className="text-xs text-slate-400 mt-1">
              Cetak tampilan dashboard atau simpan ke PDF melalui dialog print browser untuk laporan rapat manajemen IT Ops.
            </p>
          </div>
          <div>
            <button
              onClick={() => window.print()}
              className="px-4 py-2 rounded-lg bg-sky-600 hover:bg-sky-500 text-xs font-semibold inline-flex items-center gap-2"
            >
              <Printer className="w-4 h-4" /> Cetak / Simpan PDF
            </button>
          </div>
        </div>
      </div>
    </div>
  );
}

function ReportCard({
  title,
  desc,
  href,
  badge,
  extraAction,
}: {
  title: string;
  desc: string;
  href: string;
  badge: string;
  extraAction?: React.ReactNode;
}) {
  return (
    <div className="bg-[#11181c] border border-slate-800 rounded-xl p-5 flex flex-col justify-between space-y-4">
      <div>
        <span className="text-[11px] font-mono px-2 py-0.5 rounded bg-emerald-950 text-emerald-300 border border-emerald-500/30">
          {badge}
        </span>
        <h3 className="text-base font-bold mt-2 flex items-center gap-2">
          <FileSpreadsheet className="w-4 h-4 text-[#13d389] shrink-0" />
          {title}
        </h3>
        <p className="text-xs text-slate-400 mt-1">{desc}</p>
      </div>
      <div className="flex items-center gap-2">
        <a
          href={href}
          className="px-4 py-2 rounded-lg bg-[#13d389] hover:bg-[#10b977] text-slate-950 text-xs font-bold inline-flex items-center gap-2"
        >
          <Download className="w-4 h-4" /> Unduh CSV
        </a>
        {extraAction}
      </div>
    </div>
  );
}

export function UsersManagementView({ branches }: { branches: BranchSummaryCard[] }) {
  const [users, setUsers] = useState<UserRowDto[]>([]);
  const [showAdd, setShowAdd] = useState(false);
  const [username, setUsername] = useState('');
  const [fullName, setFullName] = useState('');
  const [password, setPassword] = useState('');
  const [role, setRole] = useState<'SUPERADMIN' | 'IT_BRANCH' | 'VIEWER'>('IT_BRANCH');
  const [branchId, setBranchId] = useState<string>('');
  const [err, setErr] = useState<string | null>(null);

  const load = async () => {
    const list = await apiFetch<UserRowDto[]>('/api/v1/users');
    setUsers(list);
  };

  useEffect(() => {
    load();
  }, []);

  const createUser = async () => {
    setErr(null);
    try {
      await apiFetch('/api/v1/users', {
        method: 'POST',
        body: JSON.stringify({
          username: username.trim(),
          full_name: fullName.trim(),
          password,
          role,
          assigned_branch_id: branchId ? Number(branchId) : null,
        }),
      });
      setUsername('');
      setFullName('');
      setPassword('');
      setShowAdd(false);
      await load();
    } catch (e: unknown) {
      setErr(e instanceof Error ? e.message : 'Gagal menambah user');
    }
  };

  const toggleActive = async (u: UserRowDto) => {
    await apiFetch(`/api/v1/users/${u.id}`, {
      method: 'PATCH',
      body: JSON.stringify({ is_active: !u.is_active }),
    });
    await load();
  };

  return (
    <div className="space-y-4">
      <div className="flex items-center justify-between">
        <div>
          <h2 className="text-lg font-bold flex items-center gap-2">
            <Users className="w-5 h-5 text-[#13d389]" /> Manajemen Pengguna Dashboard (RBAC)
          </h2>
          <p className="text-xs text-slate-400">
            Tambahkan akun operator IT Cabang, Superadmin Pusat, atau Viewer secara manual (password di-hash Argon2id).
          </p>
        </div>
        <button
          onClick={() => setShowAdd(true)}
          className="px-4 py-2 rounded-lg bg-[#13d389] hover:bg-[#10b977] text-slate-950 text-xs font-bold inline-flex items-center gap-1.5"
        >
          <UserPlus className="w-4 h-4" /> + Tambah User Manual
        </button>
      </div>

      <div className="bg-[#11181c] border border-slate-800 rounded-xl overflow-x-auto">
        <table className="w-full text-left text-xs">
          <thead className="bg-[#0e1418] text-slate-400 uppercase border-b border-slate-800">
            <tr>
              <th className="px-4 py-3">Username</th>
              <th className="px-4 py-3">Nama Lengkap</th>
              <th className="px-4 py-3">Role RBAC</th>
              <th className="px-4 py-3">Lingkup Cabang</th>
              <th className="px-4 py-3">Status</th>
              <th className="px-4 py-3 text-right">Aksi</th>
            </tr>
          </thead>
          <tbody className="divide-y divide-slate-800/70">
            {users.map((u) => (
              <tr key={u.id} className="hover:bg-[#1a252c]">
                <td className="px-4 py-3 font-mono font-semibold text-[#13d389]">{u.username}</td>
                <td className="px-4 py-3">{u.full_name}</td>
                <td className="px-4 py-3">
                  <span className="px-2 py-0.5 rounded bg-slate-800 border border-slate-700 font-mono">
                    {u.role}
                  </span>
                </td>
                <td className="px-4 py-3 font-mono">{u.branch_code || 'SEMUA CABANG'}</td>
                <td className="px-4 py-3">
                  <span
                    className={`px-2 py-0.5 rounded font-semibold ${
                      u.is_active
                        ? 'bg-emerald-950 text-emerald-300 border border-emerald-500/30'
                        : 'bg-rose-950 text-rose-300 border border-rose-500/30'
                    }`}
                  >
                    {u.is_active ? 'AKTIF' : 'NONAKTIF'}
                  </span>
                </td>
                <td className="px-4 py-3 text-right">
                  <button
                    onClick={() => toggleActive(u)}
                    className="px-3 py-1 rounded bg-slate-800 hover:bg-slate-700"
                  >
                    {u.is_active ? 'Nonaktifkan' : 'Aktifkan'}
                  </button>
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>

      {showAdd && (
        <div className="fixed inset-0 bg-black/75 flex items-center justify-center p-4 z-50">
          <div className="bg-[#11181c] border border-slate-700 rounded-xl max-w-md w-full p-6 space-y-4">
            <h3 className="text-base font-bold">Tambah User Dashboard Manual</h3>
            {err && (
              <div className="p-2 rounded bg-rose-950 border border-rose-500/40 text-rose-200 text-xs">
                {err}
              </div>
            )}
            <div className="space-y-3 text-xs">
              <div>
                <label className="text-slate-400">Username</label>
                <input
                  value={username}
                  onChange={(e) => setUsername(e.target.value)}
                  placeholder="misal: it.surabaya"
                  className="w-full mt-1 px-3 py-2 bg-slate-950 border border-slate-800 rounded-lg"
                />
              </div>
              <div>
                <label className="text-slate-400">Nama Lengkap</label>
                <input
                  value={fullName}
                  onChange={(e) => setFullName(e.target.value)}
                  placeholder="Budi Santoso (IT Ops Surabaya)"
                  className="w-full mt-1 px-3 py-2 bg-slate-950 border border-slate-800 rounded-lg"
                />
              </div>
              <div>
                <label className="text-slate-400">Password (min 6 karakter)</label>
                <input
                  type="password"
                  value={password}
                  onChange={(e) => setPassword(e.target.value)}
                  placeholder="••••••••"
                  className="w-full mt-1 px-3 py-2 bg-slate-950 border border-slate-800 rounded-lg"
                />
              </div>
              <div>
                <label className="text-slate-400">Role</label>
                <select
                  value={role}
                  onChange={(e) => setRole(e.target.value as 'SUPERADMIN' | 'IT_BRANCH' | 'VIEWER')}
                  className="w-full mt-1 px-3 py-2 bg-slate-950 border border-slate-800 rounded-lg"
                >
                  <option value="SUPERADMIN">SUPERADMIN (Akses Penuh)</option>
                  <option value="IT_BRANCH">IT_BRANCH (Admin Cabang)</option>
                  <option value="VIEWER">VIEWER (Hanya Lihat &amp; Export)</option>
                </select>
              </div>
              <div>
                <label className="text-slate-400">Cabang Terkait (Opsional)</label>
                <select
                  value={branchId}
                  onChange={(e) => setBranchId(e.target.value)}
                  className="w-full mt-1 px-3 py-2 bg-slate-950 border border-slate-800 rounded-lg"
                >
                  <option value="">SEMUA CABANG</option>
                  {branches.map((b) => (
                    <option key={b.id} value={b.id}>
                      {b.branch_code} — {b.branch_name}
                    </option>
                  ))}
                </select>
              </div>
            </div>
            <div className="flex justify-end gap-2 pt-2">
              <button
                onClick={() => setShowAdd(false)}
                className="px-4 py-2 rounded-lg bg-slate-800 text-xs"
              >
                Batal
              </button>
              <button
                onClick={createUser}
                className="px-4 py-2 rounded-lg bg-[#13d389] text-slate-950 text-xs font-bold"
              >
                Simpan User
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}

export function EventsAndOutboxView() {
  const [incidents, setIncidents] = useState<IncidentDto[]>([]);
  const [outbox, setOutbox] = useState<OutboxDto[]>([]);

  useEffect(() => {
    apiFetch<IncidentDto[]>('/api/v1/incidents').then(setIncidents).catch(() => {});
    apiFetch<OutboxDto[]>('/api/v1/outbox').then(setOutbox).catch(() => {});
  }, []);

  return (
    <div className="space-y-6">
      <div className="bg-[#11181c] border border-slate-800 rounded-xl overflow-x-auto">
        <div className="px-4 py-3 border-b border-slate-800 flex items-center justify-between">
          <h3 className="text-sm font-bold flex items-center gap-2">
            <Bell className="w-4 h-4 text-amber-400" /> Event Console: Riwayat Transisi State
          </h3>
          <a
            href={reportUrls.incidentsCsv()}
            className="text-xs text-[#13d389] hover:underline inline-flex items-center gap-1"
          >
            <Download className="w-3.5 h-3.5" /> Export Incidents CSV
          </a>
        </div>
        <table className="w-full text-left text-xs">
          <thead className="bg-[#0e1418] text-slate-400 border-b border-slate-800">
            <tr>
              <th className="px-4 py-2.5">Waktu</th>
              <th className="px-4 py-2.5">Cabang</th>
              <th className="px-4 py-2.5">Hostname</th>
              <th className="px-4 py-2.5">Service</th>
              <th className="px-4 py-2.5">Transisi</th>
              <th className="px-4 py-2.5">Ringkasan</th>
            </tr>
          </thead>
          <tbody className="divide-y divide-slate-800/70">
            {incidents.map((ev) => (
              <tr key={ev.id} className="hover:bg-[#1a252c]">
                <td className="px-4 py-2 font-mono text-slate-400">{ev.created_at}</td>
                <td className="px-4 py-2 font-mono text-[#13d389]">{ev.branch_code}</td>
                <td className="px-4 py-2 font-semibold">{ev.hostname}</td>
                <td className="px-4 py-2 font-mono">{ev.service_name}</td>
                <td className="px-4 py-2 font-mono">
                  {ev.from_state} &rarr; {ev.to_state}
                </td>
                <td className="px-4 py-2 text-slate-300">{ev.summary}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>

      <div className="bg-[#11181c] border border-slate-800 rounded-xl overflow-x-auto">
        <div className="px-4 py-3 border-b border-slate-800">
          <h3 className="text-sm font-bold flex items-center gap-2">
            <Send className="w-4 h-4 text-sky-400" /> Antrean Notifikasi (Telegram Topics, WA Webhook &amp; SMTP)
          </h3>
        </div>
        <table className="w-full text-left text-xs">
          <thead className="bg-[#0e1418] text-slate-400 border-b border-slate-800">
            <tr>
              <th className="px-4 py-2.5">Waktu</th>
              <th className="px-4 py-2.5">Host</th>
              <th className="px-4 py-2.5">Channel</th>
              <th className="px-4 py-2.5">Tujuan</th>
              <th className="px-4 py-2.5">Tipe</th>
              <th className="px-4 py-2.5">Status</th>
            </tr>
          </thead>
          <tbody className="divide-y divide-slate-800/70">
            {outbox.map((o) => (
              <tr key={o.id} className="hover:bg-[#1a252c]">
                <td className="px-4 py-2 font-mono text-slate-400">{o.created_at}</td>
                <td className="px-4 py-2 font-semibold">
                  {o.hostname} ({o.service_name})
                </td>
                <td className="px-4 py-2 font-mono">{o.target_role}</td>
                <td className="px-4 py-2 font-mono">{o.destination}</td>
                <td className="px-4 py-2">{o.event_type}</td>
                <td className="px-4 py-2">
                  <span
                    className={`px-2 py-0.5 rounded font-semibold ${
                      o.delivery_status === 'SENT'
                        ? 'bg-emerald-950 text-emerald-300'
                        : 'bg-amber-950 text-amber-300'
                    }`}
                  >
                    {o.delivery_status}
                  </span>
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </div>
  );
}

export function AddBranchModal({
  onClose,
  onSaved,
}: {
  onClose: () => void;
  onSaved: () => void;
}) {
  const [code, setCode] = useState('');
  const [name, setName] = useState('');
  const [token, setToken] = useState('enroll-branch-2026');
  const [chatId, setChatId] = useState('-1001987654321');
  const [topicId, setTopicId] = useState('');
  const [saving, setSaving] = useState(false);

  const save = async () => {
    setSaving(true);
    try {
      await apiFetch('/api/v1/branches', {
        method: 'POST',
        body: JSON.stringify({
          branch_code: code.trim().toUpperCase(),
          branch_name: name.trim(),
          enrollment_token: token.trim(),
          telegram_chat_id: chatId.trim() || null,
          telegram_topic_id: topicId.trim() ? Number(topicId.trim()) : null,
        }),
      });
      onSaved();
    } finally {
      setSaving(false);
    }
  };

  return (
    <div className="fixed inset-0 bg-black/75 flex items-center justify-center p-4 z-50">
      <div className="bg-[#11181c] border border-slate-700 rounded-xl max-w-md w-full p-6 space-y-4">
        <h3 className="text-base font-bold flex items-center gap-2">
          <Building2 className="w-4 h-4 text-[#13d389]" /> Tambah Cabang &amp; Token Enrollment
        </h3>
        <div className="space-y-3 text-xs">
          <div>
            <label className="text-slate-400">Kode Cabang (misal: BDG-01)</label>
            <input
              value={code}
              onChange={(e) => setCode(e.target.value)}
              className="w-full mt-1 px-3 py-2 bg-slate-950 border border-slate-800 rounded-lg font-mono uppercase"
            />
          </div>
          <div>
            <label className="text-slate-400">Nama Cabang</label>
            <input
              value={name}
              onChange={(e) => setName(e.target.value)}
              placeholder="Bandung Depo Barat"
              className="w-full mt-1 px-3 py-2 bg-slate-950 border border-slate-800 rounded-lg"
            />
          </div>
          <div>
            <label className="text-slate-400">ENROLLMENT_TOKEN Cabang</label>
            <input
              value={token}
              onChange={(e) => setToken(e.target.value)}
              className="w-full mt-1 px-3 py-2 bg-slate-950 border border-slate-800 rounded-lg font-mono"
            />
          </div>
          <div className="grid grid-cols-2 gap-2">
            <div>
              <label className="text-slate-400">Telegram Group ID</label>
              <input
                value={chatId}
                onChange={(e) => setChatId(e.target.value)}
                className="w-full mt-1 px-3 py-2 bg-slate-950 border border-slate-800 rounded-lg font-mono"
              />
            </div>
            <div>
              <label className="text-slate-400">Topic ID (thread)</label>
              <input
                type="number"
                value={topicId}
                onChange={(e) => setTopicId(e.target.value)}
                placeholder="85"
                className="w-full mt-1 px-3 py-2 bg-slate-950 border border-slate-800 rounded-lg font-mono"
              />
            </div>
          </div>
        </div>
        <div className="flex justify-end gap-2 pt-2">
          <button onClick={onClose} className="px-4 py-2 rounded-lg bg-slate-800 text-xs">
            Batal
          </button>
          <button
            disabled={saving || !code.trim() || !name.trim()}
            onClick={save}
            className="px-4 py-2 rounded-lg bg-[#13d389] text-slate-950 text-xs font-bold inline-flex items-center gap-1"
          >
            <Plus className="w-3.5 h-3.5" /> Simpan Cabang
          </button>
        </div>
      </div>
    </div>
  );
}

export function AddHostManualModal({
  branches,
  deviceTypes,
  onDeviceTypeAdded,
  onClose,
  onSaved,
}: {
  branches: BranchSummaryCard[];
  deviceTypes: DeviceTypeDto[];
  onDeviceTypeAdded: (dt: DeviceTypeDto) => void;
  onClose: () => void;
  onSaved: () => void;
}) {
  const [branchCode, setBranchCode] = useState(branches[0]?.branch_code || '');
  const [hostname, setHostname] = useState('');
  const [ipAddr, setIpAddr] = useState('');
  const [hostType, setHostType] = useState(deviceTypes[0]?.code || 'WINDOWS');
  const [showCustomType, setShowCustomType] = useState(false);
  const [customTypeLabel, setCustomTypeLabel] = useState('');
  const [legacyPull, setLegacyPull] = useState(true);
  const [ownerName, setOwnerName] = useState('');
  const [ownerWa, setOwnerWa] = useState('');
  const [ownerEmail, setOwnerEmail] = useState('');
  const [saving, setSaving] = useState(false);

  const addCustomType = async () => {
    const clean = customTypeLabel.trim();
    if (!clean) return;
    const code = clean.toUpperCase().replace(/[^A-Z0-9]+/g, '_');
    const created = await apiFetch<DeviceTypeDto>('/api/v1/device-types', {
      method: 'POST',
      body: JSON.stringify({ code, label: clean }),
    });
    onDeviceTypeAdded(created);
    setHostType(created.code);
    setCustomTypeLabel('');
    setShowCustomType(false);
  };

  const save = async () => {
    setSaving(true);
    try {
      await apiFetch('/api/v1/hosts', {
        method: 'POST',
        body: JSON.stringify({
          branch_code: branchCode,
          hostname: hostname.trim(),
          ip_address: ipAddr.trim(),
          host_type: hostType,
          pull_legacy_enabled: legacyPull,
          pull_legacy_port: 6556,
          owner_name: ownerName.trim() || null,
          owner_wa: ownerWa.trim() || null,
          owner_email: ownerEmail.trim() || null,
        }),
      });
      onSaved();
    } finally {
      setSaving(false);
    }
  };

  return (
    <div className="fixed inset-0 bg-black/75 flex items-center justify-center p-4 z-50">
      <div className="bg-[#11181c] border border-slate-700 rounded-xl max-w-md w-full p-6 space-y-4">
        <h3 className="text-base font-bold">Tambah Perangkat / Host Manual</h3>
        <div className="space-y-3 text-xs">
          <div>
            <label className="text-slate-400">Cabang</label>
            <select
              value={branchCode}
              onChange={(e) => setBranchCode(e.target.value)}
              className="w-full mt-1 px-3 py-2 bg-slate-950 border border-slate-800 rounded-lg"
            >
              {branches.map((b) => (
                <option key={b.id} value={b.branch_code}>
                  {b.branch_code} — {b.branch_name}
                </option>
              ))}
            </select>
          </div>
          <div>
            <label className="text-slate-400">Hostname</label>
            <input
              value={hostname}
              onChange={(e) => setHostname(e.target.value)}
              placeholder="BCPKDR-ADMPROG-MITHA"
              className="w-full mt-1 px-3 py-2 bg-slate-950 border border-slate-800 rounded-lg font-mono"
            />
          </div>
          <div className="grid grid-cols-2 gap-2">
            <div>
              <label className="text-slate-400">IP Address</label>
              <input
                value={ipAddr}
                onChange={(e) => setIpAddr(e.target.value)}
                placeholder="192.168.55.169"
                className="w-full mt-1 px-3 py-2 bg-slate-950 border border-slate-800 rounded-lg font-mono"
              />
            </div>
            <div>
              <div className="flex items-center justify-between">
                <label className="text-slate-400">Jenis Perangkat</label>
                <button
                  type="button"
                  onClick={() => setShowCustomType((v) => !v)}
                  className="text-[11px] text-[#13d389] hover:underline"
                >
                  + Jenis Lain
                </button>
              </div>
              <select
                value={hostType}
                onChange={(e) => setHostType(e.target.value)}
                className="w-full mt-1 px-3 py-2 bg-slate-950 border border-slate-800 rounded-lg"
              >
                {deviceTypes.map((dt) => (
                  <option key={dt.code} value={dt.code}>
                    {dt.label}
                  </option>
                ))}
              </select>
            </div>
          </div>

          {showCustomType && (
            <div className="p-2.5 rounded-lg bg-slate-950 border border-[#13d389]/40 flex items-center gap-2">
              <input
                value={customTypeLabel}
                onChange={(e) => setCustomTypeLabel(e.target.value)}
                placeholder="Jenis baru (misal: Access Point, CCTV, UPS)"
                className="flex-1 px-2.5 py-1.5 bg-[#11181c] border border-slate-800 rounded text-xs"
              />
              <button
                type="button"
                onClick={addCustomType}
                className="px-3 py-1.5 rounded bg-[#13d389] text-slate-950 font-bold text-xs"
              >
                Tambah
              </button>
            </div>
          )}

          <label className="flex items-center gap-2 pt-1">
            <input
              type="checkbox"
              checked={legacyPull}
              onChange={(e) => setLegacyPull(e.target.checked)}
            />
            Aktifkan Legacy TCP/6556 Pull (jika belum memakai agent-rust push)
          </label>
          <div className="grid grid-cols-3 gap-2">
            <div>
              <label className="text-slate-400">Owner / PIC</label>
              <input
                value={ownerName}
                onChange={(e) => setOwnerName(e.target.value)}
                placeholder="Mitha"
                className="w-full mt-1 px-2.5 py-2 bg-slate-950 border border-slate-800 rounded-lg"
              />
            </div>
            <div>
              <label className="text-slate-400">No WA</label>
              <input
                value={ownerWa}
                onChange={(e) => setOwnerWa(e.target.value)}
                placeholder="62812..."
                className="w-full mt-1 px-2.5 py-2 bg-slate-950 border border-slate-800 rounded-lg"
              />
            </div>
            <div>
              <label className="text-slate-400">Email User</label>
              <input
                value={ownerEmail}
                onChange={(e) => setOwnerEmail(e.target.value)}
                placeholder="user@..."
                className="w-full mt-1 px-2.5 py-2 bg-slate-950 border border-slate-800 rounded-lg"
              />
            </div>
          </div>
        </div>
        <div className="flex justify-end gap-2 pt-2">
          <button onClick={onClose} className="px-4 py-2 rounded-lg bg-slate-800 text-xs">
            Batal
          </button>
          <button
            disabled={saving || !hostname.trim() || !ipAddr.trim()}
            onClick={save}
            className="px-4 py-2 rounded-lg bg-[#13d389] text-slate-950 text-xs font-bold"
          >
            Tambahkan Host
          </button>
        </div>
      </div>
    </div>
  );
}

export function SettingsIntegrationView({
  deviceTypes,
  onDeviceTypeAdded,
}: {
  deviceTypes: DeviceTypeDto[];
  onDeviceTypeAdded: (dt: DeviceTypeDto) => void;
}) {
  const [form, setForm] = useState<SettingsViewDto>({
    rest_api_public_url: 'http://192.168.55.112:8080',
    default_enrollment_token: 'enroll-kdr-hq-secret-token-2026',
    batch_flush_interval_sec: 180,
    host_stale_threshold_sec: 180,
    flapping_threshold_cycles: 2,
    telegram_default_chat_id: '-1001987654321',
    has_telegram_bot_token: false,
    wa_webhook_url: 'https://wa-gateway.internal/api/send',
    has_wa_webhook_token: false,
    smtp_host: 'smtp.office365.com',
    smtp_port: 587,
    smtp_username: 'monitoring@company.co.id',
    smtp_from: 'Monitoring IT Ops <monitoring@company.co.id>',
    smtp_use_tls: true,
    has_smtp_password: false,
  });
  const [tgToken, setTgToken] = useState('');
  const [waToken, setWaToken] = useState('');
  const [smtpPassword, setSmtpPassword] = useState('');
  const [testEmailTo, setTestEmailTo] = useState('');
  const [newTypeLabel, setNewTypeLabel] = useState('');
  const [statusMsg, setStatusMsg] = useState<string | null>(null);
  const [errMsg, setErrMsg] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);

  useEffect(() => {
    apiFetch<SettingsViewDto>('/api/v1/settings')
      .then(setForm)
      .catch(() => {});
  }, []);

  const saveAll = async () => {
    setSaving(true);
    setStatusMsg(null);
    setErrMsg(null);
    try {
      await apiFetch('/api/v1/settings', {
        method: 'PUT',
        body: JSON.stringify({
          rest_api_public_url: form.rest_api_public_url,
          default_enrollment_token: form.default_enrollment_token,
          batch_flush_interval_sec: Number(form.batch_flush_interval_sec),
          host_stale_threshold_sec: Number(form.host_stale_threshold_sec),
          flapping_threshold_cycles: Number(form.flapping_threshold_cycles),
          telegram_bot_token: tgToken.trim() || null,
          telegram_default_chat_id: form.telegram_default_chat_id,
          wa_webhook_url: form.wa_webhook_url,
          wa_webhook_token: waToken.trim() || null,
          smtp_host: form.smtp_host,
          smtp_port: Number(form.smtp_port),
          smtp_username: form.smtp_username,
          smtp_password: smtpPassword || null,
          smtp_from: form.smtp_from,
          smtp_use_tls: form.smtp_use_tls,
        }),
      });
      setTgToken('');
      setWaToken('');
      setSmtpPassword('');
      setStatusMsg('Pengaturan REST API, Email Utama SMTP, WhatsApp, & Telegram berhasil disimpan!');
      const refreshed = await apiFetch<SettingsViewDto>('/api/v1/settings');
      setForm(refreshed);
    } catch (e: unknown) {
      setErrMsg(e instanceof Error ? e.message : 'Gagal menyimpan pengaturan');
    } finally {
      setSaving(false);
    }
  };

  const sendTestEmail = async () => {
    setStatusMsg(null);
    setErrMsg(null);
    try {
      const res = await apiFetch<string>('/api/v1/settings/test-email', {
        method: 'POST',
        body: JSON.stringify({ to_email: testEmailTo.trim() }),
      });
      setStatusMsg(res);
    } catch (e: unknown) {
      setErrMsg(e instanceof Error ? e.message : 'Gagal mengirim email tes');
    }
  };

  const addType = async () => {
    const clean = newTypeLabel.trim();
    if (!clean) return;
    const code = clean.toUpperCase().replace(/[^A-Z0-9]+/g, '_');
    const created = await apiFetch<DeviceTypeDto>('/api/v1/device-types', {
      method: 'POST',
      body: JSON.stringify({ code, label: clean }),
    });
    onDeviceTypeAdded(created);
    setNewTypeLabel('');
  };

  return (
    <div className="space-y-6">
      <div className="flex flex-wrap items-center justify-between gap-3">
        <div>
          <h2 className="text-lg font-bold flex items-center gap-2">
            <Settings className="w-5 h-5 text-[#13d389]" /> Pengaturan REST API, Email Utama (SMTP), &amp; Integrasi
          </h2>
          <p className="text-xs text-slate-400">
            Nilai awal dimuat dari berkas <code className="text-slate-300">.env</code> dan dapat diubah kapan saja di sini (tersimpan di MariaDB <code className="text-slate-300">system_settings</code>).
          </p>
        </div>
        <button
          disabled={saving}
          onClick={saveAll}
          className="px-4 py-2 rounded-lg bg-[#13d389] hover:bg-[#10b977] text-slate-950 text-xs font-bold inline-flex items-center gap-2"
        >
          <Save className="w-4 h-4" /> {saving ? 'Menyimpan...' : 'Simpan Semua Pengaturan'}
        </button>
      </div>

      {statusMsg && (
        <div className="p-3 rounded-xl bg-emerald-950/70 border border-emerald-500/40 text-emerald-300 text-xs flex items-center gap-2">
          <CheckCircle2 className="w-4 h-4 shrink-0" /> {statusMsg}
        </div>
      )}
      {errMsg && (
        <div className="p-3 rounded-xl bg-rose-950/70 border border-rose-500/40 text-rose-200 text-xs">
          {errMsg}
        </div>
      )}

      <div className="grid grid-cols-1 lg:grid-cols-2 gap-5">
        {/* 1. EMAIL UTAMA SMTP UNTUK KIRIM ALERT KE USER */}
        <div className="bg-[#11181c] border border-slate-800 rounded-xl p-5 space-y-4">
          <div className="flex items-center justify-between">
            <h3 className="text-sm font-bold flex items-center gap-2">
              <Mail className="w-4 h-4 text-[#13d389]" /> Email Utama Pengirim Alert (SMTP)
            </h3>
            <span className="text-[11px] px-2 py-0.5 rounded bg-slate-800 text-slate-300">
              {form.has_smtp_password ? 'Password Tersimpan' : 'Belum Ada Password'}
            </span>
          </div>

          <div className="grid grid-cols-3 gap-3 text-xs">
            <div className="col-span-2">
              <label className="text-slate-400">SMTP Host</label>
              <input
                value={form.smtp_host}
                onChange={(e) => setForm({ ...form, smtp_host: e.target.value })}
                placeholder="smtp.gmail.com / smtp.office365.com"
                className="w-full mt-1 px-3 py-2 bg-slate-950 border border-slate-800 rounded-lg font-mono"
              />
            </div>
            <div>
              <label className="text-slate-400">SMTP Port</label>
              <input
                type="number"
                value={form.smtp_port}
                onChange={(e) => setForm({ ...form, smtp_port: Number(e.target.value) })}
                className="w-full mt-1 px-3 py-2 bg-slate-950 border border-slate-800 rounded-lg font-mono"
              />
            </div>
          </div>

          <div className="grid grid-cols-2 gap-3 text-xs">
            <div>
              <label className="text-slate-400">Username / Email Akun Utama</label>
              <input
                value={form.smtp_username}
                onChange={(e) => setForm({ ...form, smtp_username: e.target.value })}
                placeholder="it-alert@company.co.id"
                className="w-full mt-1 px-3 py-2 bg-slate-950 border border-slate-800 rounded-lg font-mono"
              />
            </div>
            <div>
              <label className="text-slate-400">Password / App Password Baru</label>
              <input
                type="password"
                value={smtpPassword}
                onChange={(e) => setSmtpPassword(e.target.value)}
                placeholder={form.has_smtp_password ? '•••••••• (biarkan kosong jika tetap)' : 'Masukkan App Password'}
                className="w-full mt-1 px-3 py-2 bg-slate-950 border border-slate-800 rounded-lg"
              />
            </div>
          </div>

          <div className="text-xs">
            <label className="text-slate-400">Nama &amp; Alamat Pengirim (From Header)</label>
            <input
              value={form.smtp_from}
              onChange={(e) => setForm({ ...form, smtp_from: e.target.value })}
              placeholder="Monitoring IT Ops <it-alert@company.co.id>"
              className="w-full mt-1 px-3 py-2 bg-slate-950 border border-slate-800 rounded-lg"
            />
          </div>

          <label className="flex items-center gap-2 text-xs text-slate-300">
            <input
              type="checkbox"
              checked={form.smtp_use_tls}
              onChange={(e) => setForm({ ...form, smtp_use_tls: e.target.checked })}
            />
            Gunakan Enkripsi STARTTLS (Wajib untuk Gmail / Microsoft 365 port 587)
          </label>

          <div className="pt-3 border-t border-slate-800/80 space-y-2">
            <label className="text-xs text-slate-400 block">Uji Kirim Email Utama ke Alamat Tujuan:</label>
            <div className="flex gap-2">
              <input
                value={testEmailTo}
                onChange={(e) => setTestEmailTo(e.target.value)}
                placeholder="email-tujuan@company.co.id"
                className="flex-1 px-3 py-1.5 bg-slate-950 border border-slate-800 rounded-lg text-xs"
              />
              <button
                type="button"
                onClick={sendTestEmail}
                className="px-3.5 py-1.5 rounded-lg bg-sky-600 hover:bg-sky-500 text-xs font-semibold"
              >
                Test Kirim Email
              </button>
            </div>
          </div>
        </div>

        {/* 2. TELEGRAM BOT & WHATSAPP WEBHOOK */}
        <div className="bg-[#11181c] border border-slate-800 rounded-xl p-5 space-y-4">
          <h3 className="text-sm font-bold flex items-center gap-2">
            <Send className="w-4 h-4 text-sky-400" /> Integrasi Telegram Forum Topics &amp; WhatsApp Webhook
          </h3>

          <div className="space-y-3 text-xs">
            <div>
              <label className="text-slate-400">
                Telegram Bot Token {form.has_telegram_bot_token ? '(Sudah Terisi)' : ''}
              </label>
              <input
                type="password"
                value={tgToken}
                onChange={(e) => setTgToken(e.target.value)}
                placeholder={form.has_telegram_bot_token ? '•••••••• (kosongkan jika tidak diubah)' : '123456:ABC-DEF...'}
                className="w-full mt-1 px-3 py-2 bg-slate-950 border border-slate-800 rounded-lg font-mono"
              />
            </div>
            <div>
              <label className="text-slate-400">Default Telegram Supergroup Chat ID (IT Ops)</label>
              <input
                value={form.telegram_default_chat_id}
                onChange={(e) => setForm({ ...form, telegram_default_chat_id: e.target.value })}
                placeholder="-1001987654321"
                className="w-full mt-1 px-3 py-2 bg-slate-950 border border-slate-800 rounded-lg font-mono"
              />
            </div>
            <div className="pt-2 border-t border-slate-800/80">
              <label className="text-slate-400">WhatsApp Gateway Webhook URL (Untuk End-User)</label>
              <input
                value={form.wa_webhook_url}
                onChange={(e) => setForm({ ...form, wa_webhook_url: e.target.value })}
                placeholder="https://wa-gateway.internal/api/send"
                className="w-full mt-1 px-3 py-2 bg-slate-950 border border-slate-800 rounded-lg font-mono"
              />
            </div>
            <div>
              <label className="text-slate-400">
                WA Webhook Bearer Token {form.has_wa_webhook_token ? '(Sudah Terisi)' : ''}
              </label>
              <input
                type="password"
                value={waToken}
                onChange={(e) => setWaToken(e.target.value)}
                placeholder={form.has_wa_webhook_token ? '•••••••• (kosongkan jika tidak diubah)' : 'Bearer token WA gateway'}
                className="w-full mt-1 px-3 py-2 bg-slate-950 border border-slate-800 rounded-lg font-mono"
              />
            </div>
          </div>
        </div>

        {/* 3. SETTINGAN REST API & ENGINE INTERVALS */}
        <div className="bg-[#11181c] border border-slate-800 rounded-xl p-5 space-y-4">
          <h3 className="text-sm font-bold flex items-center gap-2">
            <Globe className="w-4 h-4 text-[#13d389]" /> Settingan REST API, Enrollment Token &amp; Cache Engine
          </h3>
          <div className="space-y-3 text-xs">
            <div>
              <label className="text-slate-400">REST API Server URL (Untuk Agen Cabang)</label>
              <input
                value={form.rest_api_public_url}
                onChange={(e) => setForm({ ...form, rest_api_public_url: e.target.value })}
                className="w-full mt-1 px-3 py-2 bg-slate-950 border border-slate-800 rounded-lg font-mono"
              />
            </div>
            <div>
              <label className="text-slate-400">Default Branch ENROLLMENT_TOKEN</label>
              <input
                value={form.default_enrollment_token}
                onChange={(e) => setForm({ ...form, default_enrollment_token: e.target.value })}
                className="w-full mt-1 px-3 py-2 bg-slate-950 border border-slate-800 rounded-lg font-mono"
              />
            </div>
            <div className="grid grid-cols-3 gap-2">
              <div>
                <label className="text-slate-400">Batch Flush (dtk)</label>
                <input
                  type="number"
                  value={form.batch_flush_interval_sec}
                  onChange={(e) =>
                    setForm({ ...form, batch_flush_interval_sec: Number(e.target.value) })
                  }
                  className="w-full mt-1 px-3 py-2 bg-slate-950 border border-slate-800 rounded-lg font-mono"
                />
              </div>
              <div>
                <label className="text-slate-400">Stale Timeout (dtk)</label>
                <input
                  type="number"
                  value={form.host_stale_threshold_sec}
                  onChange={(e) =>
                    setForm({ ...form, host_stale_threshold_sec: Number(e.target.value) })
                  }
                  className="w-full mt-1 px-3 py-2 bg-slate-950 border border-slate-800 rounded-lg font-mono"
                />
              </div>
              <div>
                <label className="text-slate-400">Flapping (siklus)</label>
                <input
                  type="number"
                  value={form.flapping_threshold_cycles}
                  onChange={(e) =>
                    setForm({ ...form, flapping_threshold_cycles: Number(e.target.value) })
                  }
                  className="w-full mt-1 px-3 py-2 bg-slate-950 border border-slate-800 rounded-lg font-mono"
                />
              </div>
            </div>
            <div className="p-3 rounded-lg bg-slate-950 border border-slate-800 font-mono text-[11px] text-slate-300 space-y-1">
              <div className="text-[#13d389] font-bold mb-1">Endpoint REST API Aktif:</div>
              <div>POST {form.rest_api_public_url}/api/v1/register (Two-Phase Enroll)</div>
              <div>POST {form.rest_api_public_url}/api/v1/ingest (HMAC-SHA256 ±5m)</div>
              <div>GET  {form.rest_api_public_url}/api/v1/branches/summary</div>
              <div>GET  {form.rest_api_public_url}/api/v1/hosts?page=1&amp;per_page=50</div>
              <div>GET  {form.rest_api_public_url}/api/v1/reports/hosts.csv</div>
            </div>
          </div>
        </div>

        {/* 4. KELOLA JENIS PERANGKAT DINAMIS (TANPA MERK + TAMBAH JENIS LAIN) */}
        <div className="bg-[#11181c] border border-slate-800 rounded-xl p-5 space-y-4">
          <h3 className="text-sm font-bold">Jenis Perangkat (Device Types)</h3>
          <p className="text-xs text-slate-400">
            Default bersih tanpa merk: <b>Windows, Linux, Android, Router, Switch, Firewall, Printer</b>. Anda bebas menambahkan jenis perangkat tambahan di bawah ini.
          </p>
          <div className="flex flex-wrap gap-2">
            {deviceTypes.map((dt) => (
              <span
                key={dt.code}
                className="px-2.5 py-1 rounded-lg text-xs font-mono bg-slate-950 border border-slate-800 text-slate-200"
              >
                {dt.label} <span className="text-slate-500">({dt.code})</span>
              </span>
            ))}
          </div>
          <div className="flex gap-2 pt-2">
            <input
              value={newTypeLabel}
              onChange={(e) => setNewTypeLabel(e.target.value)}
              placeholder="Tambah jenis lain (misal: Access Point, CCTV, UPS, NVR)"
              className="flex-1 px-3 py-2 bg-slate-950 border border-slate-800 rounded-lg text-xs"
            />
            <button
              type="button"
              onClick={addType}
              className="px-4 py-2 rounded-lg bg-[#13d389] text-slate-950 text-xs font-bold inline-flex items-center gap-1"
            >
              <Plus className="w-3.5 h-3.5" /> Tambah Jenis
            </button>
          </div>
        </div>
      </div>
    </div>
  );
}

export function AgentDownloadsView({ branches }: { branches: BranchSummaryCard[] }) {
  const [packages, setPackages] = useState<AgentPackageDto[]>([]);
  const [selectedBranch, setSelectedBranch] = useState(branches[0]?.branch_code || '');
  const [serverBaseUrl, setServerBaseUrl] = useState(
    typeof window !== 'undefined' ? window.location.origin : 'http://192.168.55.112:8080'
  );
  const [copiedId, setCopiedId] = useState<string | null>(null);

  useEffect(() => {
    apiFetch<AgentPackageDto[]>('/api/v1/downloads')
      .then(setPackages)
      .catch(() => {});
  }, []);

  const cleanBase = serverBaseUrl.trim().replace(/\/+$/, '');
  const linuxOneLiner = `curl -fsSL "${cleanBase}/api/v1/downloads/install-linux.sh?branch=${selectedBranch}" | sudo bash`;
  const winOneLiner = `irm "${cleanBase}/api/v1/downloads/install-windows.ps1?branch=${selectedBranch}" | iex`;

  const copyCmd = (id: string, text: string) => {
    navigator.clipboard?.writeText(text);
    setCopiedId(id);
    setTimeout(() => setCopiedId(null), 2000);
  };

  const formatSize = (bytes: number) => {
    if (!bytes) return 'Siap via GitHub / Upload ke ./dist/agents/';
    return `${(bytes / (1024 * 1024)).toFixed(2)} MB`;
  };

  return (
    <div className="space-y-6">
      <div>
        <h2 className="text-lg font-bold flex items-center gap-2">
          <HardDriveDownload className="w-5 h-5 text-[#13d389]" /> Download Aplikasi Agent BMS (Self-Hosted &amp; GitHub)
        </h2>
        <p className="text-xs text-slate-400">
          Unduh binary &amp; APK agent langsung dari server lokal BMS (intranet tanpa internet) atau dari GitHub Releases.
        </p>
      </div>

      {/* ONE-LINE INSTALLER GENERATOR PER CABANG */}
      <div className="bg-[#11181c] border border-slate-800 rounded-xl p-5 space-y-4">
        <div className="flex flex-wrap items-center justify-between gap-3">
          <h3 className="text-sm font-bold flex items-center gap-2">
            <Terminal className="w-4 h-4 text-[#13d389]" /> Generator Perintah Install Otomatis 1-Baris (Otomatis Enroll ke Cabang)
          </h3>
          <div className="flex flex-wrap items-center gap-2 text-xs">
            <select
              value={selectedBranch}
              onChange={(e) => setSelectedBranch(e.target.value)}
              className="px-3 py-1.5 bg-slate-950 border border-slate-800 rounded-lg"
            >
              {branches.map((b) => (
                <option key={b.id} value={b.branch_code}>
                  Cabang: {b.branch_code} — {b.branch_name}
                </option>
              ))}
            </select>
            <input
              value={serverBaseUrl}
              onChange={(e) => setServerBaseUrl(e.target.value)}
              placeholder="http://192.168.55.112:3000"
              className="px-3 py-1.5 bg-slate-950 border border-slate-800 rounded-lg font-mono w-60"
            />
          </div>
        </div>

        <div className="grid grid-cols-1 lg:grid-cols-2 gap-4 text-xs">
          <div className="bg-slate-950 border border-slate-800 rounded-lg p-3.5 space-y-2">
            <div className="flex items-center justify-between">
              <span className="font-bold text-slate-200">Linux (Ubuntu / Debian / Mint / Raspberry Pi)</span>
              <div className="flex gap-2">
                <a
                  href={reportUrls.linuxInstaller(selectedBranch)}
                  className="text-[#13d389] hover:underline inline-flex items-center gap-1"
                >
                  <Download className="w-3.5 h-3.5" /> .sh
                </a>
                <button
                  onClick={() => copyCmd('linux', linuxOneLiner)}
                  className="px-2 py-0.5 rounded bg-slate-800 hover:bg-slate-700 inline-flex items-center gap-1"
                >
                  <Copy className="w-3 h-3" /> {copiedId === 'linux' ? 'Tersalin!' : 'Copy'}
                </button>
              </div>
            </div>
            <pre className="text-[11px] font-mono text-emerald-300 overflow-x-auto whitespace-pre-wrap break-all">
              {linuxOneLiner}
            </pre>
          </div>

          <div className="bg-slate-950 border border-slate-800 rounded-lg p-3.5 space-y-2">
            <div className="flex items-center justify-between">
              <span className="font-bold text-slate-200">Windows 10 / 11 / Server (PowerShell Administrator)</span>
              <div className="flex gap-2">
                <a
                  href={reportUrls.windowsInstaller(selectedBranch)}
                  className="text-[#13d389] hover:underline inline-flex items-center gap-1"
                >
                  <Download className="w-3.5 h-3.5" /> .ps1
                </a>
                <button
                  onClick={() => copyCmd('win', winOneLiner)}
                  className="px-2 py-0.5 rounded bg-slate-800 hover:bg-slate-700 inline-flex items-center gap-1"
                >
                  <Copy className="w-3 h-3" /> {copiedId === 'win' ? 'Tersalin!' : 'Copy'}
                </button>
              </div>
            </div>
            <pre className="text-[11px] font-mono text-sky-300 overflow-x-auto whitespace-pre-wrap break-all">
              {winOneLiner}
            </pre>
          </div>
        </div>
      </div>

      {/* DAFTAR PAKET BINARY & APK DOWNLOAD LANGSUNG */}
      <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
        {packages.map((pkg) => (
          <div
            key={pkg.id}
            className="bg-[#11181c] border border-slate-800 rounded-xl p-5 flex flex-col justify-between space-y-4"
          >
            <div className="space-y-2">
              <div className="flex items-center justify-between">
                <span className="text-[11px] font-mono px-2 py-0.5 rounded bg-slate-800 text-[#13d389]">
                  {pkg.arch}
                </span>
                <span
                  className={`text-[10px] font-semibold px-2 py-0.5 rounded ${
                    pkg.available_locally
                      ? 'bg-emerald-950 text-emerald-300 border border-emerald-500/30'
                      : 'bg-amber-950 text-amber-300 border border-amber-500/30'
                  }`}
                >
                  {pkg.available_locally ? 'Siap di Server BMS' : 'GitHub / Upload Lokal'}
                </span>
              </div>
              <h3 className="text-base font-bold">{pkg.os_label}</h3>
              <p className="text-xs text-slate-400">{pkg.description}</p>
              <div className="text-[11px] font-mono text-slate-500">
                File: {pkg.filename} • {formatSize(pkg.size_bytes)}
              </div>
            </div>

            <div className="flex items-center gap-2 pt-2 border-t border-slate-800/80">
              <a
                href={pkg.direct_download_url}
                className="flex-1 text-center px-3 py-2 rounded-lg bg-[#13d389] hover:bg-[#10b977] text-slate-950 text-xs font-bold inline-flex items-center justify-center gap-1.5"
              >
                <Download className="w-3.5 h-3.5" /> Download (Server)
              </a>
              <a
                href={pkg.github_release_url}
                target="_blank"
                rel="noreferrer"
                className="px-3 py-2 rounded-lg bg-slate-800 hover:bg-slate-700 text-xs font-medium inline-flex items-center gap-1"
                title="Buka Mirror GitHub Releases"
              >
                <ExternalLink className="w-3.5 h-3.5" /> GitHub
              </a>
            </div>
          </div>
        ))}
      </div>
    </div>
  );
}
