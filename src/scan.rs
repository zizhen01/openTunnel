use colored::Colorize;
use comfy_table::{presets::UTF8_FULL, Table};
use tokio::net::TcpStream;
use tokio::time::{timeout, Duration};

use crate::config::{load_tunnel_config, save_tunnel_config, IngressRule};
use crate::error::Result;
use crate::i18n::lang;
use crate::prompt;
use crate::t;

/// Well-known development ports and their descriptions.
const DEFAULT_PORTS: &[(u16, &str, &str)] = &[
    (80, "HTTP", "HTTP"),
    (443, "HTTPS", "HTTPS"),
    (3000, "React / Node.js", "React / Node.js"),
    (3001, "React Dev", "React Dev"),
    (4000, "GraphQL / Phoenix", "GraphQL / Phoenix"),
    (4200, "Angular", "Angular"),
    (5000, "Flask / Python", "Flask / Python"),
    (5173, "Vite", "Vite"),
    (5432, "PostgreSQL", "PostgreSQL"),
    (6379, "Redis", "Redis"),
    (8000, "Django / Uvicorn", "Django / Uvicorn"),
    (8080, "HTTP Alternate", "HTTP Alternate"),
    (8443, "HTTPS Alternate", "HTTPS Alternate"),
    (8888, "Jupyter", "Jupyter"),
    (9000, "PHP-FPM / SonarQube", "PHP-FPM / SonarQube"),
    (9090, "Prometheus", "Prometheus"),
    (27017, "MongoDB", "MongoDB"),
];

#[derive(Debug)]
struct DiscoveredService {
    port: u16,
    description: String,
}

// ---------------------------------------------------------------------------
// Scan local services
// ---------------------------------------------------------------------------

/// Scan local ports for running services, optionally with custom ports.
pub async fn scan_local_services(extra_ports: Option<String>, timeout_ms: u64) -> Result<()> {
    let l = lang();
    println!(
        "\n{}",
        t!(l, "🔍 Scanning local services...", "🔍 扫描本地服务...").bold()
    );

    let dur = Duration::from_millis(timeout_ms);

    // Build full port list
    let mut ports: Vec<(u16, String)> = DEFAULT_PORTS
        .iter()
        .map(|&(p, en, _zh)| (p, en.to_string()))
        .collect();

    // Parse extra ports
    if let Some(extra) = extra_ports {
        for part in extra.split(',') {
            if let Ok(p) = part.trim().parse::<u16>() {
                if !ports.iter().any(|(pp, _)| *pp == p) {
                    ports.push((p, "custom".to_string()));
                }
            }
        }
    }

    // Scan concurrently
    let mut handles = Vec::new();
    for (port, desc) in &ports {
        let port = *port;
        let desc = desc.clone();
        handles.push(tokio::spawn(async move {
            let addr = format!("127.0.0.1:{port}");
            let open = matches!(timeout(dur, TcpStream::connect(&addr)).await, Ok(Ok(_)));
            (port, desc, open)
        }));
    }

    let mut found: Vec<DiscoveredService> = Vec::new();
    for handle in handles {
        if let Ok((port, desc, open)) = handle.await {
            if open {
                found.push(DiscoveredService {
                    port,
                    description: desc,
                });
            }
        }
    }

    found.sort_by_key(|s| s.port);

    // Display results
    if found.is_empty() {
        println!(
            "\n{}",
            t!(
                l,
                "No running services detected on common ports.",
                "未在常见端口上发现运行中的服务。"
            )
            .yellow()
        );
        return Ok(());
    }

    println!(
        "\n{} {} {}:\n",
        "✅".green(),
        t!(l, "Found", "发现"),
        found.len()
    );

    let mut table = Table::new();
    table.load_preset(UTF8_FULL);
    table.set_header(vec![
        t!(l, "Port", "端口"),
        t!(l, "Service", "服务"),
        t!(l, "URL", "地址"),
    ]);

    for svc in &found {
        table.add_row(vec![
            &svc.port.to_string(),
            &svc.description,
            &format!("http://localhost:{}", svc.port),
        ]);
    }

    println!("{table}");

    // Offer to create tunnel mappings
    offer_mapping_creation(&found).await?;

    Ok(())
}

/// Ask the user if they want to create tunnel config entries for discovered services.
async fn offer_mapping_creation(services: &[DiscoveredService]) -> Result<()> {
    let l = lang();

    let create = prompt::confirm_opt(
        t!(
            l,
            "Create tunnel mappings for these services?",
            "为发现的服务创建隧道映射?"
        ),
        false,
    )
    .unwrap_or(false);

    if !create {
        return Ok(());
    }

    let mut cfg = match load_tunnel_config() {
        Ok(c) => c,
        Err(e) => {
            println!(
                "{} {} {}",
                "⚠️".yellow(),
                t!(l, "Cannot load tunnel config:", "无法加载隧道配置:"),
                e
            );
            return Ok(());
        }
    };

    let mut added = 0u32;

    for svc in services {
        let prompt = format!(
            "{} {} ({}) {}",
            t!(l, "Hostname for port", "端口"),
            svc.port,
            svc.description,
            t!(l, "(leave empty to skip)", "(留空跳过)")
        );

        let hostname = match prompt::input_opt(&prompt, true, None) {
            Some(v) => v,
            None => return Ok(()),
        };

        if hostname.is_empty() {
            continue;
        }

        // Check duplicate
        if cfg
            .ingress
            .iter()
            .any(|r| r.hostname.as_deref() == Some(&hostname))
        {
            println!(
                "  ⏭️ {} {}",
                hostname,
                t!(l, "(already mapped)", "(已映射)")
            );
            continue;
        }

        let service_url = format!("http://localhost:{}", svc.port);

        // Insert before catch-all
        let pos = if cfg.ingress.is_empty() {
            0
        } else {
            cfg.ingress.len() - 1
        };
        cfg.ingress.insert(
            pos,
            IngressRule {
                hostname: Some(hostname.clone()),
                service: service_url.clone(),
            },
        );

        println!("  {} {} → {}", "✅".green(), hostname.cyan(), service_url);
        added += 1;
    }

    if added > 0 {
        save_tunnel_config(&cfg)?;
        println!(
            "\n{} {} {} {}",
            "📝".green(),
            added,
            t!(l, "mapping(s) saved.", "条映射已保存。"),
            t!(
                l,
                "Restart cloudflared to apply.",
                "重启 cloudflared 生效。"
            )
        );
    }

    Ok(())
}
