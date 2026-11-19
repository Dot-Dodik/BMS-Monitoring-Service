import React, { useEffect, useState } from 'react';
import {
  Activity,
  AlertTriangle,
  Bell,
  Building2,
  CheckCircle2,
  Download,
  FileSpreadsheet,
  HardDrive,
  HardDriveDownload,
  Plus,
  RefreshCw,
  Search,
  Send,
  Server,
  Settings,
  ShieldAlert,
  UserCheck,
  Users,
  XCircle,
} from 'lucide-react';
import {
  apiFetch,
  BranchSummaryCard,
  DeviceTypeDto,
  HostRowDto,
  InventoryItemDto,
  PaginatedHostsResponse,
  reportUrls,
  ServiceRowDto,
} from './api/client';
import { HostServicesView } from './components/HostServicesView';
import { ServiceDetailView } from './components/ServiceDetailView';
import {
  AddBranchModal,
  AddHostManualModal,
  AgentDownloadsView,
  EventsAndOutboxView,
  ReportsCenterView,
  SettingsIntegrationView,
  UsersManagementView,
} from './components/AdminViews';

export class AppErrorBoundary extends React.Component<
  { children: React.ReactNode },
  { hasError: boolean; message: string }
> {
  constructor(props: { children: React.ReactNode }) {
    super(props);
    this.state = { hasError: false, message: '' };
  }
  static getDerivedStateFromError(err: Error) {
    return { hasError: true, message: err.message };
  }
  render() {
    if (this.state.hasError) {
      return (
        <div className="min-h-screen bg-[#0c1216] text-slate-100 flex items-center justify-center p-6">
          <div className="max-w-md w-full bg-[#11181c] border border-rose-500/40 rounded-xl p-6 text-center space-y-4">
            <AlertTriangle className="w-10 h-10 text-rose-400 mx-auto" />
            <h2 className="text-lg font-bold">Terjadi Gangguan Tampilan</h2>
            <p className="text-sm text-slate-400">{this.state.message}</p>
            <button
              onClick={() => window.location.reload()}
              className="px-4 py-2 bg-[#13d389] text-slate-950 rounded-lg text-sm font-bold"
            >
              Muat Ulang Halaman
            </button>
          </div>
        </div>
      );
    }
    return this.props.children;
  }
}

type ActiveSection =
  | 'branches'
  | 'hosts'
  | 'events'
  | 'inventory'
  | 'reports'
  | 'downloads'
  | 'users'
  | 'approvals'
  | 'settings';

const DEFAULT_DEVICE_TYPES: DeviceTypeDto[] = [
  { code: 'WINDOWS', label: 'Windows', is_builtin: true },
  { code: 'LINUX', label: 'Linux', is_builtin: true },
  { code: 'ANDROID', label: 'Android', is_builtin: true },
  { code: 'ROUTER', label: 'Router', is_builtin: true },
  { code: 'SWITCH', label: 'Switch', is_builtin: true },
  { code: 'FIREWALL', label: 'Firewall', is_builtin: true },
  { code: 'PRINTER', label: 'Printer', is_builtin: true },
];

