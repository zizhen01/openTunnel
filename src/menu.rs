use colored::Colorize;

use crate::client::CloudflareClient;
use crate::config;
use crate::error::Result;
use crate::i18n::lang;
use crate::{access, dns, monitor, prompt, scan, t, tools, tunnel};

// ---------------------------------------------------------------------------
// Main interactive menu
// ---------------------------------------------------------------------------

/// Entry point for the interactive TUI menu.
pub async fn interactive_menu() -> Result<()> {
    loop {
        let l = lang();
        print_banner();

        let status = tools::get_system_status();
        tools::print_status(&status);

        let options = vec![
            t!(l, "🌩️  Tunnel Management", "🌩️  隧道管理"),
            t!(l, "🌐 DNS Management", "🌐 DNS 管理"),
            t!(l, "🔐 Zero Trust / Access", "🔐 Zero Trust / Access"),
            t!(l, "📊 Statistics & Monitoring", "📊 统计与监控"),
            t!(l, "🔍 Scan Local Services", "🔍 扫描本地服务"),
            t!(l, "⚙️  API Configuration", "⚙️  API 配置"),
            t!(l, "🔧 System Tools", "🔧 系统工具"),
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
            Some(0) => tunnel_menu().await,
            Some(1) => dns_menu().await,
            Some(2) => access_menu().await,
            Some(3) => monitoring_menu().await,
            Some(4) => scan::scan_local_services(None, 500).await,
            Some(5) => config_menu().await,
            Some(6) => tools_menu().await,
            Some(7) | None => {
                println!("{}", t!(l, "👋 Goodbye!", "👋 再见！").cyan());
                break;
            }
            _ => Ok(()),
        };

        // Catch errors from submenus: display and continue instead of crashing
        if let Err(e) = result {
            println!("\n{} {:#}", "❌".red(), e);
            println!();
        }
    }
    Ok(())
}

/// Run only the API token configuration wizard.
pub async fn run_config_set_wizard() -> Result<()> {
    set_api_token().await
}

