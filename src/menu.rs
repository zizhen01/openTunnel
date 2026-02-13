use colored::Colorize;

use crate::client::{CloudflareClient, TokenVerifyStatus};
use crate::config;
use crate::error::Result;
use crate::i18n::lang;
use crate::{access, dns, monitor, prompt, scan, service, t, tools, tunnel};

// ---------------------------------------------------------------------------
// Main interactive menu
// ---------------------------------------------------------------------------

/// Entry point for the interactive TUI menu.
pub async fn interactive_menu() -> Result<()> {
    let mut asked_config = false;
    loop {
        let l = lang();
        clear_screen();
        print_banner();

        let status = tools::get_system_status();
        tools::print_status(&status);

        if !asked_config && !status.api_configured {
            asked_config = true;
            let confirm = prompt::confirm_opt(
                t!(
                    l,
                    "API not configured. Set up now?",
                    "API 未配置。现在设置?"
                ),
                true,
            )
            .unwrap_or(false);
            if confirm {
                if let Err(e) = set_api_token().await {
                    println!("\n{} {:#}", "❌".red(), e);
                }
            }
        }

        let options = vec![
            t!(l, "➕ Add Domain Mapping", "➕ 添加域名映射"),
            t!(l, "🌩️  Tunnel Management", "🌩️  隧道管理"),
            t!(l, "⚙️  cloudflared Service", "⚙️  cloudflared 服务"),
            t!(l, "🌐 DNS Management", "🌐 DNS 管理"),
            t!(l, "🔐 Zero Trust / Access", "🔐 Zero Trust / Access"),
            t!(l, "📊 Monitoring & Scan", "📊 监控与扫描"),
            t!(l, "🔧 Settings", "🔧 设置"),
            t!(l, "❌ Exit", "❌ 退出"),
        ];

        let sel = match prompt::select_opt_result(
            t!(l, "Select module", "选择功能模块"),
            &options,
            Some(0),
        ) {
            Ok(v) => v,
            Err(e) => {
                println!("\n{} {:#}\n", "❌".red(), e);
                continue;
            }
        };

        let result = match sel {
            Some(0) => {
                // Quick Map — the killer feature
                if let Some(client) = try_build_client() {
                    tunnel::add_mapping(&client, None, None, None).await
                } else {
                    Ok(())
                }
            }
            Some(1) => tunnel_menu().await,
            Some(2) => tunnel_service_menu().await,
            Some(3) => dns_menu().await,
            Some(4) => access_menu().await,
            Some(5) => monitoring_scan_menu().await,
            Some(6) => settings_menu().await,
            Some(7) | None => {
                println!("{}", t!(l, "👋 Goodbye!", "👋 再见！").cyan());
                break;
            }
            _ => Ok(()),
        };

        if let Err(e) = result {
            println!("\n{} {:#}", "❌".red(), e);
        }

        // Wait for user to read the output before clearing
        println!();
        prompt::pause(t!(l, "Press Enter to continue...", "按 Enter 继续..."));
    }
    Ok(())
}

/// Run only the API token configuration wizard.
pub async fn run_config_set_wizard() -> Result<()> {
    set_api_token().await
}

fn print_banner() {
    println!("\n{}", "═".repeat(60).cyan());
    println!(
        "{}",
        format!("  🌩️  openTunnel v{}", env!("CARGO_PKG_VERSION"))
            .bold()
            .cyan()
    );
    println!("{}", "═".repeat(60).cyan());
}

fn clear_screen() {
    print!("\x1B[2J\x1B[H");
}

/// Try to build a `CloudflareClient`. On failure, print the error and return None.
fn try_build_client() -> Option<CloudflareClient> {
    let l = lang();
    match config::require_api_config() {
        Ok(cfg) => match CloudflareClient::from_config(&cfg) {
            Ok(c) => Some(c),
            Err(e) => {
                println!("{} {}", "❌".red(), e);
                None
            }
        },
        Err(_) => {
            println!(
                "{} {}",
                "❌".red(),
                t!(
                    l,
                    "API not configured. Run `tunnel config set` first.",
                    "API 未配置，请先运行 `tunnel config set`。"
                )
            );
            None
        }
    }
}