export default function App() {
  const [section, setSection] = useState<ActiveSection>('branches');
  const [branches, setBranches] = useState<BranchSummaryCard[]>([]);
  const [deviceTypes, setDeviceTypes] = useState<DeviceTypeDto[]>(DEFAULT_DEVICE_TYPES);
  const [hostsPage, setHostsPage] = useState<PaginatedHostsResponse | null>(null);
  const [inventory, setInventory] = useState<InventoryItemDto[]>([]);

  // Drill-down 1-per-1 Checkmk Style
  const [selectedHost, setSelectedHost] = useState<HostRowDto | null>(null);
  const [selectedService, setSelectedService] = useState<ServiceRowDto | null>(null);

  const [loading, setLoading] = useState<boolean>(false);
  const [errorMsg, setErrorMsg] = useState<string | null>(null);

  // Filter & Pagination 50 baris/halaman
  const [page, setPage] = useState<number>(1);
  const [selectedBranch, setSelectedBranch] = useState<string>('');
  const [selectedType, setSelectedType] = useState<string>('');
  const [searchQuery, setSearchQuery] = useState<string>('');
  const [actionNeededOnly, setActionNeededOnly] = useState<boolean>(false);

  // Modals
  const [editingHost, setEditingHost] = useState<HostRowDto | null>(null);
  const [editingBranch, setEditingBranch] = useState<BranchSummaryCard | null>(null);
  const [showAddHost, setShowAddHost] = useState(false);
  const [showAddBranch, setShowAddBranch] = useState(false);

  const loadData = async () => {
    setLoading(true);
    setErrorMsg(null);
    try {
      const [bSummary, dtList] = await Promise.all([
        apiFetch<BranchSummaryCard[]>('/api/v1/branches/summary'),
        apiFetch<DeviceTypeDto[]>('/api/v1/device-types').catch(() => DEFAULT_DEVICE_TYPES),
      ]);
      setBranches(bSummary);
      if (dtList.length > 0) setDeviceTypes(dtList);

      if (section === 'hosts' || section === 'approvals') {
        const params = new URLSearchParams();
        params.set('page', String(page));
        params.set('per_page', '50');
        if (selectedBranch) params.set('branch_code', selectedBranch);
        if (selectedType) params.set('host_type', selectedType);
        if (searchQuery.trim()) params.set('search', searchQuery.trim());
        if (actionNeededOnly) params.set('action_needed', 'true');
        if (section === 'approvals') params.set('approval_filter', 'CONFLICT');

        const hResp = await apiFetch<PaginatedHostsResponse>(
          `/api/v1/hosts?${params.toString()}`
        );
        setHostsPage(hResp);
      } else if (section === 'inventory') {
        const invResp = await apiFetch<InventoryItemDto[]>('/api/v1/inventory');
        setInventory(invResp);
      }
    } catch (e: unknown) {
      setErrorMsg(e instanceof Error ? e.message : 'Gagal memuat data dari server');
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    loadData();
    const timer = setInterval(loadData, 15000);
    return () => clearInterval(timer);
  }, [section, page, selectedBranch, selectedType, actionNeededOnly]);

  const navigateSection = (next: ActiveSection) => {
    setSelectedService(null);
    setSelectedHost(null);
    setSection(next);
  };

  const totalFleet = branches.reduce((acc, b) => acc + b.total_hosts, 0);
  const totalUp = branches.reduce((acc, b) => acc + b.up_count, 0);
  const totalProblems = branches.reduce(
    (acc, b) => acc + b.warn_count + b.crit_count + b.down_count,
    0
  );
  const healthPct = totalFleet > 0 ? Math.round((totalUp / totalFleet) * 100) : 100;

  return (
    <div className="min-h-screen bg-[#0c1216] text-slate-100 flex">
      {/* LEFT CHECKMK-STYLE NAVIGATION RAIL */}
      <aside className="w-56 shrink-0 bg-[#11181c] border-r border-slate-800/90 flex flex-col justify-between">
        <div>
          <div className="px-4 py-4 border-b border-slate-800/90 flex items-center gap-2.5">
            <div className="p-1.5 rounded-lg bg-[#13d389]/15 border border-[#13d389]/40">
              <Activity className="w-5 h-5 text-[#13d389]" />
            </div>
            <div>
              <div className="text-base font-extrabold tracking-wide leading-tight text-white">BMS</div>
              <div className="text-[11px] text-[#13d389] leading-tight">Borwita Monitoring Service</div>
            </div>
          </div>

          <nav className="p-2.5 space-y-1 text-xs">
            <div className="px-2.5 pt-2 pb-1 text-[10px] font-bold uppercase tracking-wider text-slate-500">
              Monitor
            </div>
            <SidebarItem
              active={section === 'branches' && !selectedHost}
              icon={<Building2 className="w-4 h-4" />}
              label={`Overview Cabang (${branches.length})`}
              onClick={() => navigateSection('branches')}
            />
            <SidebarItem
              active={section === 'hosts' && !actionNeededOnly}
              icon={<Server className="w-4 h-4" />}
              label="All Hosts (50/Hlm)"
              onClick={() => {
                setActionNeededOnly(false);
                navigateSection('hosts');
              }}
            />
            <SidebarItem
              active={section === 'hosts' && actionNeededOnly}
              icon={<AlertTriangle className="w-4 h-4 text-rose-400" />}
              label={`Action Needed (${totalProblems})`}
              onClick={() => {
                setActionNeededOnly(true);
                setPage(1);
                navigateSection('hosts');
              }}
            />
            <SidebarItem
              active={section === 'events'}
              icon={<Bell className="w-4 h-4" />}
              label="Event & Alert Outbox"
              onClick={() => navigateSection('events')}
            />

            <div className="px-2.5 pt-4 pb-1 text-[10px] font-bold uppercase tracking-wider text-slate-500">
              Assets &amp; Reports
            </div>
            <SidebarItem
              active={section === 'inventory'}
              icon={<HardDrive className="w-4 h-4" />}
              label="HW/SW Inventory"
              onClick={() => navigateSection('inventory')}
            />
            <SidebarItem
              active={section === 'reports'}
              icon={<FileSpreadsheet className="w-4 h-4" />}
              label="Export Laporan"
              onClick={() => navigateSection('reports')}
            />
            <SidebarItem
              active={section === 'downloads'}
              icon={<HardDriveDownload className="w-4 h-4 text-[#13d389]" />}
              label="Download Agent"
              onClick={() => navigateSection('downloads')}
            />

            <div className="px-2.5 pt-4 pb-1 text-[10px] font-bold uppercase tracking-wider text-slate-500">
              Setup &amp; Admin
            </div>
            <SidebarItem
              active={section === 'users'}
              icon={<Users className="w-4 h-4" />}
              label="Users (Tambah Manual)"
              onClick={() => navigateSection('users')}
            />
            <SidebarItem
              active={section === 'approvals'}
              icon={<ShieldAlert className="w-4 h-4 text-amber-400" />}
              label="Approval & Konflik UUID"
              onClick={() => navigateSection('approvals')}
            />
            <SidebarItem
              active={section === 'settings'}
              icon={<Settings className="w-4 h-4 text-sky-400" />}
              label="Settings (API & Email)"
              onClick={() => navigateSection('settings')}
            />
            <button
              onClick={() => setShowAddHost(true)}
              className="w-full mt-2 px-3 py-2 rounded-lg bg-[#172824] hover:bg-[#1e352f] border border-[#13d389]/40 text-[#13d389] font-semibold flex items-center gap-2 transition-colors"
            >
              <Plus className="w-4 h-4" /> + Tambah Host Manual
            </button>
          </nav>
        </div>

        {/* Mini Fleet Health Meter ala Checkmk Sidebar */}
        <div className="p-3 m-2.5 rounded-xl bg-[#0c1216] border border-slate-800 space-y-2 text-[11px]">
          <div className="flex items-center justify-between text-slate-400">
            <span>Fleet Health</span>
            <span className="text-[#13d389] font-bold">{healthPct}% UP</span>
          </div>
          <div className="w-full h-2 bg-slate-800 rounded-full overflow-hidden">
            <div
              className="h-full bg-[#13d389]"
              style={{ width: `${healthPct}%` }}
            />
          </div>
          <div className="flex justify-between text-slate-400">
            <span>Armada: <b className="text-slate-200">{totalFleet}</b></span>
            <span>Masalah: <b className="text-rose-400">{totalProblems}</b></span>
          </div>
        </div>
      </aside>

      {/* MAIN WORKSPACE */}
      <div className="flex-1 min-w-0 flex flex-col">
        {/* Top Breadcrumb Bar ala Checkmk */}
        <header className="bg-[#11181c] border-b border-slate-800/90 px-5 py-3 flex flex-wrap items-center justify-between gap-3 sticky top-0 z-20">
          <div className="text-xs text-slate-400 flex items-center gap-1.5 flex-wrap">
            <button onClick={() => navigateSection('branches')} className="hover:text-[#13d389]">
              Monitor
            </button>
            <span>&gt;</span>
            <span className="text-slate-200 font-medium capitalize">{section}</span>
            {selectedHost && (
              <>
                <span>&gt;</span>
                <button
                  onClick={() => setSelectedService(null)}
                  className="text-[#13d389] hover:underline font-mono"
                >
                  {selectedHost.hostname}
                </button>
              </>
            )}
            {selectedService && (
              <>
                <span>&gt;</span>
                <span className="text-slate-100 font-mono">{selectedService.service_name}</span>
              </>
            )}
          </div>

          <div className="flex items-center gap-2 text-xs">
            <a
              href={reportUrls.hostsCsv()}
              className="px-3 py-1.5 rounded-lg bg-[#1b262c] hover:bg-[#24323a] border border-slate-700 inline-flex items-center gap-1.5"
            >
              <Download className="w-3.5 h-3.5 text-[#13d389]" /> Export CSV
            </a>
            <button
              onClick={loadData}
              className="p-2 rounded-lg bg-[#1b262c] hover:bg-[#24323a] border border-slate-700"
              title="Refresh Data"
            >
              <RefreshCw className={`w-3.5 h-3.5 ${loading ? 'animate-spin text-[#13d389]' : ''}`} />
            </button>
          </div>
        </header>

        <main className="p-5 flex-1 space-y-5 max-w-[1440px] w-full">
          {errorMsg && (
            <div className="bg-rose-950/60 border border-rose-500/40 rounded-xl p-4 flex items-center justify-between">
              <div className="flex items-center gap-3 text-sm text-rose-200">
                <AlertTriangle className="w-5 h-5 text-rose-400 shrink-0" />
                <span>{errorMsg}</span>
              </div>
              <button
                onClick={loadData}
                className="px-3 py-1 bg-rose-600 hover:bg-rose-500 rounded-lg text-xs font-semibold"
              >
                Coba Lagi
              </button>
            </div>
          )}

          {/* DRILL-DOWN LEVEL 3: DETAIL 1 SERVICE + GRAFIK */}
          {selectedHost && selectedService ? (
            <ServiceDetailView
              host={selectedHost}
              service={selectedService}
              onBack={() => setSelectedService(null)}
              onUpdated={loadData}
            />
          ) : selectedHost ? (
            /* DRILL-DOWN LEVEL 2: DAFTAR 1-PER-1 SERVICE MILIK HOST */
            <HostServicesView
              host={selectedHost}
              onBack={() => setSelectedHost(null)}
              onSelectService={(svc) => setSelectedService(svc)}
              onEditOwner={(h) => setEditingHost(h)}
              onHostChanged={loadData}
            />
          ) : (
            <>
              {/* VIEW 1: OVERVIEW CABANG */}
              {section === 'branches' && (
                <div className="space-y-4">
                  <div className="flex items-center justify-between">
                    <div>
                      <h2 className="text-lg font-bold">Ringkasan Kesehatan Lintas Cabang</h2>
                      <p className="text-xs text-slate-400">
                        Klik kartu cabang untuk menelusuri host &amp; cek 1-per-1 service di dalamnya.
                      </p>
                    </div>
                    <button
                      onClick={() => setShowAddBranch(true)}
                      className="px-3.5 py-2 rounded-lg bg-[#13d389] hover:bg-[#10b977] text-slate-950 text-xs font-bold inline-flex items-center gap-1.5"
                    >
                      <Plus className="w-4 h-4" /> + Tambah Cabang
                    </button>
                  </div>

                  <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
                    {branches.map((b) => (
                      <div
                        key={b.id}
                        className="bg-[#11181c] border border-slate-800 rounded-xl p-5 flex flex-col justify-between hover:border-slate-700"
                      >
                        <div>
                          <div className="flex items-start justify-between gap-2">
                            <div>
                              <span className="text-xs font-mono uppercase px-2 py-0.5 rounded bg-slate-800 text-[#13d389]">
                                {b.branch_code}
                              </span>
                              <h3 className="text-base font-bold mt-1">{b.branch_name}</h3>
                            </div>
                            <button
                              onClick={() => setEditingBranch(b)}
                              className="text-xs text-slate-300 hover:text-[#13d389] flex items-center gap-1 bg-slate-800/90 px-2.5 py-1 rounded-lg"
                            >
                              <Send className="w-3.5 h-3.5" />
                              {b.telegram_topic_id ? `Topic #${b.telegram_topic_id}` : 'Set Topic'}
                            </button>
                          </div>

                          <div className="grid grid-cols-4 gap-2 mt-4 text-center">
                            <div className="bg-emerald-950/40 border border-emerald-500/20 rounded-lg p-2">
                              <div className="text-lg font-bold text-[#13d389]">{b.up_count}</div>
                              <div className="text-[11px] text-slate-400">UP</div>
                            </div>
                            <div className="bg-amber-950/40 border border-amber-500/20 rounded-lg p-2">
                              <div className="text-lg font-bold text-amber-400">{b.warn_count}</div>
                              <div className="text-[11px] text-slate-400">WARN</div>
                            </div>
                            <div className="bg-rose-950/40 border border-rose-500/20 rounded-lg p-2">
                              <div className="text-lg font-bold text-rose-400">{b.crit_count}</div>
                              <div className="text-[11px] text-slate-400">CRIT</div>
                            </div>
                            <div className="bg-slate-800/80 border border-slate-700 rounded-lg p-2">
                              <div className="text-lg font-bold text-slate-300">{b.down_count}</div>
                              <div className="text-[11px] text-slate-400">DOWN</div>
                            </div>
                          </div>
                        </div>

                        <div className="mt-4 pt-3 border-t border-slate-800/80 flex items-center justify-between text-xs">
                          <span className="text-slate-400">Total: {b.total_hosts} perangkat</span>
                          <button
                            onClick={() => {
                              setSelectedBranch(b.branch_code);
                              setActionNeededOnly(false);
                              setPage(1);
                              setSection('hosts');
                            }}
                            className="text-[#13d389] hover:underline font-medium"
                          >
                            Buka All Hosts Cabang &rarr;
                          </button>
                        </div>
                      </div>
                    ))}
                  </div>
                </div>
              )}

              {/* VIEW 2 & 7: ALL HOSTS (50/HLM) + SERVICE STACKED BARS + DRILL-DOWN */}
              {(section === 'hosts' || section === 'approvals') && (
                <div className="space-y-4">
                  <div className="bg-[#11181c] border border-slate-800 rounded-xl p-4 flex flex-wrap items-center gap-3">
                    <div className="relative flex-1 min-w-[220px]">
                      <Search className="w-4 h-4 text-slate-400 absolute left-3 top-2.5" />
                      <input
                        type="text"
                        value={searchQuery}
                        onChange={(e) => setSearchQuery(e.target.value)}
                        onKeyDown={(e) => e.key === 'Enter' && loadData()}
                        placeholder="Cari Hostname, IP, Cabang, atau HostUUID..."
                        className="w-full pl-9 pr-3 py-1.5 bg-slate-950 border border-slate-800 rounded-lg text-sm focus:outline-none focus:border-[#13d389]"
                      />
                    </div>

                    <select
                      value={selectedBranch}
                      onChange={(e) => {
                        setSelectedBranch(e.target.value);
                        setPage(1);
                      }}
                      className="bg-slate-950 border border-slate-800 rounded-lg px-3 py-1.5 text-sm"
                    >
                      <option value="">Semua Cabang</option>
                      {branches.map((b) => (
                        <option key={b.id} value={b.branch_code}>
                          {b.branch_code} — {b.branch_name}
                        </option>
                      ))}
                    </select>

                    <select
                      value={selectedType}
                      onChange={(e) => {
                        setSelectedType(e.target.value);
                        setPage(1);
                      }}
                      className="bg-slate-950 border border-slate-800 rounded-lg px-3 py-1.5 text-sm"
                    >
                      <option value="">Semua Jenis Device</option>
                      {deviceTypes.map((dt) => (
                        <option key={dt.code} value={dt.code}>
                          {dt.label}
                        </option>
                      ))}
                    </select>

                    <label className="flex items-center gap-2 text-xs cursor-pointer bg-rose-950/40 border border-rose-500/30 px-3 py-1.5 rounded-lg text-rose-200">
                      <input
                        type="checkbox"
                        checked={actionNeededOnly}
                        onChange={(e) => {
                          setActionNeededOnly(e.target.checked);
                          setPage(1);
                        }}
                      />
                      Action Needed
                    </label>

                    <button
                      onClick={() => setShowAddHost(true)}
                      className="px-3 py-1.5 rounded-lg bg-[#13d389] text-slate-950 text-xs font-bold inline-flex items-center gap-1"
                    >
                      <Plus className="w-3.5 h-3.5" /> Tambah Host
                    </button>
                  </div>

                  <div className="bg-[#11181c] border border-slate-800 rounded-xl overflow-x-auto">
                    <table className="w-full text-left text-xs">
                      <thead className="bg-[#0e1418] text-slate-400 uppercase border-b border-slate-800">
                        <tr>
                          <th className="px-4 py-3">Status</th>
                          <th className="px-4 py-3">Cabang</th>
                          <th className="px-4 py-3">Hostname (Klik Cek 1-per-1)</th>
                          <th className="px-4 py-3">IP &amp; Tipe</th>
                          <th className="px-4 py-3 w-44">Services (OK / WA / CR)</th>
                          <th className="px-4 py-3">Pemilik (WA/Email)</th>
                          <th className="px-4 py-3 text-right">Aksi</th>
                        </tr>
                      </thead>
                      <tbody className="divide-y divide-slate-800/70">
                        {hostsPage?.items.length === 0 && (
                          <tr>
                            <td colSpan={7} className="px-4 py-8 text-center text-slate-500">
                              Tidak ada perangkat yang cocok dengan filter saat ini.
                            </td>
                          </tr>
                        )}
                        {hostsPage?.items.map((h) => {
                          const okN = h.ok_services ?? 0;
                          const warnN = h.warn_services ?? 0;
                          const critN = h.crit_services ?? 0;
                          const tot = Math.max(1, okN + warnN + critN);
                          return (
                            <tr key={h.host_id} className="hover:bg-[#1a252c]">
                              <td className="px-4 py-3">
                                <StatusBadge status={h.status} approval={h.approval_status} />
                              </td>
                              <td className="px-4 py-3 font-mono text-[#13d389]">{h.branch_code}</td>
                              <td className="px-4 py-3">
                                <button
                                  onClick={() => setSelectedHost(h)}
                                  className="font-bold text-sm text-slate-100 hover:text-[#13d389] hover:underline text-left"
                                >
                                  {h.hostname}
                                </button>
                                <div className="text-[11px] text-slate-500 font-mono">
                                  {h.host_uuid}
                                </div>
                              </td>
                              <td className="px-4 py-3">
                                <div className="font-mono">{h.ip_address}</div>
                                <div className="text-[11px] text-slate-400">{h.host_type}</div>
                              </td>
                              <td className="px-4 py-3">
                                {/* Checkmk-style stacked service counts */}
                                <button
                                  onClick={() => setSelectedHost(h)}
                                  className="w-full text-left space-y-1"
                                  title="Klik untuk cek 1-per-1 seluruh service host ini"
                                >
                                  <div className="flex h-4 w-full rounded overflow-hidden bg-slate-900 border border-slate-700 text-[10px] font-bold text-slate-950">
                                    {okN > 0 && (
                                      <div
                                        style={{ width: `${(okN / tot) * 100}%` }}
                                        className="bg-[#13d389] flex items-center justify-center"
                                      >
                                        {okN}
                                      </div>
                                    )}
                                    {warnN > 0 && (
                                      <div
                                        style={{ width: `${(warnN / tot) * 100}%` }}
                                        className="bg-amber-400 flex items-center justify-center"
                                      >
                                        {warnN}
                                      </div>
                                    )}
                                    {critN > 0 && (
                                      <div
                                        style={{ width: `${(critN / tot) * 100}%` }}
                                        className="bg-rose-500 text-white flex items-center justify-center"
                                      >
                                        {critN}
                                      </div>
                                    )}
                                  </div>
                                  {h.problem_services[0] && (
                                    <div className="text-[11px] text-amber-300 truncate max-w-[190px]">
                                      {h.problem_services[0]}
                                    </div>
                                  )}
                                </button>
                              </td>
                              <td className="px-4 py-3">
                                {h.owner_name ? (
                                  <div>
                                    <div className="font-medium text-slate-200">{h.owner_name}</div>
                                    <div className="text-slate-400">
                                      {h.owner_wa || '-'}
                                    </div>
                                  </div>
                                ) : (
                                  <span className="text-slate-500 italic">Belum diatur</span>
                                )}
                              </td>
                              <td className="px-4 py-3 text-right space-x-1.5 whitespace-nowrap">
                                <button
                                  onClick={() => setSelectedHost(h)}
                                  className="px-2.5 py-1 rounded bg-[#172824] border border-[#13d389]/40 text-[#13d389] font-medium"
                                >
                                  Cek Services
                                </button>
                                <button
                                  onClick={() => setEditingHost(h)}
                                  className="px-2.5 py-1 rounded bg-slate-800 hover:bg-slate-700 font-medium inline-flex items-center gap-1"
                                >
                                  <UserCheck className="w-3.5 h-3.5 text-emerald-400" /> Owner
                                </button>
                              </td>
                            </tr>
                          );
                        })}
                      </tbody>
                    </table>
                  </div>

                  {hostsPage && (
                    <div className="flex items-center justify-between text-xs text-slate-400">
                      <span>
                        Halaman <b>{hostsPage.page}</b> dari <b>{hostsPage.total_pages}</b> (Total{' '}
                        <b>{hostsPage.total_items}</b> perangkat)
                      </span>
                      <div className="flex gap-2">
                        <button
                          disabled={page <= 1}
                          onClick={() => setPage((p) => Math.max(1, p - 1))}
                          className="px-3 py-1.5 rounded bg-[#11181c] border border-slate-800 disabled:opacity-40"
                        >
                          &larr; Sebelumnya
                        </button>
                        <button
                          disabled={page >= hostsPage.total_pages}
                          onClick={() => setPage((p) => p + 1)}
                          className="px-3 py-1.5 rounded bg-[#11181c] border border-slate-800 disabled:opacity-40"
                        >
                          Berikutnya &rarr;
                        </button>
                      </div>
                    </div>
                  )}
                </div>
              )}

              {section === 'events' && <EventsAndOutboxView />}

              {section === 'inventory' && (
                <div className="space-y-4">
                  <div className="flex items-center justify-between">
                    <div>
                      <h2 className="text-lg font-bold">
                        Inventaris Kesehatan SSD/NVMe, Baterai PDA &amp; Lisensi OS/Office
                      </h2>
                      <p className="text-xs text-slate-400">
                        Diperbarui otomatis oleh Slow Tier Agent tiap 6 jam.
                      </p>
                    </div>
                    <a
                      href={reportUrls.inventoryCsv()}
                      className="px-4 py-2 rounded-lg bg-[#13d389] hover:bg-[#10b977] text-slate-950 text-xs font-bold inline-flex items-center gap-2"
                    >
                      <Download className="w-4 h-4" /> Export CSV Inventaris
                    </a>
                  </div>

                  <div className="bg-[#11181c] border border-slate-800 rounded-xl overflow-x-auto">
                    <table className="w-full text-left text-xs">
                      <thead className="bg-[#0e1418] text-slate-400 uppercase border-b border-slate-800">
                        <tr>
                          <th className="px-3 py-3">Cabang</th>
                          <th className="px-3 py-3">Hostname / IP</th>
                          <th className="px-3 py-3">OS &amp; Office</th>
                          <th className="px-3 py-3">CPU &amp; RAM</th>
                          <th className="px-3 py-3">Kesehatan Disk (SSD/NVMe)</th>
                          <th className="px-3 py-3">Baterai PDA</th>
                          <th className="px-3 py-3">AnyDesk / RustDesk</th>
                        </tr>
                      </thead>
                      <tbody className="divide-y divide-slate-800/70">
                        {inventory.map((item) => (
                          <tr key={item.host_id} className="hover:bg-[#1a252c]">
                            <td className="px-3 py-2.5 font-mono text-[#13d389]">{item.branch_code}</td>
                            <td className="px-3 py-2.5">
                              <div className="font-semibold text-slate-100">{item.hostname}</div>
                              <div className="text-slate-500 font-mono">{item.ip_address}</div>
                            </td>
                            <td className="px-3 py-2.5">
                              <div>{item.os_name || '-'}</div>
                              <div className="text-slate-400">{item.office_product || '-'}</div>
                            </td>
                            <td className="px-3 py-2.5">
                              <div>{item.cpu_model || '-'}</div>
                              <div className="text-slate-400">
                                RAM: {item.ram_total_gb ? `${item.ram_total_gb.toFixed(1)} GB` : '-'}
                              </div>
                            </td>
                            <td className="px-3 py-2.5">
                              <div>
                                {item.disk_primary_type || '-'} • {item.disk_primary_model || '-'}
                              </div>
                              <div className="text-[#13d389] font-semibold">
                                {item.disk_health_pct != null ? `Health ${item.disk_health_pct}%` : '-'}
                              </div>
                            </td>
                            <td className="px-3 py-2.5">
                              {item.battery_health_pct != null ? (
                                <div>
                                  <span className="font-semibold text-[#13d389]">
                                    {item.battery_health_pct.toFixed(1)}%
                                  </span>{' '}
                                  • Cycles: {item.battery_cycles ?? 0}
                                </div>
                              ) : (
                                <span className="text-slate-500">N/A (Desktop)</span>
                              )}
                            </td>
                            <td className="px-3 py-2.5 font-mono">
                              <div>AD: {item.remote_anydesk_id || '-'}</div>
                              <div>RD: {item.remote_rustdesk_id || '-'}</div>
                            </td>
                          </tr>
                        ))}
                      </tbody>
                    </table>
                  </div>
                </div>
              )}

              {section === 'reports' && (
                <ReportsCenterView onOpenInventory={() => setSection('inventory')} />
              )}

              {section === 'downloads' && <AgentDownloadsView branches={branches} />}

              {section === 'users' && <UsersManagementView branches={branches} />}

              {section === 'settings' && (
                <SettingsIntegrationView
                  deviceTypes={deviceTypes}
                  onDeviceTypeAdded={(dt) =>
                    setDeviceTypes((prev) =>
                      prev.some((p) => p.code === dt.code) ? prev : [...prev, dt]
                    )
                  }
                />
              )}
            </>
          )}
        </main>
      </div>

      {/* MODALS */}
      {editingHost && (
        <HostMetadataModal
          host={editingHost}
          onClose={() => setEditingHost(null)}
          onSaved={() => {
            setEditingHost(null);
            loadData();
          }}
        />
      )}
      {editingBranch && (
        <BranchTelegramModal
          branch={editingBranch}
          onClose={() => setEditingBranch(null)}
          onSaved={() => {
            setEditingBranch(null);
            loadData();
          }}
        />
      )}
      {showAddHost && (
        <AddHostManualModal
          branches={branches}
          deviceTypes={deviceTypes}
          onDeviceTypeAdded={(dt) =>
            setDeviceTypes((prev) =>
              prev.some((p) => p.code === dt.code) ? prev : [...prev, dt]
            )
          }
          onClose={() => setShowAddHost(false)}
          onSaved={() => {
            setShowAddHost(false);
            setSection('hosts');
            loadData();
          }}
        />
      )}
      {showAddBranch && (
        <AddBranchModal
          onClose={() => setShowAddBranch(false)}
          onSaved={() => {
            setShowAddBranch(false);
            loadData();
          }}
        />
      )}
    </div>
  );
}

