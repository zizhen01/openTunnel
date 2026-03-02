use colored::Colorize;
use comfy_table::{presets::UTF8_FULL, Table};

use crate::client::{CloudflareClient, CreateDnsRecord, ZoneSetting};
use crate::error::Result;
use crate::i18n::lang;
use crate::prompt;
use crate::t;
use crate::tunnel;

/// Create a CNAME record for a single hostname pointing to a tunnel.
/// Skips silently if the record already exists.
pub async fn ensure_dns_for_hostname(
    client: &CloudflareClient,
    tunnel_id: &str,
    hostname: &str,
) -> Result<()> {
    let l = lang();
    let tunnel_cname = format!("{tunnel_id}.cfargotunnel.com");

    let existing = client.list_dns_records().await.unwrap_or_default();
    let exists = existing
        .iter()
        .any(|r| r.name == hostname && r.record_type == "CNAME");

    if exists {
        println!(
            "  ⏭️ {} {} → {}",
            hostname,
            t!(l, "(CNAME already exists)", "(CNAME 已存在)"),
            tunnel_cname
        );
        return Ok(());
    }

    let record = CreateDnsRecord {
        record_type: "CNAME".to_string(),
        name: hostname.to_string(),
        content: tunnel_cname.clone(),
        proxied: true,
        ttl: None,
    };

    client.create_dns_record(&record).await?;
    println!("  {} {} → {}", "✅".green(), hostname, tunnel_cname);
    Ok(())
}

fn short_id(id: &str) -> String {
    id.chars().take(8).collect()
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        s.chars().take(max - 2).collect::<String>() + ".."
    }
}

// ---------------------------------------------------------------------------
// List DNS records
// ---------------------------------------------------------------------------

