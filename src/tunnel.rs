use anyhow::bail;
use base64::Engine;
use colored::Colorize;
use comfy_table::{presets::UTF8_FULL, Table};

use crate::client::CloudflareClient;
use crate::config::{self, load_tunnel_config, save_tunnel_config, IngressRule, TunnelConfig};
use crate::error::Result;
use crate::i18n::lang;
use crate::prompt;
use crate::t;

fn short_id(id: &str) -> String {
    id.chars().take(8).collect()
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
    table.set_header(vec![
        t!(l, "Name", "名称"),
        t!(l, "ID", "ID"),
        t!(l, "Status", "状态"),
        t!(l, "Created", "创建时间"),
    ]);

    for t_info in &tunnels {
        table.add_row(vec![
            &t_info.name,
            &t_info.id,
            t_info.status.as_deref().unwrap_or("-"),
            t_info.created_at.as_deref().unwrap_or("-"),
        ]);
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
// Switch active tunnel (update config.yml)
// ---------------------------------------------------------------------------

/// Switch the active tunnel in the local cloudflared config.
pub async fn switch_tunnel(client: &CloudflareClient) -> Result<()> {
    let l = lang();
    let tunnels = client.list_tunnels().await?;

    if tunnels.is_empty() {
        println!("{}", t!(l, "No tunnels available.", "没有可用的隧道。"));
        return Ok(());
    }

    let current_id = load_tunnel_config().ok().map(|c| c.tunnel.clone());

    let items: Vec<String> = tunnels
        .iter()
        .map(|t_info| {
            let mark = if current_id.as_deref() == Some(&t_info.id) {
                " ← current"
            } else {
                ""
            };
            format!("{} ({}){mark}", t_info.name, short_id(&t_info.id))
        })
        .collect();

    let sel = prompt::select_opt(t!(l, "Select tunnel", "选择隧道"), &items, None);

    let idx = match sel {
        Some(i) => i,
        None => return Ok(()),
    };

    let target = match tunnels.get(idx) {
        Some(t) => t,
        None => return Ok(()),
    };

    // Update the tunnel config YAML
    let cred_dir = dirs::home_dir()
        .map(|h| h.join(".cloudflared"))
        .unwrap_or_else(|| std::path::PathBuf::from("/etc/cloudflared"));

    let mut cfg = load_tunnel_config().unwrap_or_else(|_| TunnelConfig {
        tunnel: String::new(),
        credentials_file: String::new(),
        ingress: vec![IngressRule {
            hostname: None,
            service: "http_status:404".to_string(),
        }],
    });

    cfg.tunnel = target.id.clone();
    cfg.credentials_file = cred_dir
        .join(format!("{}.json", target.id))
        .to_string_lossy()
        .to_string();

    save_tunnel_config(&cfg)?;
    println!(
        "{} {} '{}'",
        "✅".green(),
        t!(l, "Switched to tunnel", "已切换到隧道"),
        target.name
    );

    Ok(())
}

// ---------------------------------------------------------------------------
// Add / remove ingress mappings
// ---------------------------------------------------------------------------

/// Add a hostname→service mapping to the tunnel config.
pub async fn add_mapping(hostname: Option<String>, service: Option<String>) -> Result<()> {
    let l = lang();
    let mut cfg = load_tunnel_config()?;

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

    let service = match service {
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

    // Check for duplicates
    if cfg
        .ingress
        .iter()
        .any(|r| r.hostname.as_deref() == Some(&hostname))
    {
        bail!(
            "{}",
            t!(l, "Hostname already mapped.", "该域名已存在映射。")
        );
    }

    // Insert before the catch-all rule (last entry)
    let insert_pos = if cfg.ingress.is_empty() {
        0
    } else {
        cfg.ingress.len() - 1
    };

    cfg.ingress.insert(
        insert_pos,
        IngressRule {
            hostname: Some(hostname.clone()),
            service: service.clone(),
        },
    );

    save_tunnel_config(&cfg)?;
    println!("{} {} → {}", "✅".green(), hostname.cyan(), service);
    Ok(())
}

/// Remove a hostname mapping from the tunnel config.
pub async fn remove_mapping(hostname: Option<String>) -> Result<()> {
    let l = lang();
    let mut cfg = load_tunnel_config()?;

    let hostnames: Vec<String> = config::configured_hostnames(&cfg);

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

    let before = cfg.ingress.len();
    cfg.ingress
        .retain(|r| r.hostname.as_deref() != Some(&target));

    if cfg.ingress.len() == before {
        bail!("{}", t!(l, "Mapping not found.", "未找到该映射。"));
    }

    save_tunnel_config(&cfg)?;
    println!(
        "{} {} {}",
        "✅".green(),
        target.cyan(),
        t!(l, "removed.", "已移除。")
    );
    Ok(())
}

/// Show current tunnel config and mappings.
pub fn show_mappings() -> Result<()> {
    let l = lang();
    let cfg = load_tunnel_config()?;

    println!(
        "\n{} {}",
        t!(l, "Tunnel:", "隧道:").bold(),
        cfg.tunnel.cyan()
    );
    println!(
        "{} {}",
        t!(l, "Credentials:", "凭证:").bold(),
        cfg.credentials_file
    );

    if cfg.ingress.is_empty() {
        println!("\n{}", t!(l, "No mappings configured.", "未配置映射。"));
        return Ok(());
    }

    let mut table = Table::new();
    table.load_preset(UTF8_FULL);
    table.set_header(vec![
        "#",
        t!(l, "Hostname", "域名"),
        t!(l, "Service", "服务"),
    ]);

    for (i, rule) in cfg.ingress.iter().enumerate() {
        table.add_row(vec![
            &(i + 1).to_string(),
            rule.hostname.as_deref().unwrap_or("* (catch-all)"),
            &rule.service,
        ]);
    }

    println!("\n{table}");
    Ok(())
}
