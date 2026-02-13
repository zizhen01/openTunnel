use colored::Colorize;
use comfy_table::{presets::UTF8_FULL, Table};
use tokio::net::TcpStream;
use tokio::time::{timeout, Duration};

use crate::error::Result;
use crate::i18n::lang;
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

    let mut found = Vec::new();
    for handle in handles {
        if let Ok((port, desc, open)) = handle.await {
            if open {
                found.push((port, desc));
            }
        }
    }

    found.sort_by_key(|(p, _)| *p);

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
    table.set_header(vec![t!(l, "Port", "端口"), t!(l, "Service", "服务")]);

    for (port, desc) in &found {
        table.add_row(vec![&port.to_string(), desc.as_str()]);
    }

    println!("{table}");

    println!(
        "\n💡 {}",
        t!(
            l,
            "Use `tunnel map` to create tunnel mappings for these services.",
            "使用 `tunnel map` 为这些服务创建隧道映射。"
        )
    );

    Ok(())
}
