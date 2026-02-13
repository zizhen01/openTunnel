use anyhow::bail;
use base64::Engine;
use colored::Colorize;
use comfy_table::{presets::UTF8_FULL, Table};

use crate::client::{CloudflareClient, IngressRule, TunnelConfigInner, TunnelConfiguration};
use crate::error::Result;
use crate::i18n::lang;
use crate::{dns, prompt, service, t};

fn short_id(id: &str) -> String {
    id.chars().take(8).collect()
}

/// Format an ISO-8601 timestamp to "YYYY-MM-DD HH:MM".
fn format_time(ts: Option<&str>) -> String {
    match ts {
        Some(s) if s.len() >= 16 => {
            // "2026-02-07T10:25:27..." → "2026-02-07 10:25"
            s[..10].to_string() + " " + &s[11..16]
        }
        Some(s) => s.to_string(),
        None => "-".to_string(),
    }
}

fn normalize_service_input(input: &str) -> String {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return trimmed.to_string();
    }

    if trimmed.starts_with("http://")
        || trimmed.starts_with("https://")
        || trimmed.starts_with("http_status:")
        || trimmed.starts_with("unix:")
        || trimmed.starts_with("ssh://")
        || trimmed.starts_with("rdp://")
        || trimmed.starts_with("tcp://")
    {
        return trimmed.to_string();
    }

    if trimmed.chars().all(|c| c.is_ascii_digit()) {
        return format!("http://localhost:{trimmed}");
    }

    if let Some((host, port)) = trimmed.rsplit_once(':') {
        if !host.is_empty()
            && port.chars().all(|c| c.is_ascii_digit())
            && !host.contains("://")
            && !host.starts_with('/')
        {
            return format!("http://{trimmed}");
        }
    }

    trimmed.to_string()
}

// ---------------------------------------------------------------------------
// Tunnel selection helper
// ---------------------------------------------------------------------------

/// Interactively select a tunnel from the API. Returns `None` if cancelled.
pub async fn select_tunnel(client: &CloudflareClient) -> Result<Option<crate::client::Tunnel>> {
    let l = lang();
    let tunnels = client.list_tunnels().await?;

    if tunnels.is_empty() {
        println!("{}", t!(l, "No tunnels found.", "未找到隧道。"));
        return Ok(None);
    }

    let items: Vec<String> = tunnels
        .iter()
        .map(|t_info| {
            format!(
                "{} ({}) [{}]",
                t_info.name,
                short_id(&t_info.id),
                t_info.status.as_deref().unwrap_or("-")
            )
        })
        .collect();

    let sel = prompt::select_opt(t!(l, "Select tunnel", "选择隧道"), &items, None);

    Ok(sel.and_then(|i| tunnels.into_iter().nth(i)))
}

/// Resolve a tunnel ID: use provided `id` or select interactively.
async fn resolve_tunnel_id(
    client: &CloudflareClient,
    id: Option<String>,
) -> Result<Option<String>> {
    match id {
        Some(id) => Ok(Some(id)),
        None => Ok(select_tunnel(client).await?.map(|t| t.id)),
    }
}

// ---------------------------------------------------------------------------
// List tunnels
// ---------------------------------------------------------------------------