/// Try to build a client with zone_id. On failure, print the error and return None.
fn try_build_client_with_zone() -> Option<CloudflareClient> {
    let l = lang();
    match config::require_zone_config() {
        Ok(cfg) => match CloudflareClient::from_config(&cfg) {
            Ok(c) => Some(c),
            Err(e) => {
                println!("{} {}", "❌".red(), e);
                None
            }
        },
        Err(_) => {
            println!(
                "{} {}",
                "❌".red(),
                t!(
                    l,
                    "API/Zone not configured. Run `tunnel config set` first.",
                    "API/域名未配置，请先运行 `tunnel config set`。"
                )
            );
            None
        }
    }
}

// ---------------------------------------------------------------------------
// Tunnel sub-menu
// ---------------------------------------------------------------------------

async fn tunnel_menu() -> Result<()> {
    let l = lang();
    let client = match try_build_client() {
        Some(c) => c,
        None => return Ok(()),
    };

    let options = vec![
        t!(l, "📋 Show mappings", "📋 查看当前映射"),
        t!(l, "➕ Add domain mapping", "➕ 添加域名映射"),
        t!(l, "➖ Remove domain mapping", "➖ 移除域名映射"),
        t!(l, "📋 List tunnels", "📋 查看隧道列表"),
        t!(l, "🆕 Create tunnel", "🆕 创建新隧道"),
        t!(l, "🗑️  Delete tunnel", "🗑️  删除隧道"),
        t!(l, "🔑 Get tunnel token", "🔑 获取隧道 Token"),
        t!(l, "◀️  Back", "◀️  返回主菜单"),
    ];

    let sel = prompt::select_opt(t!(l, "Tunnel Management", "隧道管理"), &options, None);

    match sel {
        Some(0) => tunnel::show_mappings(&client, None).await?,
        Some(1) => tunnel::add_mapping(&client, None, None, None).await?,
        Some(2) => tunnel::remove_mapping(&client, None, None).await?,
        Some(3) => tunnel::list_tunnels(&client).await?,
        Some(4) => tunnel::create_tunnel(&client, None).await?,
        Some(5) => tunnel::delete_tunnel(&client).await?,
        Some(6) => tunnel::get_token(&client, None).await?,
        Some(7) | None => {}
        _ => {}
    }
    Ok(())
}