function SidebarItem({
  active,
  icon,
  label,
  onClick,
}: {
  active: boolean;
  icon: React.ReactNode;
  label: string;
  onClick: () => void;
}) {
  return (
    <button
      onClick={onClick}
      className={`w-full px-3 py-2 rounded-lg flex items-center gap-2.5 text-left transition-colors ${
        active
          ? 'bg-[#172824] text-[#13d389] font-semibold border border-[#13d389]/30'
          : 'text-slate-300 hover:bg-slate-800/70'
      }`}
    >
      {icon}
      <span className="truncate">{label}</span>
    </button>
  );
}

function StatusBadge({ status, approval }: { status: string; approval: string }) {
  if (approval === 'CONFLICT' || approval === 'PENDING') {
    return (
      <span className="px-2 py-0.5 rounded text-[11px] font-bold bg-amber-950 border border-amber-500/40 text-amber-300 inline-flex items-center gap-1">
        <ShieldAlert className="w-3 h-3" /> {approval}
      </span>
    );
  }
  switch (status) {
    case 'UP':
      return (
        <span className="px-2 py-0.5 rounded text-[11px] font-bold bg-[#13d389] text-slate-950 inline-flex items-center gap-1">
          <CheckCircle2 className="w-3 h-3" /> UP
        </span>
      );
    case 'WARN':
      return (
        <span className="px-2 py-0.5 rounded text-[11px] font-bold bg-amber-400 text-slate-950 inline-flex items-center gap-1">
          <AlertTriangle className="w-3 h-3" /> WARN
        </span>
      );
    case 'CRIT':
    case 'DOWN':
      return (
        <span className="px-2 py-0.5 rounded text-[11px] font-bold bg-rose-500 text-white inline-flex items-center gap-1">
          <XCircle className="w-3 h-3" /> {status}
        </span>
      );
    default:
      return <span className="text-xs text-slate-400">{status}</span>;
  }
}

