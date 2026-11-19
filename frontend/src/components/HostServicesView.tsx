import { useEffect, useState } from 'react';
import {
  ArrowLeft,
  Check,
  Clock,
  Download,
  RefreshCw,
  ShieldCheck,
  UserCheck,
  Wrench,
} from 'lucide-react';
import { apiFetch, HostRowDto, reportUrls, ServiceRowDto } from '../api/client';

interface HostServicesViewProps {
  host: HostRowDto;
  onBack: () => void;
  onSelectService: (svc: ServiceRowDto) => void;
  onEditOwner: (host: HostRowDto) => void;
  onHostChanged: () => void;
}

function parsePerfMeter(metrics: string): { pct: number; label: string } | null {
  if (!metrics || metrics === '-') return null;
  const first = metrics.split('|')[0];
  const eqIdx = first.indexOf('=');
  if (eqIdx === -1) return null;
  const rawVal = first.slice(eqIdx + 1).split(';')[0].trim();
  const num = parseFloat(rawVal);
  if (Number.isNaN(num)) return null;
  if (rawVal.endsWith('%')) {
    return { pct: Math.min(100, Math.max(0, num)), label: rawVal };
  }
  if (rawVal.endsWith('ms')) {
    return { pct: Math.min(100, Math.max(5, (num / 150) * 100)), label: rawVal };
  }
  return { pct: Math.min(100, Math.max(4, num)), label: rawVal };
}

function formatRelSec(epoch: number): string {
  const diff = Math.max(0, Math.floor(Date.now() / 1000) - epoch);
  if (diff < 60) return `${diff} s`;
  if (diff < 3600) return `${Math.floor(diff / 60)} m`;
  if (diff < 86400) return `${Math.floor(diff / 3600)} h`;
  return `${Math.floor(diff / 86400)} d`;
}