/// List all tunnels via the Cloudflare API.
pub async fn list_tunnels(client: &CloudflareClient) -> Result<()> {
    let l = lang();
    println!(
        "{}",
        t!(l, "Fetching tunnel list...", "获取隧道列表...").bold()
    );

    let tunnels = client.list_tunnels().await?;

    if tunnels.is_empty() {
        println!("{}", t!(l, "No tunnels found.", "未找到隧道。"));
        return Ok(());
    }

    let mut table = Table::new();
    table.load_preset(UTF8_FULL);
    table.set_header(vec![t!(l, "Name", "名称"), t!(l, "Status", "状态")]);

    for t_info in tunnels.iter() {
        table.add_row(vec![&t_info.name, t_info.status.as_deref().unwrap_or("-")]);
    }

    println!("{table}");
    println!(
        "\n{} {}",
        t!(l, "Total:", "共:"),
        tunnels.len().to_string().cyan()
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// Create tunnel
// ---------------------------------------------------------------------------

/// Create a new tunnel.
pub async fn create_tunnel(client: &CloudflareClient, name: Option<String>) -> Result<()> {
    let l = lang();
    let name = match name {
        Some(n) => n,
        None => match prompt::input_opt(t!(l, "Tunnel name", "隧道名称"), false, None) {
            Some(v) => v,
            None => return Ok(()),
        },
    };

    // Generate a random tunnel secret (32 bytes, base64)
    let secret_bytes: Vec<u8> = (0..32).map(|_| rand::random::<u8>()).collect();
    let secret = base64::engine::general_purpose::STANDARD.encode(&secret_bytes);

    println!("{}", t!(l, "Creating tunnel...", "正在创建隧道...").bold());
    let tunnel = client.create_tunnel(&name, &secret).await?;

    println!(
        "{} {} (ID: {})",
        "✅".green(),
        t!(l, "Tunnel created:", "隧道已创建:"),
        tunnel.id
    );

    let takeover = prompt::confirm_opt(
        t!(
            l,
            "Manage this tunnel in background now (install + start service)?",
            "现在由程序接管后台运行该隧道（安装并启动服务）？"
        ),
        true,
    )
    .unwrap_or(false);

    if takeover {
        println!(
            "{}",
            t!(
                l,
                "⚙️ Applying service management...",
                "⚙️ 正在应用服务托管..."
            )
            .bold()
        );

        match service::install(client, Some(tunnel.id.clone())).await {
            Ok(_) => match service::start() {
                Ok(_) => {
                    println!(
                        "{} {}",
                        "✅".green(),
                        t!(
                            l,
                            "Background service is running. Tunnel should become active shortly.",
                            "后台服务已启动，隧道应很快变为 active。"
                        )
                    );
                }
                Err(e) => {
                    println!("{} {:#}", "⚠️".yellow(), e);
                    println!(
                        "{}",
                        t!(
                            l,
                            "Service installed, but start failed. You can retry:",
                            "服务已安装，但启动失败。可手动重试："
                        )
                        .yellow()
                    );
                    println!("  tunnel service start");
                }
            },
            Err(e) => {
                println!("{} {:#}", "⚠️".yellow(), e);
                println!(
                    "{}",
                    t!(
                        l,
                        "Automatic background management failed. Run manually:",
                        "自动后台托管失败。请手动执行："
                    )
                    .yellow()
                );
                println!("  tunnel service install --tunnel {}", tunnel.id);
                println!("  tunnel service start");
            }
        }
    } else {
        println!(
            "\n{}",
            t!(
                l,
                "To run this tunnel in background via program, use:",
                "若要由程序后台托管运行，请执行："
            )
            .bold()
        );
        println!("  tunnel service install --tunnel {}", tunnel.id);
        println!("  tunnel service start");
        println!(
            "  {}",
            t!(
                l,
                "Or fetch token manually only when needed: `tunnel token <id>`",
                "或仅在需要时手动取 token：`tunnel token <id>`"
            )
        );
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Delete tunnel
// ---------------------------------------------------------------------------

/// Interactively select and delete a tunnel.
pub async fn delete_tunnel(client: &CloudflareClient) -> Result<()> {
    let l = lang();
    let tunnels = client.list_tunnels().await?;

    if tunnels.is_empty() {
        println!("{}", t!(l, "No tunnels to delete.", "没有可删除的隧道。"));
        return Ok(());
    }

    let items: Vec<String> = tunnels
        .iter()
        .map(|t_info| {
            format!(
                "{} ({})",
                t_info.name,
                t_info.status.as_deref().unwrap_or("unknown")
            )
        })
        .collect();

    let sel = prompt::select_opt(
        t!(l, "Select tunnel to delete", "选择要删除的隧道"),
        &items,
        None,
    );

    let idx = match sel {
        Some(i) => i,
        None => return Ok(()),
    };

    let target = match tunnels.get(idx) {
        Some(t) => t,
        None => return Ok(()),
    };

    let confirmed = prompt::confirm_opt(
        &format!(
            "{} '{}' ?",
            t!(l, "Delete tunnel", "确认删除隧道"),
            target.name
        ),
        false,
    )
    .unwrap_or(false);

    if !confirmed {
        return Ok(());
    }

    client.delete_tunnel(&target.id).await?;
    println!(
        "{} {}",
        "✅".green(),
        t!(l, "Tunnel deleted.", "隧道已删除。")
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// Get tunnel token
// ---------------------------------------------------------------------------

/// Get and display the run token for a tunnel.
pub async fn get_token(client: &CloudflareClient, id: Option<String>) -> Result<()> {
    let l = lang();

    let tunnel_id = match resolve_tunnel_id(client, id).await? {
        Some(id) => id,
        None => return Ok(()),
    };

    let token = client.get_tunnel_token(&tunnel_id).await?;
    println!(
        "\n{}",
        t!(l, "Run this tunnel with:", "使用以下命令运行隧道:").bold()
    );
    println!("  cloudflared tunnel run --token {}", token);
    Ok(())
}

// ---------------------------------------------------------------------------
// Show mappings (remotely-managed tunnel config via API)
// ---------------------------------------------------------------------------

/// Show current ingress mappings for a tunnel via the API.
pub async fn show_mappings(client: &CloudflareClient, id: Option<String>) -> Result<()> {
    let l = lang();

    let tunnel_id = match resolve_tunnel_id(client, id).await? {
        Some(id) => id,
        None => return Ok(()),
    };

    let config = client.get_tunnel_config(&tunnel_id).await?;
    let rules = &config.config.ingress;

    if rules.is_empty() {
        println!("\n{}", t!(l, "No mappings configured.", "未配置映射。"));
        return Ok(());
    }

    // Fetch connector info for origin IP and running time
    let conns = client.list_tunnel_connections(&tunnel_id).await.ok();
    let (origin_ip, run_at) = conns
        .as_ref()
        .and_then(|c| c.first())
        .map(|conn| {
            let ip = conn
                .conns
                .first()
                .and_then(|c| c.origin_ip.clone())
                .unwrap_or_else(|| "-".to_string());
            let run = format_time(conn.run_at.as_deref());
            (ip, run)
        })
        .unwrap_or_else(|| ("-".to_string(), "-".to_string()));

    println!(
        "\n{} {}  {} {}  {} {}",
        t!(l, "Tunnel:", "隧道:").bold(),
        short_id(&tunnel_id).cyan(),
        t!(l, "Origin IP:", "来源 IP:").bold(),
        origin_ip,
        t!(l, "Running since:", "运行时间:").bold(),
        run_at,
    );

    let mut table = Table::new();
    table.load_preset(UTF8_FULL);
    table.set_header(vec![
        "#",
        t!(l, "Hostname", "域名"),
        t!(l, "Service", "服务"),
    ]);

    for (i, rule) in rules.iter().enumerate() {
        table.add_row(vec![
            &(i + 1).to_string(),
            rule.hostname.as_deref().unwrap_or("* (catch-all)"),
            &rule.service,
        ]);
    }

    println!("{table}");
    Ok(())
}

// ---------------------------------------------------------------------------
// Add mapping (remotely-managed via API)
// ---------------------------------------------------------------------------

/// Add a hostname→service mapping via the tunnel configuration API.
pub async fn add_mapping(
    client: &CloudflareClient,
    tunnel_id: Option<String>,
    hostname: Option<String>,
    service: Option<String>,
) -> Result<()> {
    let l = lang();

    let tunnel_id = match resolve_tunnel_id(client, tunnel_id).await? {
        Some(id) => id,
        None => return Ok(()),
    };

    let hostname = match hostname {
        Some(h) => h,
        None => match prompt::input_opt(
            t!(
                l,
                "Hostname (e.g. app.example.com)",
                "域名 (如 app.example.com)"
            ),
            false,
            None,
        ) {
            Some(v) => v,
            None => return Ok(()),
        },
    };

    let raw_service = match service {
        Some(s) => s,
        None => match prompt::input_opt(
            t!(
                l,
                "Service URL (e.g. http://localhost:3000)",
                "服务地址 (如 http://localhost:3000)"
            ),
            false,
            None,
        ) {
            Some(v) => v,
            None => return Ok(()),
        },
    };
    let service = normalize_service_input(&raw_service);
    if service != raw_service {
        println!(
            "{} {} {}",
            "ℹ️".cyan(),
            t!(
                l,
                "Normalized service target to:",
                "已自动规范化服务地址为:"
            ),
            service
        );
    }

    // Fetch current config
    let mut config = client
        .get_tunnel_config(&tunnel_id)
        .await
        .unwrap_or_else(|_| TunnelConfiguration {
            config: TunnelConfigInner {
                ingress: vec![IngressRule {
                    hostname: None,
                    service: "http_status:404".to_string(),
                    origin_request: None,
                }],
            },
        });

    // Check for duplicates
    if config
        .config
        .ingress
        .iter()
        .any(|r| r.hostname.as_deref() == Some(hostname.as_str()))
    {
        bail!(
            "{}",
            t!(l, "Hostname already mapped.", "该域名已存在映射。")
        );
    }

    // Insert before the catch-all rule (last entry)
    let insert_pos = if config.config.ingress.is_empty() {
        0
    } else {
        config.config.ingress.len() - 1
    };

    config.config.ingress.insert(
        insert_pos,
        IngressRule {
            hostname: Some(hostname.clone()),
            service: service.clone(),
            origin_request: None,
        },
    );

    client.put_tunnel_config(&tunnel_id, &config).await?;
    println!("{} {} → {}", "✅".green(), hostname.cyan(), service);

    // Offer to create DNS record for this specific hostname (only if zone is configured)
    if client.zone_id.is_some() {
        let dns_prompt = t!(
            l,
            "Create DNS record for this hostname now?",
            "是否立刻为该域名创建 DNS 记录？"
        );
        if prompt::confirm_opt(dns_prompt, true) == Some(true) {
            if let Err(e) =
                dns::ensure_dns_for_hostname(client, &tunnel_id, &hostname).await
            {
                println!(
                    "{} {} {:#}",
                    "⚠️".yellow(),
                    t!(l, "DNS record creation failed:", "DNS 记录创建失败:"),
                    e
                );
                println!(
                    "  {}",
                    t!(
                        l,
                        "You can manually run: tunnel dns sync",
                        "可手动执行: tunnel dns sync"
                    )
                );
            }
        }
    } else {
        println!(
            "{}",
            t!(
                l,
                "💡 DNS zone not configured. Run `tunnel config set` to enable auto DNS sync.",
                "💡 未配置 DNS 域名，运行 `tunnel config set` 后可自动同步 DNS。"
            )
            .cyan()
        );
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Remove mapping (remotely-managed via API)
// ---------------------------------------------------------------------------

/// Remove a hostname mapping via the tunnel configuration API.
pub async fn remove_mapping(
    client: &CloudflareClient,
    tunnel_id: Option<String>,
    hostname: Option<String>,
) -> Result<()> {
    let l = lang();

    let tunnel_id = match resolve_tunnel_id(client, tunnel_id).await? {
        Some(id) => id,
        None => return Ok(()),
    };

    let mut config = client.get_tunnel_config(&tunnel_id).await?;

    let hostnames: Vec<String> = config
        .config
        .ingress
        .iter()
        .filter_map(|r| r.hostname.clone())
        .collect();

    if hostnames.is_empty() {
        println!("{}", t!(l, "No mappings to remove.", "没有可移除的映射。"));
        return Ok(());
    }

    let target = match hostname {
        Some(h) => h,
        None => {
            let sel = prompt::select_opt(
                t!(l, "Select mapping to remove", "选择要移除的映射"),
                &hostnames,
                None,
            );
            match sel {
                Some(i) => match hostnames.get(i) {
                    Some(h) => h.clone(),
                    None => return Ok(()),
                },
                None => return Ok(()),
            }
        }
    };

    let before = config.config.ingress.len();
    config
        .config
        .ingress
        .retain(|r| r.hostname.as_deref() != Some(&target));

    if config.config.ingress.len() == before {
        bail!("{}", t!(l, "Mapping not found.", "未找到该映射。"));
    }

    client.put_tunnel_config(&tunnel_id, &config).await?;
    println!(
        "{} {} {}",
        "✅".green(),
        target.cyan(),
        t!(l, "removed.", "已移除。")
    );
    Ok(())
}
