use crate::model::{MetricTier, ServiceCheckPayload};

/// Membentuk teks standar Checkmk (`<<<check_mk>>>` dan `<<<local:sep(0)>>>`)
pub fn format_checkmk_output(
    version: &str,
    agent_os: &str,
    hostname: &str,
    checks: &[ServiceCheckPayload],
) -> String {
    let mut out = String::with_capacity(1024 + checks.len() * 128);
    out.push_str("<<<check_mk>>>\n");
    out.push_str(&format!("Version: {}\n", version));
    out.push_str(&format!("AgentOS: {}\n", agent_os));
    out.push_str(&format!("Hostname: {}\n", hostname));
    out.push_str("<<<local:sep(0)>>>\n");

    for c in checks {
        let clean_name: String = c
            .service_name
            .chars()
            .map(|ch| if ch.is_whitespace() { '_' } else { ch })
            .collect();
        let metrics = if c.metrics.trim().is_empty() {
            "-"
        } else {
            c.metrics.trim()
        };
        let summary = c.summary.replace('\n', " ").replace('\r', "");
        out.push_str(&format!(
            "{} \"{}\" {} {}\n",
            c.state.min(3),
            clean_name,
            metrics,
            summary.trim()
        ));
    }
    out
}

/// Mem-parsing baris-baris output `<<<local>>>` dari skrip lokal atau agen legacy TCP/6556
pub fn parse_checkmk_local_lines(raw_text: &str, default_tier: MetricTier) -> Vec<ServiceCheckPayload> {
    let mut results = Vec::new();
    let mut in_local = false;

    for raw_line in raw_text.lines() {
        let line = raw_line.trim();
        if line.is_empty() {
            continue;
        }
        if line.starts_with("<<<") && line.ends_with(">>>") {
            in_local = line.starts_with("<<<local");
            continue;
        }
        // Jika teks berasal dari skrip lokal langsung (tanpa header <<<local>>>), tetap dukung:
        if !in_local && raw_text.contains("<<<") {
            continue;
        }

        let mut rest = line;
        // Lewati prefix cached(...) jika ada
        if rest.starts_with("cached(") {
            if let Some(idx) = rest.find(')') {
                rest = rest[idx + 1..].trim_start();
            }
        }

        let mut parts = rest.splitn(2, char::is_whitespace);
        let Some(state_tok) = parts.next() else {
            continue;
        };
        let Ok(state) = state_tok.parse::<u8>() else {
            continue;
        };
        let Some(after_state) = parts.next() else {
            continue;
        };
        let after_state = after_state.trim_start();

        let (service_name, after_name) = if let Some(stripped) = after_state.strip_prefix('"') {
            if let Some(end_q) = stripped.find('"') {
                (&stripped[..end_q], stripped[end_q + 1..].trim_start())
            } else {
                continue;
            }
        } else {
            let mut sp = after_state.splitn(2, char::is_whitespace);
            let Some(name) = sp.next() else {
                continue;
            };
            (name, sp.next().unwrap_or("").trim_start())
        };

        let mut sp2 = after_name.splitn(2, char::is_whitespace);
        let metrics = sp2.next().unwrap_or("-").to_string();
        let summary = sp2.next().unwrap_or("OK").trim().to_string();

        results.push(ServiceCheckPayload {
            service_name: service_name.to_string(),
            tier: default_tier,
            state: state.min(3),
            summary,
            metrics,
        });
    }

    results
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_roundtrip_checkmk_local() {
        let checks = vec![ServiceCheckPayload {
            service_name: "Health_Battery".to_string(),
            tier: MetricTier::Fast,
            state: 0,
            summary: "Status : OK | Level : 92%".to_string(),
            metrics: "battery_level=92%".to_string(),
        }];
        let formatted = format_checkmk_output("2.0.0", "linux", "PDA-01", &checks);
        let parsed = parse_checkmk_local_lines(&formatted, MetricTier::Fast);
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].service_name, "Health_Battery");
        assert_eq!(parsed[0].state, 0);
        assert_eq!(parsed[0].metrics, "battery_level=92%");
    }
}