function HostMetadataModal({
  host,
  onClose,
  onSaved,
}: {
  host: HostRowDto;
  onClose: () => void;
  onSaved: () => void;
}) {
  const [ownerName, setOwnerName] = useState(host.owner_name || '');
  const [ownerWa, setOwnerWa] = useState(host.owner_wa || '');
  const [ownerEmail, setOwnerEmail] = useState(host.owner_email || '');
  const [alertServices, setAlertServices] = useState(
    host.user_alert_services || 'Health_Battery,Storage_Usage,Printer_Paper'
  );
  const [approvalStatus, setApprovalStatus] = useState<string>(host.approval_status);
  const [saving, setSaving] = useState(false);

  const save = async () => {
    setSaving(true);
    try {
      await apiFetch(`/api/v1/hosts/${host.host_id}/metadata`, {
        method: 'PATCH',
        body: JSON.stringify({
          owner_name: ownerName.trim() || null,
          owner_wa: ownerWa.trim() || null,
          owner_email: ownerEmail.trim() || null,
          user_alert_services: alertServices.trim(),
          approval_status: approvalStatus,
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
        <h3 className="text-base font-bold">
          Konfigurasi Perangkat: {host.hostname} ({host.branch_code})
        </h3>
        <div className="space-y-3 text-xs">
          <div>
            <label className="text-slate-400">Nama Pemilik / User Perangkat</label>
            <input
              value={ownerName}
              onChange={(e) => setOwnerName(e.target.value)}
              className="w-full mt-1 px-3 py-2 bg-slate-950 border border-slate-800 rounded-lg"
            />
          </div>
          <div>
            <label className="text-slate-400">Nomor WhatsApp User (628xxx)</label>
            <input
              value={ownerWa}
              onChange={(e) => setOwnerWa(e.target.value)}
              className="w-full mt-1 px-3 py-2 bg-slate-950 border border-slate-800 rounded-lg"
            />
          </div>
          <div>
            <label className="text-slate-400">Email User</label>
            <input
              value={ownerEmail}
              onChange={(e) => setOwnerEmail(e.target.value)}
              className="w-full mt-1 px-3 py-2 bg-slate-950 border border-slate-800 rounded-lg"
            />
          </div>
          <div>
            <label className="text-slate-400">Filter Service ke WA/Email User (CSV)</label>
            <input
              value={alertServices}
              onChange={(e) => setAlertServices(e.target.value)}
              className="w-full mt-1 px-3 py-2 bg-slate-950 border border-slate-800 rounded-lg font-mono"
            />
          </div>
          <div>
            <label className="text-slate-400">Status Approval Host</label>
            <select
              value={approvalStatus}
              onChange={(e) => setApprovalStatus(e.target.value)}
              className="w-full mt-1 px-3 py-2 bg-slate-950 border border-slate-800 rounded-lg"
            >
              <option value="APPROVED">APPROVED (Aktif)</option>
              <option value="PENDING">PENDING</option>
              <option value="CONFLICT">CONFLICT (Duplikat UUID)</option>
              <option value="REVOKED">REVOKED (Blokir Agen)</option>
            </select>
          </div>
        </div>
        <div className="flex justify-end gap-2 pt-2">
          <button onClick={onClose} className="px-4 py-2 rounded-lg bg-slate-800 text-xs">
            Batal
          </button>
          <button
            disabled={saving}
            onClick={save}
            className="px-4 py-2 rounded-lg bg-[#13d389] text-slate-950 text-xs font-bold"
          >
            {saving ? 'Menyimpan...' : 'Simpan Perubahan'}
          </button>
        </div>
      </div>
    </div>
  );
}

function BranchTelegramModal({
  branch,
  onClose,
  onSaved,
}: {
  branch: BranchSummaryCard;
  onClose: () => void;
  onSaved: () => void;
}) {
  const [chatId, setChatId] = useState(branch.telegram_chat_id || '');
  const [topicId, setTopicId] = useState(
    branch.telegram_topic_id != null ? String(branch.telegram_topic_id) : ''
  );
  const [saving, setSaving] = useState(false);

  const save = async () => {
    setSaving(true);
    try {
      await apiFetch(`/api/v1/branches/${branch.id}`, {
        method: 'PATCH',
        body: JSON.stringify({
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
        <h3 className="text-base font-bold">
          Routing Telegram IT Ops — {branch.branch_code} ({branch.branch_name})
        </h3>
        <div className="space-y-3 text-xs">
          <div>
            <label className="text-slate-400">Telegram Supergroup Chat ID</label>
            <input
              value={chatId}
              onChange={(e) => setChatId(e.target.value)}
              className="w-full mt-1 px-3 py-2 bg-slate-950 border border-slate-800 rounded-lg font-mono"
            />
          </div>
          <div>
            <label className="text-slate-400">Telegram Forum Topic ID (`message_thread_id`)</label>
            <input
              type="number"
              value={topicId}
              onChange={(e) => setTopicId(e.target.value)}
              className="w-full mt-1 px-3 py-2 bg-slate-950 border border-slate-800 rounded-lg font-mono"
            />
          </div>
        </div>
        <div className="flex justify-end gap-2 pt-2">
          <button onClick={onClose} className="px-4 py-2 rounded-lg bg-slate-800 text-xs">
            Batal
          </button>
          <button
            disabled={saving}
            onClick={save}
            className="px-4 py-2 rounded-lg bg-[#13d389] text-slate-950 text-xs font-bold"
          >
            Simpan Topik Telegram
          </button>
        </div>
      </div>
    </div>
  );
}