export function HostServicesView({
  host,
  onBack,
  onSelectService,
  onEditOwner,
  onHostChanged,
}: HostServicesViewProps) {
  const [services, setServices] = useState<ServiceRowDto[]>([]);
  const [loading, setLoading] = useState(true);
  const [filterText, setFilterText] = useState('');
  const [inDowntime, setInDowntime] = useState(Boolean(host.is_in_downtime));
  const [busyMsg, setBusyMsg] = useState<string | null>(null);

  const loadServices = async () => {
    setLoading(true);
    try {
      const list = await apiFetch<ServiceRowDto[]>(`/api/v1/hosts/${host.host_id}/services`);
      setServices(list);
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    loadServices();
  }, [host.host_id]);

  const toggleDowntime = async () => {
    const next = !inDowntime;
    await apiFetch(`/api/v1/hosts/${host.host_id}/downtime`, {
      method: 'POST',
      body: JSON.stringify({ in_downtime: next }),
    });
    setInDowntime(next);
    setBusyMsg(next ? 'Scheduled downtime diaktifkan (alert diredam).' : 'Downtime dimatikan.');
    onHostChanged();
  };

  const acknowledgeAllProblems = async () => {
    const problems = services.filter((s) => s.state > 0 && !s.is_acknowledged);
    for (const s of problems) {
      await apiFetch(
        `/api/v1/hosts/${host.host_id}/services/${encodeURIComponent(s.service_name)}/ack`,
        {
          method: 'POST',
          body: JSON.stringify({ acknowledged: true }),
        }
      );
    }
    setBusyMsg(
      problems.length > 0
        ? `${problems.length} service bermasalah berhasil di-Acknowledge.`
        : 'Tidak ada masalah baru yang perlu di-Acknowledge.'
    );
    await loadServices();
  };

  const visibleServices = services.filter((s) =>
    s.service_name.toLowerCase().includes(filterText.toLowerCase()) ||
    s.summary.toLowerCase().includes(filterText.toLowerCase())
  );

  return (
    <div className="space-y-4">
      {/* Header & Command Toolbar ala Checkmk "Services of host" */}
      <div className="bg-[#11181c] border border-slate-800 rounded-xl p-4 space-y-3">
        <div className="flex flex-wrap items-center justify-between gap-3">
          <div className="flex items-center gap-3">
            <button
              onClick={onBack}
              className="px-3 py-1.5 rounded-lg bg-slate-800 hover:bg-slate-700 text-xs font-medium inline-flex items-center gap-1.5"
            >
              <ArrowLeft className="w-3.5 h-3.5" /> All Hosts
            </button>
            <div>
              <h2 className="text-lg font-bold flex items-center gap-2">
                Services of host <span className="text-[#16cb7f] font-mono">{host.hostname}</span>
              </h2>
              <p className="text-xs text-slate-400">
                Cabang: <b className="text-slate-200">{host.branch_code}</b> • IP:{' '}
                <span className="font-mono text-slate-200">{host.ip_address}</span> • UUID:{' '}
                <span className="font-mono">{host.host_uuid}</span> • Tipe: {host.host_type}
              </p>
            </div>
          </div>

          <div className="text-xs text-right">
            <div className="text-slate-400">
              Pemilik: <b className="text-slate-200">{host.owner_name || 'Belum diatur'}</b>
            </div>
            <div className="text-slate-400">
              WA: {host.owner_wa || '-'} • Email: {host.owner_email || '-'}
            </div>
          </div>
        </div>

        {/* Action Buttons Checkmk Style */}
        <div className="flex flex-wrap items-center gap-2 pt-2 border-t border-slate-800/80">
          <button
            onClick={acknowledgeAllProblems}
            className="px-3 py-1.5 rounded bg-[#1b262c] hover:bg-[#24323a] border border-slate-700 text-xs font-medium inline-flex items-center gap-1.5"
          >
            <Wrench className="w-3.5 h-3.5 text-amber-400" /> Acknowledge problems
          </button>
          <button
            onClick={toggleDowntime}
            className={`px-3 py-1.5 rounded border text-xs font-medium inline-flex items-center gap-1.5 ${
              inDowntime
                ? 'bg-sky-950 border-sky-500/50 text-sky-300'
                : 'bg-[#1b262c] hover:bg-[#24323a] border-slate-700'
            }`}
          >
            <Clock className="w-3.5 h-3.5 text-sky-400" />
            {inDowntime ? 'In Downtime (Aktif)' : 'Schedule downtimes'}
          </button>
          <button
            onClick={loadServices}
            className="px-3 py-1.5 rounded bg-[#1b262c] hover:bg-[#24323a] border border-slate-700 text-xs font-medium inline-flex items-center gap-1.5"
          >
            <RefreshCw className={`w-3.5 h-3.5 text-[#16cb7f] ${loading ? 'animate-spin' : ''}`} />
            Reschedule / Refresh
          </button>
          <button
            onClick={() => onEditOwner(host)}
            className="px-3 py-1.5 rounded bg-[#1b262c] hover:bg-[#24323a] border border-slate-700 text-xs font-medium inline-flex items-center gap-1.5"
          >
            <UserCheck className="w-3.5 h-3.5 text-emerald-400" /> Atur Owner WA/Email
          </button>
          <a
            href={reportUrls.hostServicesCsv(host.host_id)}
            className="px-3 py-1.5 rounded bg-[#1b262c] hover:bg-[#24323a] border border-slate-700 text-xs font-medium inline-flex items-center gap-1.5"
          >
            <Download className="w-3.5 h-3.5 text-emerald-400" /> Export Services CSV
          </a>

          <input
            type="text"
            value={filterText}
            onChange={(e) => setFilterText(e.target.value)}
            placeholder="Filter nama service..."
            className="ml-auto px-3 py-1 rounded bg-slate-950 border border-slate-800 text-xs focus:outline-none focus:border-[#16cb7f]"
          />
        </div>

        {busyMsg && (
          <div className="text-xs px-3 py-1.5 rounded bg-emerald-950/60 border border-emerald-500/30 text-emerald-300">
            {busyMsg}
          </div>
        )}
      </div>

      {/* Tabel 1-per-1 Services ala Checkmk */}
      <div className="bg-[#11181c] border border-slate-800 rounded-xl overflow-x-auto">
        <div className="px-4 py-2 bg-[#162026] border-b border-slate-800 flex items-center justify-between text-xs text-slate-400">
          <span className="font-semibold text-slate-200">{host.hostname}</span>
          <span>{visibleServices.length} rows</span>
        </div>

        <table className="w-full text-left text-xs">
          <thead className="bg-[#0e1418] text-slate-400 border-b border-slate-800">
            <tr>
              <th className="px-3 py-2.5 w-20">State</th>
              <th className="px-3 py-2.5">Service</th>
              <th className="px-3 py-2.5 w-16 text-center">Icons</th>
              <th className="px-3 py-2.5">Summary</th>
              <th className="px-3 py-2.5 w-20">Tier</th>
              <th className="px-3 py-2.5 w-20">Age</th>
              <th className="px-3 py-2.5 w-20">Checked</th>
              <th className="px-3 py-2.5 w-44">Perf-O-Meter</th>
            </tr>
          </thead>
          <tbody className="divide-y divide-slate-800/70">
            {visibleServices.map((svc) => {
              const meter = parsePerfMeter(svc.metrics);
              return (
                <tr
                  key={svc.service_name}
                  onClick={() => onSelectService(svc)}
                  className="hover:bg-[#1a252c] cursor-pointer transition-colors"
                >
                  <td className="px-3 py-2">
                    <ServiceStatePill state={svc.state} />
                  </td>
                  <td className="px-3 py-2 font-semibold text-slate-100 hover:text-[#16cb7f]">
                    {svc.service_name}
                  </td>
                  <td className="px-3 py-2 text-center">
                    {svc.is_acknowledged && (
                      <span
                        title="Problem Acknowledged"
                        className="inline-flex items-center px-1.5 py-0.5 rounded bg-sky-950 text-sky-300 border border-sky-500/30"
                      >
                        <Check className="w-3 h-3" />
                      </span>
                    )}
                    {svc.state === 0 && !svc.is_acknowledged && (
                      <ShieldCheck className="w-3.5 h-3.5 text-slate-600 inline" />
                    )}
                  </td>
                  <td className="px-3 py-2 text-slate-300">{svc.summary}</td>
                  <td className="px-3 py-2 font-mono text-[11px] text-slate-400">{svc.tier}</td>
                  <td className="px-3 py-2 text-slate-400">
                    {formatRelSec(svc.last_state_change_epoch)}
                  </td>
                  <td className="px-3 py-2 text-slate-400">
                    {formatRelSec(svc.last_check_epoch)}
                  </td>
                  <td className="px-3 py-2">
                    {meter ? (
                      <div className="w-full h-5 bg-slate-950 border border-slate-700 rounded overflow-hidden relative flex items-center justify-center">
                        <div
                          className={`h-full absolute left-0 top-0 ${
                            svc.state >= 2
                              ? 'bg-rose-500/70'
                              : svc.state === 1
                              ? 'bg-amber-500/70'
                              : 'bg-sky-500/50'
                          }`}
                          style={{ width: `${meter.pct}%` }}
                        />
                        <span className="relative z-10 text-[11px] font-mono text-slate-100">
                          {meter.label}
                        </span>
                      </div>
                    ) : (
                      <span className="text-slate-600">-</span>
                    )}
                  </td>
                </tr>
              );
            })}
          </tbody>
        </table>
      </div>
    </div>
  );
}

export function ServiceStatePill({ state }: { state: number }) {
  if (state === 0) {
    return (
      <span className="inline-block w-14 text-center py-0.5 rounded text-[11px] font-bold bg-[#13d389] text-slate-950">
        OK
      </span>
    );
  }
  if (state === 1) {
    return (
      <span className="inline-block w-14 text-center py-0.5 rounded text-[11px] font-bold bg-amber-400 text-slate-950">
        WARN
      </span>
    );
  }
  return (
    <span className="inline-block w-14 text-center py-0.5 rounded text-[11px] font-bold bg-rose-500 text-white">
      CRIT
    </span>
  );
}