fn print_banner() {
    println!("\n{}", "═".repeat(60).cyan());
    println!("{}", "  🌩️  opentunnel v0.1.0".bold().cyan());
    println!("{}", "═".repeat(60).cyan());
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

/// Ensure tunnel config exists before actions that require local config.
fn ensure_tunnel_config_ready() -> bool {
    let l = lang();
    if config::tunnel_config_path().exists() {
        return true;
    }

    println!(
        "{} {}",
        "❌".red(),
        t!(
            l,
            "Tunnel config file not found. Create / import config first.",
            "未找到隧道配置文件。请先创建或导入配置。"
        )
    );
    println!(
        "💡 {}",
        t!(
            l,
            "Expected path shown in status panel above.",
            "预期路径可在上方状态面板中查看。"
        )
    );
    false
}

// ---------------------------------------------------------------------------
// Tunnel sub-menu
// ---------------------------------------------------------------------------

async fn tunnel_menu() -> Result<()> {
    let l = lang();
    let options = vec![
        t!(l, "📋 List tunnels", "📋 查看隧道列表"),
        t!(l, "🔄 Switch tunnel", "🔄 切换隧道"),
        t!(l, "➕ Add domain mapping", "➕ 添加域名映射"),
        t!(l, "➖ Remove domain mapping", "➖ 移除域名映射"),
        t!(l, "📋 Show mappings", "📋 查看当前映射"),
        t!(l, "🆕 Create tunnel", "🆕 创建新隧道"),
        t!(l, "🗑️  Delete tunnel", "🗑️  删除隧道"),
        t!(l, "🚀 Start service", "🚀 启动服务"),
        t!(l, "🛑 Stop service", "🛑 停止服务"),
        t!(l, "🔄 Restart service", "🔄 重启服务"),
        t!(l, "◀️  Back", "◀️  返回主菜单"),
    ];

    let sel = prompt::select_opt(t!(l, "Tunnel Management", "隧道管理"), &options, None);

    match sel {
        Some(0) => {
            if let Some(client) = try_build_client() {
                tunnel::list_tunnels(&client).await?;
            }
        }
        Some(1) => {
            if let Some(client) = try_build_client() {
                tunnel::switch_tunnel(&client).await?;
            }
        }
        Some(2) => {
            if ensure_tunnel_config_ready() {
                tunnel::add_mapping(None, None).await?;
            }
        }
        Some(3) => {
            if ensure_tunnel_config_ready() {
                tunnel::remove_mapping(None).await?;
            }
        }
        Some(4) => {
            if ensure_tunnel_config_ready() {
                tunnel::show_mappings()?;
            }
        }
        Some(5) => {
            if let Some(client) = try_build_client() {
                tunnel::create_tunnel(&client, None).await?;
            }
        }
        Some(6) => {
            if let Some(client) = try_build_client() {
                tunnel::delete_tunnel(&client).await?;
            }
        }
        Some(7) => tools::start_service()?,
        Some(8) => tools::stop_service()?,
        Some(9) => tools::restart_service()?,
        Some(10) | None => {}
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
        Some(3) => {
            if ensure_tunnel_config_ready() {
                dns::sync_tunnel_routes(&client).await?;
            }
        }
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

async fn monitoring_menu() -> Result<()> {
    let l = lang();
    let options = vec![
        t!(l, "📊 Tunnel statistics", "📊 隧道统计"),
        t!(l, "📈 Real-time monitor", "📈 实时监控"),
        t!(l, "📋 Service status", "📋 服务状态"),
        t!(l, "◀️  Back", "◀️  返回主菜单"),
    ];

    let sel = prompt::select_opt(
        t!(l, "Statistics & Monitoring", "统计与监控"),
        &options,
        None,
    );

    match sel {
        Some(0) => monitor::show_stats().await?,
        Some(1) => monitor::real_time_monitor().await?,
        Some(2) => tools::show_service_status()?,
        Some(3) | None => {}
        _ => {}
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Config sub-menu
// ---------------------------------------------------------------------------

async fn config_menu() -> Result<()> {
    let l = lang();
    let options = vec![
        t!(l, "🔑 Set API Token", "🔑 设置 API Token"),
        t!(l, "📋 Show config", "📋 查看当前配置"),
        t!(l, "🧪 Test API connection", "🧪 测试 API 连接"),
        t!(l, "🗑️  Clear config", "🗑️  清除配置"),
        t!(l, "◀️  Back", "◀️  返回主菜单"),
    ];

    let sel = prompt::select_opt(t!(l, "API Configuration", "API 配置"), &options, None);

    match sel {
        Some(0) => set_api_token().await?,
        Some(1) => show_api_config()?,
        Some(2) => test_api_connection().await?,
        Some(3) => clear_config()?,
        Some(4) | None => {}
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

    let token = match prompt::input_opt("API Token", false, None) {
        Some(v) => v,
        None => return Ok(()),
    };
    if token.is_empty() {
        return Ok(());
    }

    // Verify token
    println!("{}", t!(l, "🔍 Verifying token...", "🔍 验证 Token..."));
    if !CloudflareClient::verify_token(&token).await? {
        println!(
            "{} {}",
            "❌".red(),
            t!(l, "Token is invalid.", "Token 无效。")
        );
        return Ok(());
    }
    println!("{} {}", "✅".green(), t!(l, "Token valid.", "Token 有效。"));

    // Fetch accounts
    let accounts = CloudflareClient::fetch_accounts(&token).await?;
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
        None
    };

    // Fetch zones
    let zones = CloudflareClient::fetch_zones(&token).await?;
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
        "{}",
        t!(l, "🔍 Testing API connection...", "🔍 测试 API 连接...")
    );

    if CloudflareClient::verify_token(token).await? {
        println!(
            "{} {}",
            "✅".green(),
            t!(l, "API connection successful.", "API 连接正常。")
        );
    } else {
        println!(
            "{} {}",
            "❌".red(),
            t!(
                l,
                "API connection failed. Token may be expired.",
                "API 连接失败，Token 可能已过期。"
            )
        );
    }
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

// ---------------------------------------------------------------------------
// Tools sub-menu
// ---------------------------------------------------------------------------

async fn tools_menu() -> Result<()> {
    let l = lang();
    let options = vec![
        t!(l, "🔧 Health check", "🔧 健康检查"),
        t!(l, "🐛 Debug info", "🐛 调试信息"),
        t!(l, "📦 Export config", "📦 导出配置"),
        t!(l, "📋 Service status", "📋 服务状态"),
        t!(l, "◀️  Back", "◀️  返回主菜单"),
    ];

    let sel = prompt::select_opt(t!(l, "System Tools", "系统工具"), &options, None);

    match sel {
        Some(0) => tools::health_check().await?,
        Some(1) => tools::debug_mode()?,
        Some(2) => tools::export_config()?,
        Some(3) => tools::show_service_status()?,
        Some(4) | None => {}
        _ => {}
    }
    Ok(())
}
