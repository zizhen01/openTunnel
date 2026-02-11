use anyhow::Context;
use colored::Colorize;
use comfy_table::{presets::UTF8_FULL, Table};

use crate::error::Result;
use crate::i18n::lang;
use crate::t;

const METRICS_URL: &str = "http://127.0.0.1:20241/metrics";

/// Parsed Prometheus metrics from cloudflared.
#[derive(Debug, Default)]
pub struct TunnelMetrics {
    pub total_requests: Option<f64>,
    pub active_streams: Option<f64>,
    pub response_time_avg: Option<f64>,
    pub request_errors: Option<f64>,
    pub connections: Vec<ConnectionMetric>,
}

#[derive(Debug)]
pub struct ConnectionMetric {
    pub label: String,
    pub value: f64,
}

// ---------------------------------------------------------------------------
// Show stats (one-shot)
// ---------------------------------------------------------------------------

/// Fetch and display tunnel statistics.
pub async fn show_stats() -> Result<()> {
    let l = lang();
    println!(
        "\n{}",
        t!(l, "📊 Tunnel Statistics", "📊 隧道统计信息").bold()
    );

    let metrics = fetch_metrics().await?;

    let mut table = Table::new();
    table.load_preset(UTF8_FULL);
    table.set_header(vec![t!(l, "Metric", "指标"), t!(l, "Value", "值")]);

    table.add_row(vec![
        t!(l, "Total requests", "总请求数"),
        &format_metric(metrics.total_requests),
    ]);
    table.add_row(vec![
        t!(l, "Active streams", "活跃连接"),
        &format_metric(metrics.active_streams),
    ]);
    table.add_row(vec![
        t!(l, "Request errors", "请求错误"),
        &format_metric(metrics.request_errors),
    ]);

    if let Some(avg) = metrics.response_time_avg {
        table.add_row(vec![
            t!(l, "Avg response time", "平均响应时间"),
            &format!("{avg:.2}ms"),
        ]);
    }

    println!("{table}");

    if !metrics.connections.is_empty() {
        println!("\n{}", t!(l, "Connection details:", "连接详情:").bold());
        for conn in &metrics.connections {
            println!("  • {} = {}", conn.label, conn.value);
        }
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Real-time monitor
// ---------------------------------------------------------------------------

/// Continuously display metrics with a refresh interval.
pub async fn real_time_monitor() -> Result<()> {
    let l = lang();
    println!(
        "{}",
        t!(
            l,
            "📈 Real-time Monitor (press Ctrl+C to exit)",
            "📈 实时监控 (按 Ctrl+C 退出)"
        )
        .bold()
    );

    // Install a Ctrl+C handler so we can exit cleanly
    let running = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(true));
    let r = running.clone();
    ctrlc::set_handler(move || {
        r.store(false, std::sync::atomic::Ordering::SeqCst);
    })
    .context("failed to set Ctrl+C handler")?;

    while running.load(std::sync::atomic::Ordering::SeqCst) {
        // Clear screen
        print!("\x1B[2J\x1B[1;1H");

        println!(
            "{}\n",
            t!(
                l,
                "📈 Real-time Monitor (press Ctrl+C to exit)",
                "📈 实时监控 (按 Ctrl+C 退出)"
            )
            .bold()
        );

        match fetch_metrics().await {
            Ok(m) => print_compact_metrics(&m),
            Err(_) => {
                println!(
                    "{}",
                    t!(
                        l,
                        "⚠️  Cannot reach metrics endpoint. Is cloudflared running?",
                        "⚠️  无法连接指标端点。cloudflared 是否在运行?"
                    )
                    .yellow()
                );
            }
        }

        let ts = chrono::Local::now().format("%H:%M:%S");
        println!(
            "\n{} {}",
            t!(l, "Last update:", "上次更新:").dimmed(),
            ts.to_string().dimmed()
        );

        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
    }

    println!("\n{}", t!(l, "Monitor stopped.", "监控已停止。"));
    Ok(())
}

fn print_compact_metrics(m: &TunnelMetrics) {
    let l = lang();
    println!(
        "  {} {:>12}   {} {:>8}   {} {:>8}",
        t!(l, "Requests:", "请求数:").bold(),
        format_metric(m.total_requests).cyan(),
        t!(l, "Streams:", "连接:").bold(),
        format_metric(m.active_streams).green(),
        t!(l, "Errors:", "错误:").bold(),
        format_metric(m.request_errors).normal().red()
    );
}

// ---------------------------------------------------------------------------
// Fetch & parse Prometheus metrics
// ---------------------------------------------------------------------------

async fn fetch_metrics() -> Result<TunnelMetrics> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()?;

    let body = client
        .get(METRICS_URL)
        .send()
        .await
        .context("failed to reach cloudflared metrics endpoint")?
        .text()
        .await?;

    Ok(parse_prometheus(&body))
}

fn parse_prometheus(body: &str) -> TunnelMetrics {
    let mut m = TunnelMetrics::default();

    for line in body.lines() {
        if line.starts_with('#') {
            continue;
        }
        if let Some(val) = extract_metric(line, "cloudflared_tunnel_total_requests") {
            m.total_requests = Some(m.total_requests.unwrap_or(0.0) + val);
        } else if let Some(val) = extract_metric(line, "cloudflared_tunnel_active_streams") {
            m.active_streams = Some(m.active_streams.unwrap_or(0.0) + val);
        } else if let Some(val) = extract_metric(line, "cloudflared_tunnel_request_errors") {
            m.request_errors = Some(m.request_errors.unwrap_or(0.0) + val);
        } else if let Some(val) = extract_metric(line, "cloudflared_tunnel_response_by_code") {
            // Track per-code responses as connection metrics
            if let Some(label) = line.split('{').nth(1).and_then(|s| s.split('}').next()) {
                m.connections.push(ConnectionMetric {
                    label: label.to_string(),
                    value: val,
                });
            }
        }
    }

    m
}

fn extract_metric(line: &str, prefix: &str) -> Option<f64> {
    if line.starts_with(prefix) {
        // Format: metric_name{labels} value  OR  metric_name value
        line.split_whitespace().last()?.parse().ok()
    } else {
        None
    }
}

fn format_metric(val: Option<f64>) -> String {
    match val {
        Some(v) if v >= 1_000_000.0 => format!("{:.1}M", v / 1_000_000.0),
        Some(v) if v >= 1_000.0 => format!("{:.1}K", v / 1_000.0),
        Some(v) => format!("{v:.0}"),
        None => "-".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_prometheus_metrics() {
        let input = r#"# HELP cloudflared_tunnel_total_requests Total number of requests
# TYPE cloudflared_tunnel_total_requests counter
cloudflared_tunnel_total_requests 12345
cloudflared_tunnel_active_streams 42
cloudflared_tunnel_request_errors 3
"#;
        let m = parse_prometheus(input);
        assert_eq!(m.total_requests, Some(12345.0));
        assert_eq!(m.active_streams, Some(42.0));
        assert_eq!(m.request_errors, Some(3.0));
    }

    #[test]
    fn format_metric_values() {
        assert_eq!(format_metric(Some(500.0)), "500");
        assert_eq!(format_metric(Some(1500.0)), "1.5K");
        assert_eq!(format_metric(Some(2_500_000.0)), "2.5M");
        assert_eq!(format_metric(None), "-");
    }
}