async fn tunnel_service_menu() -> Result<()> {
    let l = lang();
    let options = vec![
        t!(l, "🔎 Service status", "🔎 服务状态"),
        t!(
            l,
            "📦 Install service (with tunnel token)",
            "📦 安装服务 (携带隧道 Token)"
        ),
        t!(l, "▶️ Start service", "▶️ 启动服务"),
        t!(l, "⏹ Stop service", "⏹ 停止服务"),
        t!(l, "🔄 Restart service", "🔄 重启服务"),
        t!(l, "📜 Show logs", "📜 查看日志"),
        t!(l, "◀️  Back", "◀️  返回"),
    ];

    let sel = prompt::select_opt(t!(l, "Tunnel Service", "隧道服务"), &options, None);
    match sel {
        Some(0) => service::status().await?,
        Some(1) => {
            if let Some(client) = try_build_client() {
                service::install(&client, None).await?;
            }
        }
        Some(2) => service::start()?,
        Some(3) => service::stop()?,
        Some(4) => service::restart()?,
        Some(5) => service::logs(100)?,
        Some(6) | None => {}
        _ => {}
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// DNS sub-menu
// ---------------------------------------------------------------------------

async fn dns_menu() -> Result<()> {
    let l = lang();

    let client = match try_build_client_with_zone() {
        Some(c) => c,
        None => {
            println!(
                "💡 {}",
                t!(l, "Run: tunnel config set", "请运行: tunnel config set")
            );
            return Ok(());
        }
    };

    let options = vec![
        t!(l, "📋 List DNS records", "📋 查看 DNS 记录"),
        t!(l, "➕ Add DNS record", "➕ 添加 DNS 记录"),
        t!(l, "🗑️  Delete DNS record", "🗑️  删除 DNS 记录"),
        t!(l, "🔄 Sync tunnel routes", "🔄 同步隧道路由"),
        t!(l, "◀️  Back", "◀️  返回主菜单"),
    ];

    let sel = prompt::select_opt(t!(l, "DNS Management", "DNS 管理"), &options, None);

    match sel {
        Some(0) => dns::list_records(&client).await?,
        Some(1) => dns::add_record(&client, None, None, None, true).await?,
        Some(2) => dns::delete_record(&client, None).await?,
        Some(3) => dns::sync_tunnel_routes(&client, None).await?,
        Some(4) | None => {}
        _ => {}
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Access sub-menu
// ---------------------------------------------------------------------------

async fn access_menu() -> Result<()> {
    let client = match try_build_client() {
        Some(c) => c,
        None => return Ok(()),
    };

    let l = lang();
    let options = vec![
        t!(l, "📋 List Access apps", "📋 查看 Access 应用"),
        t!(l, "🆕 Create app", "🆕 创建新应用"),
        t!(l, "🗑️  Delete app", "🗑️  删除应用"),
        t!(l, "🔐 Manage policies", "🔐 管理访问策略"),
        t!(l, "◀️  Back", "◀️  返回主菜单"),
    ];

    let sel = prompt::select_opt(
        t!(l, "Zero Trust / Access", "Zero Trust / Access"),
        &options,
        None,
    );

    match sel {
        Some(0) => access::list_apps(&client).await?,
        Some(1) => access::create_app(&client, None, None).await?,
        Some(2) => access::delete_app(&client, None).await?,
        Some(3) => access::manage_policies(&client, None).await?,
        Some(4) | None => {}
        _ => {}
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Monitoring sub-menu
// ---------------------------------------------------------------------------

async fn monitoring_scan_menu() -> Result<()> {
    let l = lang();
    let options = vec![
        t!(l, "📊 Tunnel statistics", "📊 隧道统计"),
        t!(l, "📈 Real-time monitor", "📈 实时监控"),
        t!(l, "🔍 Scan local services", "🔍 扫描本地服务"),
        t!(l, "◀️  Back", "◀️  返回主菜单"),
    ];

    let sel = prompt::select_opt(
        t!(l, "Monitoring & Scan", "监控与扫描"),
        &options,
        None,
    );

    match sel {
        Some(0) => monitor::show_stats().await?,
        Some(1) => monitor::real_time_monitor().await?,
        Some(2) => scan::scan_local_services(None, 500).await?,
        Some(3) | None => {}
        _ => {}
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Config sub-menu
// ---------------------------------------------------------------------------

async fn settings_menu() -> Result<()> {
    let l = lang();
    let options = vec![
        t!(l, "🌐 Switch language", "🌐 切换语言"),
        t!(l, "🔑 Set API Token", "🔑 设置 API Token"),
        t!(l, "👤 Account Management", "👤 账户管理"),
        t!(l, "📋 Show config", "📋 查看当前配置"),
        t!(l, "🧪 Test API connection", "🧪 测试 API 连接"),
        t!(l, "🔧 Health check", "🔧 健康检查"),
        t!(l, "🐛 Debug info", "🐛 调试信息"),
        t!(l, "📦 Export config", "📦 导出配置"),
        t!(l, "🗑️  Clear config", "🗑️  清除配置"),
        t!(l, "◀️  Back", "◀️  返回主菜单"),
    ];

    let sel = prompt::select_opt(t!(l, "Settings", "设置"), &options, None);

    match sel {
        Some(0) => switch_language()?,
        Some(1) => set_api_token().await?,
        Some(2) => account_menu().await?,
        Some(3) => show_api_config()?,
        Some(4) => test_api_connection().await?,
        Some(5) => tools::health_check().await?,
        Some(6) => tools::debug_mode()?,
        Some(7) => tools::export_config()?,
        Some(8) => clear_config()?,
        Some(9) | None => {}
        _ => {}
    }
    Ok(())
}

async fn account_menu() -> Result<()> {
    let l = lang();
    let options = vec![
        t!(l, "📋 List accounts", "📋 列出账户"),
        t!(l, "✅ Set active account", "✅ 设置当前账户"),
        t!(l, "◀️  Back", "◀️  返回"),
    ];

    let sel = prompt::select_opt(t!(l, "Account Management", "账户管理"), &options, None);
    match sel {
        Some(0) => list_accounts().await?,
        Some(1) => set_account(None).await?,
        Some(2) | None => {}
        _ => {}
    }
    Ok(())
}

/// Interactive API token setup wizard.
async fn set_api_token() -> Result<()> {
    let l = lang();
    println!(
        "{}",
        t!(
            l,
            "🔑 Configure Cloudflare API Token",
            "🔑 配置 Cloudflare API Token"
        )
        .bold()
    );
    println!();
    println!(
        "{}",
        t!(l, "📖 How to get an API Token:", "📖 获取 API Token:")
    );
    println!(
        "   1. {} https://dash.cloudflare.com/profile/api-tokens",
        t!(l, "Visit:", "访问:")
    );
    println!("   2. {} 'Create Token'", t!(l, "Click", "点击"));
    println!("   3. {}:", t!(l, "Required permissions", "所需权限"));
    println!("      • Account - Cloudflare Tunnel: Edit");
    println!("      • Zone - DNS: Edit");
    println!("      • Account - Access: Edit");
    println!();

    let token = match prompt::secret_input_opt("API Token", false) {
        Some(v) => v,
        None => return Ok(()),
    };
    if token.is_empty() {
        return Ok(());
    }

    // Fetch accounts
    let mut account_err = None;
    let accounts = match CloudflareClient::fetch_accounts(&token).await {
        Ok(v) => v,
        Err(e) => {
            account_err = Some(e);
            Vec::new()
        }
    };
    let account_id = if accounts.len() == 1 {
        println!("📋 {} '{}'", t!(l, "Account:", "账户:"), accounts[0].name);
        Some(accounts[0].id.clone())
    } else if accounts.len() > 1 {
        let items: Vec<String> = accounts
            .iter()
            .map(|a| format!("{} ({})", a.name, a.id))
            .collect();
        let sel = prompt::select_opt(t!(l, "Select account", "选择账户"), &items, None);
        sel.and_then(|i| accounts.get(i).map(|a| a.id.clone()))
    } else {
        println!(
            "{}",
            t!(l, "⚠️  No accounts found.", "⚠️  未找到账户。").yellow()
        );
        println!(
            "{}",
            t!(
                l,
                "Tip: ensure the token has 'Account - Account: Read' permission.",
                "提示：请确认 Token 包含 'Account - Account: Read' 权限。"
            )
            .yellow()
        );
        None
    };

    // Verify token with detailed checks
    println!(
        "\n{}",
        t!(l, "🔍 Verifying permissions...", "🔍 验证权限...").bold()
    );

    // 1. Token validity
    let verify = match CloudflareClient::verify_token(&token, account_id.as_deref()).await {
        Ok(v) => v,
        Err(_) => TokenVerifyStatus::Unknown,
    };
    match verify {
        TokenVerifyStatus::Valid => {
            println!("  {} {}", "✅".green(), t!(l, "Token valid", "Token 有效"))
        }
        TokenVerifyStatus::Invalid => println!(
            "  {} {}",
            "❌".red(),
            t!(l, "Token invalid or expired", "Token 无效或已过期")
        ),
        TokenVerifyStatus::Unknown => println!(
            "  {} {}",
            "⚠️".yellow(),
            t!(l, "Token status unknown", "Token 状态未知")
        ),
    }

    // 2. Tunnel permission (list tunnels)
    if let Some(ref acct) = account_id {
        let tmp_cfg = config::ApiConfig {
            api_token: Some(token.clone()),
            account_id: Some(acct.clone()),
            zone_id: None,
            zone_name: None,
            language: None,
        };
        let tmp_client = CloudflareClient::from_config(&tmp_cfg)?;
        match tmp_client.list_tunnels().await {
            Ok(tunnels) => println!(
                "  {} {} ({} {})",
                "✅".green(),
                t!(l, "Tunnel permission", "隧道权限"),
                tunnels.len(),
                t!(l, "tunnels found", "个隧道")
            ),
            Err(_) => println!(
                "  {} {}",
                "❌".red(),
                t!(
                    l,
                    "Tunnel permission — cannot list tunnels",
                    "隧道权限 — 无法列出隧道"
                )
            ),
        }
    }

    // 3. Zone / DNS permission (fetch zones)
    let mut zone_err = None;
    let zones = match CloudflareClient::fetch_zones(&token).await {
        Ok(v) => {
            println!(
                "  {} {} ({} {})",
                "✅".green(),
                t!(l, "DNS permission", "DNS 权限"),
                v.len(),
                t!(l, "zones found", "个域名")
            );
            v
        }
        Err(e) => {
            println!(
                "  {} {}",
                "❌".red(),
                t!(
                    l,
                    "DNS permission — cannot list zones",
                    "DNS 权限 — 无法列出域名"
                )
            );
            zone_err = Some(e);
            Vec::new()
        }
    };

    println!(); // blank line after permission checks
    let (zone_id, zone_name) = if zones.len() == 1 {
        println!("🌐 {} '{}'", t!(l, "Zone:", "域名:"), zones[0].name);
        (Some(zones[0].id.clone()), Some(zones[0].name.clone()))
    } else if zones.len() > 1 {
        let items: Vec<String> = zones
            .iter()
            .map(|z| format!("{} ({})", z.name, z.id))
            .collect();
        let sel = prompt::select_opt(
            t!(l, "Select zone (for DNS)", "选择域名 (用于 DNS 管理)"),
            &items,
            None,
        );
        match sel {
            Some(i) => match zones.get(i) {
                Some(z) => (Some(z.id.clone()), Some(z.name.clone())),
                None => (None, None),
            },
            None => (None, None),
        }
    } else {
        println!(
            "{}",
            t!(l, "⚠️  No zones found.", "⚠️  未找到域名。").yellow()
        );
        (None, None)
    };

    if accounts.is_empty() && zones.is_empty() {
        println!(
            "{} {}",
            "❌".red(),
            t!(
                l,
                "No accounts/zones accessible. Check token permissions.",
                "无法访问任何账户或域名。请检查 Token 权限。"
            )
        );
        if let Some(e) = account_err {
            println!("   {}: {}", t!(l, "Accounts", "账户"), e);
        }
        if let Some(e) = zone_err {
            println!("   {}: {}", t!(l, "Zones", "域名"), e);
        }
        return Ok(());
    }

    // Save config
    let cfg = config::ApiConfig {
        api_token: Some(token),
        account_id,
        zone_id,
        zone_name,
        language: None,
    };
    config::save_api_config(&cfg)?;
    println!(
        "\n{} {}",
        "✅".green(),
        t!(l, "Configuration saved.", "配置已保存。")
    );
    Ok(())
}

fn show_api_config() -> Result<()> {
    let l = lang();
    match config::load_api_config()? {
        Some(cfg) => {
            println!(
                "\n⚙️ {}",
                t!(l, "Current API Configuration:", "当前 API 配置:").bold()
            );
            println!("├─ API Token: {}", cfg.masked_token());
            println!(
                "├─ Account ID: {}",
                cfg.account_id
                    .as_deref()
                    .unwrap_or(t!(l, "not set", "未设置"))
            );
            println!(
                "├─ Zone ID: {}",
                cfg.zone_id.as_deref().unwrap_or(t!(l, "not set", "未设置"))
            );
            println!(
                "└─ Zone Name: {}",
                cfg.zone_name
                    .as_deref()
                    .unwrap_or(t!(l, "not set", "未设置"))
            );
        }
        None => {
            println!(
                "⚠️ {}",
                t!(
                    l,
                    "API not configured. Run: tunnel config set",
                    "API 未配置，请运行: tunnel config set"
                )
                .yellow()
            );
        }
    }
    Ok(())
}

async fn test_api_connection() -> Result<()> {
    let l = lang();

    let cfg = match config::load_api_config()? {
        Some(c) if c.api_token.is_some() => c,
        _ => {
            println!(
                "{} {}",
                "❌".red(),
                t!(
                    l,
                    "API not configured. Run `tunnel config set` first.",
                    "API 未配置，请先运行 `tunnel config set`。"
                )
            );
            return Ok(());
        }
    };

    let token = cfg
        .api_token
        .as_deref()
        .ok_or_else(|| anyhow::anyhow!("missing api token in config"))?;

    println!(
        "\n{}",
        t!(l, "🔍 Testing API connection...", "🔍 测试 API 连接...").bold()
    );

    // 1. Token validity
    match CloudflareClient::verify_token(token, cfg.account_id.as_deref()).await? {
        TokenVerifyStatus::Valid => {
            println!("  {} {}", "✅".green(), t!(l, "Token valid", "Token 有效"))
        }
        TokenVerifyStatus::Invalid => println!(
            "  {} {}",
            "❌".red(),
            t!(l, "Token invalid or expired", "Token 无效或已过期")
        ),
        TokenVerifyStatus::Unknown => println!(
            "  {} {}",
            "⚠️".yellow(),
            t!(l, "Token status unknown", "Token 状态未知")
        ),
    }

    // 2. Tunnel permission
    if let Some(ref _acct) = cfg.account_id {
        let client = CloudflareClient::from_config(&cfg)?;
        match client.list_tunnels().await {
            Ok(tunnels) => println!(
                "  {} {} ({} {})",
                "✅".green(),
                t!(l, "Tunnel permission", "隧道权限"),
                tunnels.len(),
                t!(l, "tunnels", "个隧道")
            ),
            Err(_) => println!(
                "  {} {}",
                "❌".red(),
                t!(l, "Tunnel permission — failed", "隧道权限 — 失败")
            ),
        }

        // 3. DNS permission
        if cfg.zone_id.is_some() {
            match client.list_dns_records().await {
                Ok(records) => println!(
                    "  {} {} ({} {})",
                    "✅".green(),
                    t!(l, "DNS permission", "DNS 权限"),
                    records.len(),
                    t!(l, "records", "条记录")
                ),
                Err(_) => println!(
                    "  {} {}",
                    "❌".red(),
                    t!(l, "DNS permission — failed", "DNS 权限 — 失败")
                ),
            }
        } else {
            println!(
                "  {} {}",
                "⚠️".yellow(),
                t!(l, "DNS — no zone configured", "DNS — 未配置域名")
            );
        }
    } else {
        println!(
            "  {} {}",
            "⚠️".yellow(),
            t!(
                l,
                "Account not set — skipping permission checks",
                "未设置账户 — 跳过权限检查"
            )
        );
    }

    Ok(())
}

pub async fn list_accounts() -> Result<()> {
    let l = lang();

    let cfg = match config::load_api_config()? {
        Some(c) if c.api_token.is_some() => c,
        _ => {
            println!(
                "{} {}",
                "❌".red(),
                t!(
                    l,
                    "API not configured. Run `tunnel config set` first.",
                    "API 未配置，请先运行 `tunnel config set`。"
                )
            );
            return Ok(());
        }
    };

    let token = cfg
        .api_token
        .as_deref()
        .ok_or_else(|| anyhow::anyhow!("missing api token in config"))?;

    let accounts = match CloudflareClient::fetch_accounts(token).await {
        Ok(v) => v,
        Err(e) => {
            println!(
                "{} {}",
                "❌".red(),
                t!(l, "Failed to fetch accounts.", "获取账户失败。")
            );
            println!("   {}", e);
            return Ok(());
        }
    };
    if accounts.is_empty() {
        println!(
            "{}",
            t!(l, "⚠️  No accounts found.", "⚠️  未找到账户。").yellow()
        );
        return Ok(());
    }

    println!("\n{}", t!(l, "📋 Accounts:", "📋 账户列表:").bold());
    let current = cfg.account_id.as_deref();
    for (idx, account) in accounts.iter().enumerate() {
        let mark = if current == Some(account.id.as_str()) {
            t!(l, " (current)", " (当前)")
        } else {
            ""
        };
        println!("{}. {} ({}){}", idx + 1, account.name, account.id, mark);
    }

    Ok(())
}

pub async fn set_account(id: Option<String>) -> Result<()> {
    let l = lang();

    let mut cfg = match config::load_api_config()? {
        Some(c) if c.api_token.is_some() => c,
        _ => {
            println!(
                "{} {}",
                "❌".red(),
                t!(
                    l,
                    "API not configured. Run `tunnel config set` first.",
                    "API 未配置，请先运行 `tunnel config set`。"
                )
            );
            return Ok(());
        }
    };

    let token = cfg
        .api_token
        .as_deref()
        .ok_or_else(|| anyhow::anyhow!("missing api token in config"))?;

    let accounts = CloudflareClient::fetch_accounts(token).await?;
    if accounts.is_empty() {
        println!(
            "{}",
            t!(l, "⚠️  No accounts found.", "⚠️  未找到账户。").yellow()
        );
        return Ok(());
    }

    let selected = if let Some(id) = id {
        match accounts.iter().find(|a| a.id == id) {
            Some(a) => a.clone(),
            None => {
                println!(
                    "{} {}",
                    "❌".red(),
                    t!(
                        l,
                        "Account ID not found in your accessible accounts.",
                        "账户 ID 不在当前 Token 可访问范围内。"
                    )
                );
                return Ok(());
            }
        }
    } else if accounts.len() == 1 {
        accounts[0].clone()
    } else {
        let items: Vec<String> = accounts
            .iter()
            .map(|a| format!("{} ({})", a.name, a.id))
            .collect();
        let sel = prompt::select_opt(t!(l, "Select account", "选择账户"), &items, None);
        match sel.and_then(|i| accounts.get(i).cloned()) {
            Some(a) => a,
            None => return Ok(()),
        }
    };

    cfg.account_id = Some(selected.id.clone());
    config::save_api_config(&cfg)?;
    println!(
        "{} {} {}",
        "✅".green(),
        t!(l, "Account set to", "已设置账户为"),
        selected.name
    );
    Ok(())
}

fn switch_language() -> Result<()> {
    let l = lang();
    let options = vec!["English", "中文"];
    let current = match l {
        crate::i18n::Lang::En => 0,
        crate::i18n::Lang::Zh => 1,
    };

    let sel = prompt::select_opt(
        t!(l, "Select language", "选择语言"),
        &options,
        Some(current),
    );

    let (code, new_lang) = match sel {
        Some(0) => ("en", crate::i18n::Lang::En),
        Some(1) => ("zh", crate::i18n::Lang::Zh),
        _ => return Ok(()),
    };

    // Save to config
    let mut cfg = config::load_api_config()?.unwrap_or_default();
    cfg.language = Some(code.to_string());
    config::save_api_config(&cfg)?;

    // Apply immediately
    crate::i18n::set_lang(new_lang);

    let l = lang();
    println!(
        "{} {}",
        "✅".green(),
        t!(l, "Language switched to English.", "语言已切换为中文。")
    );
    Ok(())
}

fn clear_config() -> Result<()> {
    let l = lang();
    let confirmed = prompt::confirm_opt(
        t!(l, "Clear all API configuration?", "确认清除所有 API 配置?"),
        false,
    )
    .unwrap_or(false);

    if confirmed {
        config::clear_api_config()?;
        println!(
            "{} {}",
            "✅".green(),
            t!(l, "Configuration cleared.", "配置已清除。")
        );
    }
    Ok(())
}

