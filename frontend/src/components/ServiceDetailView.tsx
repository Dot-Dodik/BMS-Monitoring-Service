import { useMemo, useState } from 'react';
import { ArrowLeft, CheckCircle2, Clock, Wrench } from 'lucide-react';
import { apiFetch, HostRowDto, ServiceRowDto } from '../api/client';

interface ServiceDetailViewProps {
  host: HostRowDto;
  service: ServiceRowDto;
  onBack: () => void;
  onUpdated: () => void;
}

type TimeRange = '4h' | '25h' | '8d' | '35d' | '400d';

const RANGE_LABELS: { id: TimeRange; label: string; stepMin: number }[] = [
  { id: '4h', label: 'The last 4 hours', stepMin: 5 },
  { id: '25h', label: 'The last 25 hours', stepMin: 30 },
  { id: '8d', label: 'The last 8 days', stepMin: 180 },
  { id: '35d', label: 'The last 35 days', stepMin: 720 },
  { id: '400d', label: 'The last 400 days', stepMin: 7200 },
];

export function ServiceDetailView({
  host,
  service,
  onBack,
  onUpdated,
}: ServiceDetailViewProps) {
  const [range, setRange] = useState<TimeRange>('4h');
  const [ack, setAck] = useState(service.is_acknowledged);
  const [msg, setMsg] = useState<string | null>(null);

  const toggleAck = async () => {
    const next = !ack;
    await apiFetch(
      `/api/v1/hosts/${host.host_id}/services/${encodeURIComponent(service.service_name)}/ack`,
      {
        method: 'POST',
        body: JSON.stringify({ acknowledged: next }),
      }
    );
    setAck(next);
    setMsg(next ? 'Service ditandai Acknowledged.' : 'Acknowledge dibatalkan.');
    onUpdated();
  };

  // Bangkitkan titik deret waktu dari metrik aktual service untuk preview grafik Checkmk-style
  const series = useMemo(() => {
    const baseMatch = service.metrics.match(/=([0-9.]+)/);
    const baseVal = baseMatch ? parseFloat(baseMatch[1]) : service.state === 0 ? 12 : 88;
    const pts: { label: string; value: number }[] = [];
    const count = 48;
    const seed = service.service_name
      .split('')
      .reduce((acc, ch) => acc + ch.charCodeAt(0), 0);
    const spanMul = range === '4h' ? 0.12 : range === '25h' ? 0.2 : 0.3;

    for (let i = 0; i < count; i++) {
      const wave = Math.sin((i + seed) * 0.37) * baseVal * spanMul;
      const jitter = Math.cos((i * 2 + seed) * 0.19) * baseVal * 0.06;
      const val = i === count - 1 ? baseVal : Math.max(0, Number((baseVal + wave + jitter).toFixed(2)));
      pts.push({ label: `T-${count - 1 - i}`, value: val });
    }
    return pts;
  }, [service.metrics, service.service_name, service.state, range]);

  const minVal = Math.min(...series.map((s) => s.value));
  const maxVal = Math.max(...series.map((s) => s.value));
  const avgVal = series.reduce((a, b) => a + b.value, 0) / series.length;
  const lastVal = series[series.length - 1]?.value ?? 0;

  // Koordinat SVG 640 x 190
  const svgW = 640;
  const svgH = 190;
  const padL = 42;
  const padR = 14;
  const padT = 16;
  const padB = 24;
  const plotW = svgW - padL - padR;
  const plotH = svgH - padT - padB;
  const yMax = Math.max(10, Math.ceil(maxVal * 1.2));

  const polylinePoints = series
    .map((pt, idx) => {
      const x = padL + (idx / (series.length - 1)) * plotW;
      const y = padT + plotH - (pt.value / yMax) * plotH;
      return `${x.toFixed(1)},${y.toFixed(1)}`;
    })
    .join(' ');

  const areaPoints = `${padL},${padT + plotH} ${polylinePoints} ${padL + plotW},${padT + plotH}`;

  return (
    <div className="space-y-4">
      <div className="bg-[#11181c] border border-slate-800 rounded-xl p-4 space-y-3">
        <div className="flex flex-wrap items-center justify-between gap-3">
          <div className="flex items-center gap-3">
            <button
              onClick={onBack}
              className="px-3 py-1.5 rounded-lg bg-slate-800 hover:bg-slate-700 text-xs font-medium inline-flex items-center gap-1.5"
            >
              <ArrowLeft className="w-3.5 h-3.5" /> Services of {host.hostname}
            </button>
            <h2 className="text-lg font-bold">
              Service <span className="text-[#16cb7f]">{service.service_name}</span>, {host.hostname}
            </h2>
          </div>

          <div className="flex items-center gap-2">
            <button
              onClick={toggleAck}
              className={`px-3 py-1.5 rounded border text-xs font-medium inline-flex items-center gap-1.5 ${
                ack
                  ? 'bg-sky-950 border-sky-500/50 text-sky-300'
                  : 'bg-[#1b262c] hover:bg-[#24323a] border-slate-700'
              }`}
            >
              <Wrench className="w-3.5 h-3.5 text-amber-400" />
              {ack ? 'Acknowledged' : 'Acknowledge problems'}
            </button>
            <span className="px-3 py-1.5 rounded bg-[#1b262c] border border-slate-700 text-xs inline-flex items-center gap-1.5 text-slate-300">
              <Clock className="w-3.5 h-3.5 text-sky-400" />
              Downtime: {host.is_in_downtime ? 'YES' : 'NO'}
            </span>
          </div>
        </div>

        {msg && (
          <div className="text-xs px-3 py-1.5 rounded bg-emerald-950/60 border border-emerald-500/30 text-emerald-300">
            {msg}
          </div>
        )}
      </div>

      {/* Tabel Detail Checkmk ala Screenshot ke-4 */}
      <div className="bg-[#11181c] border border-slate-800 rounded-xl overflow-hidden text-xs">
        <table className="w-full">
          <tbody className="divide-y divide-slate-800/80">
            <tr>
              <td className="px-4 py-2 w-56 text-slate-400 bg-[#0d1317]">Branch / Site</td>
              <td className="px-4 py-2 font-mono">{host.branch_code}</td>
            </tr>
            <tr>
              <td className="px-4 py-2 text-slate-400 bg-[#0d1317]">Host name</td>
              <td className="px-4 py-2 font-semibold">{host.hostname} ({host.ip_address})</td>
            </tr>
            <tr>
              <td className="px-4 py-2 text-slate-400 bg-[#0d1317]">Service name</td>
              <td className="px-4 py-2 font-semibold">{service.service_name}</td>
            </tr>
            <tr>
              <td className="px-4 py-2 text-slate-400 bg-[#0d1317]">Service state</td>
              <td className="px-4 py-1.5">
                <div
                  className={`w-full py-1 text-center font-bold rounded text-xs ${
                    service.state === 0
                      ? 'bg-[#13d389] text-slate-950'
                      : service.state === 1
                      ? 'bg-amber-400 text-slate-950'
                      : 'bg-rose-500 text-white'
                  }`}
                >
                  {service.state === 0 ? 'OK' : service.state === 1 ? 'WARN' : 'CRIT'}
                </div>
              </td>
            </tr>
            <tr>
              <td className="px-4 py-2 text-slate-400 bg-[#0d1317]">Summary</td>
              <td className="px-4 py-2 text-slate-200">{service.summary}</td>
            </tr>
            <tr>
              <td className="px-4 py-2 text-slate-400 bg-[#0d1317]">Service metrics</td>
              <td className="px-4 py-2 font-mono text-emerald-300">{service.metrics}</td>
            </tr>
            <tr>
              <td className="px-4 py-2 text-slate-400 bg-[#0d1317] align-top">Service graphs</td>
              <td className="px-4 py-3">
                <div className="flex flex-col lg:flex-row gap-4">
                  {/* SVG Chart */}
                  <div className="flex-1 bg-[#0b1014] border border-slate-800 rounded-lg p-3">
                    <div className="flex items-center justify-between text-[11px] text-slate-300 mb-2">
                      <span className="font-semibold">
                        {service.service_name} ({RANGE_LABELS.find((r) => r.id === range)?.label})
                      </span>
                      <span className="font-mono text-slate-400">
                        Tier: {service.tier}
                      </span>
                    </div>
                    <svg viewBox={`0 0 ${svgW} ${svgH}`} className="w-full h-48">
                      {[0, 0.25, 0.5, 0.75, 1].map((frac) => {
                        const y = padT + plotH * (1 - frac);
                        const labelVal = (yMax * frac).toFixed(0);
                        return (
                          <g key={frac}>
                            <line
                              x1={padL}
                              y1={y}
                              x2={padL + plotW}
                              y2={y}
                              stroke="#1e293b"
                              strokeDasharray="3,3"
                            />
                            <text
                              x={padL - 6}
                              y={y + 4}
                              textAnchor="end"
                              fill="#64748b"
                              fontSize="10"
                            >
                              {labelVal}
                            </text>
                          </g>
                        );
                      })}
                      <polygon points={areaPoints} fill="rgba(19, 211, 137, 0.16)" />
                      <polyline
                        fill="none"
                        stroke="#13d389"
                        strokeWidth="2"
                        points={polylinePoints}
                      />
                    </svg>
                    <div className="grid grid-cols-4 gap-2 pt-2 border-t border-slate-800/70 text-[11px] font-mono">
                      <div>
                        <span className="text-slate-500">Minimum:</span>{' '}
                        <b className="text-slate-200">{minVal.toFixed(2)}</b>
                      </div>
                      <div>
                        <span className="text-slate-500">Maximum:</span>{' '}
                        <b className="text-slate-200">{maxVal.toFixed(2)}</b>
                      </div>
                      <div>
                        <span className="text-slate-500">Average:</span>{' '}
                        <b className="text-slate-200">{avgVal.toFixed(2)}</b>
                      </div>
                      <div>
                        <span className="text-slate-500">Last:</span>{' '}
                        <b className="text-[#13d389]">{lastVal.toFixed(2)}</b>
                      </div>
                    </div>
                  </div>

                  {/* Time Range Switcher Cards ala Checkmk */}
                  <div className="w-full lg:w-52 flex flex-col gap-2">
                    {RANGE_LABELS.map((r) => (
                      <button
                        key={r.id}
                        onClick={() => setRange(r.id)}
                        className={`px-3 py-2.5 rounded border text-left transition ${
                          range === r.id
                            ? 'bg-[#172824] border-[#13d389] text-[#13d389] font-semibold'
                            : 'bg-[#0b1014] border-slate-800 text-slate-300 hover:border-slate-700'
                        }`}
                      >
                        {r.label}
                      </button>
                    ))}
                  </div>
                </div>
              </td>
            </tr>
            <tr>
              <td className="px-4 py-2 text-slate-400 bg-[#0d1317]">Currently in downtime</td>
              <td className="px-4 py-2">{host.is_in_downtime ? 'yes' : 'no'}</td>
            </tr>
            <tr>
              <td className="px-4 py-2 text-slate-400 bg-[#0d1317]">Current check attempt</td>
              <td className="px-4 py-2 font-mono">1/1</td>
            </tr>
            <tr>
              <td className="px-4 py-2 text-slate-400 bg-[#0d1317]">Acknowledged</td>
              <td className="px-4 py-2 inline-flex items-center gap-1.5">
                {ack ? (
                  <>
                    <CheckCircle2 className="w-3.5 h-3.5 text-sky-400" /> Yes
                  </>
                ) : (
                  'No'
                )}
              </td>
            </tr>
          </tbody>
        </table>
      </div>
    </div>
  );
}