/// Display all DNS records for the configured zone.
pub async fn list_records(client: &CloudflareClient) -> Result<()> {
    let l = lang();
    println!(
        "{}",
        t!(l, "Fetching DNS records...", "获取 DNS 记录...").bold()
    );

    let records = client.list_dns_records().await?;

    if records.is_empty() {
        println!("{}", t!(l, "No DNS records found.", "未找到 DNS 记录。"));
        return Ok(());
    }

    let mut table = Table::new();
    table.load_preset(UTF8_FULL);
    table.set_header(vec![
        t!(l, "Name", "名称"),
        t!(l, "Type", "类型"),
        t!(l, "Content", "内容"),
        t!(l, "Proxy", "代理"),
    ]);

    for r in &records {
        let proxied_str = match r.proxied {
            Some(true) => "🟠",
            Some(false) => "⚪",
            None => "-",
        };
        let content = truncate(&r.content, 30);
        table.add_row(vec![&r.name, &r.record_type, &content, proxied_str]);
    }

    println!("{table}");
    println!(
        "\n{} {}",
        t!(l, "Total:", "共:"),
        records.len().to_string().cyan()
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// Add DNS record
// ---------------------------------------------------------------------------

/// Add a new DNS record, with optional interactive prompts.
pub async fn add_record(
    client: &CloudflareClient,
    name: Option<String>,
    record_type: Option<String>,
    content: Option<String>,
    proxied: bool,
) -> Result<()> {
    let l = lang();

    let name = match name {
        Some(n) => n,
        None => match prompt::input_opt(
            t!(l, "Record name (e.g. app)", "记录名 (如 app)"),
            false,
            None,
        ) {
            Some(v) => v,
            None => return Ok(()),
        },
    };

    let record_type = match record_type {
        Some(rt) => rt.to_uppercase(),
        None => {
            let types = vec!["CNAME", "A", "AAAA", "TXT", "MX"];
            let sel = prompt::select_opt(t!(l, "Record type", "记录类型"), &types, Some(0));
            let sel = sel.unwrap_or(0);
            types.get(sel).unwrap_or(&"CNAME").to_string()
        }
    };

    let content = match content {
        Some(c) => c,
        None => match prompt::input_opt(t!(l, "Record content / target", "记录内容"), false, None)
        {
            Some(v) => v,
            None => return Ok(()),
        },
    };

    let record = CreateDnsRecord {
        record_type: record_type.clone(),
        name: name.clone(),
        content: content.clone(),
        proxied,
        ttl: None,
    };

    println!(
        "{}",
        t!(l, "Creating DNS record...", "正在创建 DNS 记录...").bold()
    );
    let created = client.create_dns_record(&record).await?;

    println!(
        "{} {} {} → {} (ID: {})",
        "✅".green(),
        record_type,
        created.name.cyan(),
        content,
        short_id(&created.id)
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// Delete DNS record
// ---------------------------------------------------------------------------

/// Delete a DNS record. If `id` is None, show interactive picker.
pub async fn delete_record(client: &CloudflareClient, id: Option<String>) -> Result<()> {
    let l = lang();

    let record_id = match id {
        Some(id) => id,
        None => {
            let records = client.list_dns_records().await?;
            if records.is_empty() {
                println!(
                    "{}",
                    t!(l, "No DNS records to delete.", "没有可删除的 DNS 记录。")
                );
                return Ok(());
            }
            let items: Vec<String> = records
                .iter()
                .map(|r| format!("{} {} → {}", r.record_type, r.name, r.content))
                .collect();

            let sel = prompt::select_opt(
                t!(l, "Select record to delete", "选择要删除的记录"),
                &items,
                None,
            );

            match sel {
                Some(i) => match records.get(i) {
                    Some(record) => record.id.clone(),
                    None => return Ok(()),
                },
                None => return Ok(()),
            }
        }
    };

    let confirmed = prompt::confirm_opt(
        t!(
            l,
            "Are you sure you want to delete this record?",
            "确认删除该记录?"
        ),
        false,
    )
    .unwrap_or(false);

    if !confirmed {
        return Ok(());
    }

    client.delete_dns_record(&record_id).await?;
    println!(
        "{} {}",
        "✅".green(),
        t!(l, "DNS record deleted.", "DNS 记录已删除。")
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// Sync tunnel routes → DNS (via remotely-managed tunnel config API)
// ---------------------------------------------------------------------------

/// For each hostname in the tunnel's remote config, ensure a CNAME record
/// pointing to the tunnel exists.
pub async fn sync_tunnel_routes(
    client: &CloudflareClient,
    tunnel_id: Option<String>,
) -> Result<()> {
    let l = lang();

    let tunnel_id = match tunnel_id {
        Some(id) => id,
        None => match tunnel::select_tunnel(client).await? {
            Some(t) => t.id,
            None => return Ok(()),
        },
    };

    let config = client.get_tunnel_config(&tunnel_id).await?;
    let hostnames: Vec<String> = config
        .config
        .ingress
        .iter()
        .filter_map(|r| r.hostname.clone())
        .collect();

    if hostnames.is_empty() {
        println!(
            "{}",
            t!(
                l,
                "No hostnames configured in tunnel config.",
                "隧道配置中没有域名映射。"
            )
        );
        return Ok(());
    }

    let tunnel_cname = format!("{}.cfargotunnel.com", tunnel_id);

    println!(
        "{} {} {} ...",
        "🔄".cyan(),
        t!(l, "Syncing", "同步中"),
        hostnames.len()
    );

    let existing = client.list_dns_records().await.unwrap_or_default();

    let mut created = 0u32;
    let mut skipped = 0u32;

    for hostname in &hostnames {
        let exists = existing
            .iter()
            .any(|r| r.name == *hostname && r.record_type == "CNAME");

        if exists {
            println!(
                "  ⏭️ {} {}",
                hostname,
                t!(l, "(already exists)", "(已存在)")
            );
            skipped += 1;
            continue;
        }

        let record = CreateDnsRecord {
            record_type: "CNAME".to_string(),
            name: hostname.clone(),
            content: tunnel_cname.clone(),
            proxied: true,
            ttl: None,
        };

        match client.create_dns_record(&record).await {
            Ok(_) => {
                println!("  {} {} → {}", "✅".green(), hostname, tunnel_cname);
                created += 1;
            }
            Err(e) => {
                println!("  {} {} — {}", "❌".red(), hostname, e);
            }
        }
    }

    println!(
        "\n📊 {} {}, {} {}",
        created,
        t!(l, "created", "已创建"),
        skipped,
        t!(l, "skipped", "已跳过")
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// Zone settings (Always Use HTTPS, etc.)
// ---------------------------------------------------------------------------

/// Interactive menu to view and toggle zone settings.
pub async fn zone_settings_menu(client: &CloudflareClient) -> Result<()> {
    let l = lang();

    // Fetch current state of "always_use_https"
    let setting = match client.get_zone_setting("always_use_https").await {
        Ok(s) => s,
        Err(e) => {
            println!("{} {}: {}", "❌".red(), t!(l, "Failed to fetch zone settings", "获取域名设置失败"), e);
            return Ok(());
        }
    };

    let current_on = setting.value.as_str() == Some("on");
    let status_str = if current_on {
        t!(l, "ON", "已开启").green().to_string()
    } else {
        t!(l, "OFF", "已关闭").red().to_string()
    };

    println!(
        "\n{}",
        t!(l, "🔒 Zone Settings", "🔒 域名设置").bold()
    );
    println!(
        "  ├─ {}: {}",
        t!(l, "Always Use HTTPS", "强制 HTTPS"),
        status_str
    );
    println!();

    let toggle_label = if current_on {
        t!(l, "Turn OFF (allow HTTP)", "关闭强制 HTTPS (允许 HTTP)")
    } else {
        t!(l, "Turn ON (redirect HTTP → HTTPS)", "开启强制 HTTPS (HTTP 自动跳转 HTTPS)")
    };

    let options = vec![
        toggle_label,
        t!(l, "◀️  Back", "◀️  返回"),
    ];

    let sel = prompt::select_opt(t!(l, "Zone Settings", "域名设置"), &options, None);

    if sel != Some(0) {
        return Ok(());
    }

    let new_value = if current_on { "off" } else { "on" };
    match client
        .patch_zone_setting("always_use_https", serde_json::json!(new_value))
        .await
    {
        Ok(ZoneSetting { value, .. }) => {
            let v = value.as_str().unwrap_or(new_value);
            let label = if v == "on" {
                t!(l, "ON", "已开启").green().to_string()
            } else {
                t!(l, "OFF", "已关闭").red().to_string()
            };
            println!(
                "{} {} {}",
                "✅".green(),
                t!(l, "Always Use HTTPS is now", "强制 HTTPS 已设置为"),
                label
            );
        }
        Err(e) => {
            println!("{} {}: {}", "❌".red(), t!(l, "Failed to update setting", "更新设置失败"), e);
        }
    }

    Ok(())
}
