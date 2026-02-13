use colored::Colorize;
use comfy_table::{presets::UTF8_FULL, Table};

use crate::config;
use crate::error::Result;
use crate::i18n::lang;
use crate::t;

// ---------------------------------------------------------------------------
// System status (API-only, no local cloudflared dependency)
// ---------------------------------------------------------------------------

/// Aggregated system health.
pub struct SystemStatus {
    pub api_configured: bool,
    pub account_configured: bool,
    pub zone_configured: bool,
    pub warnings: Vec<String>,
}

/// Collect system status by checking API configuration.
pub fn get_system_status() -> SystemStatus {
    let l = lang();

    let api_configured = config::is_api_configured();
    let account_configured = config::is_account_configured();
    let zone_configured = config::load_api_config()
        .ok()
        .flatten()
        .map(|c| c.zone_id.is_some())
        .unwrap_or(false);

    let mut warnings = Vec::new();

    if !api_configured {
        warnings.push(
            t!(
                l,
                "API not configured. Run `tunnel config set`",
                "API 未配置，请运行 `tunnel config set`"
            )
            .to_string(),
        );
    } else if !account_configured {
        warnings.push(
            t!(
                l,
                "Account not selected. Run `tunnel config set`",
                "未选择账户，请运行 `tunnel config set`"
            )
            .to_string(),
        );
    }

    if api_configured && !zone_configured {
        warnings.push(
            t!(
                l,
                "Zone not configured. DNS features require a zone. Run `tunnel config set`",
                "域名未配置。DNS 功能需要设置域名。请运行 `tunnel config set`"
            )
            .to_string(),
        );
    }

    SystemStatus {
        api_configured,
        account_configured,
        zone_configured,
        warnings,
    }
}

/// Pretty-print the system status block.
pub fn print_status(status: &SystemStatus) {
    let l = lang();

    println!("\n{}", t!(l, "📊 System Status", "📊 系统状态").bold());

    let ok = |b: bool| -> colored::ColoredString {
        if b {
            t!(l, "✅ yes", "✅ 是").green()
        } else {
            t!(l, "❌ no", "❌ 否").red()
        }
    };

    println!(
        "├─ {}: {}",
        t!(l, "API Token", "API Token"),
        ok(status.api_configured)
    );
    println!(
        "├─ {}: {}",
        t!(l, "Account", "账户"),
        ok(status.account_configured)
    );
    println!(
        "└─ {}: {}",
        t!(l, "Zone (DNS)", "域名 (DNS)"),
        ok(status.zone_configured)
    );

    if !status.warnings.is_empty() {
        println!("\n⚠️  {}", t!(l, "Warnings:", "提示:").yellow().bold());
        for w in &status.warnings {
            println!("   • {}", w.yellow());
        }
    }
}

// ---------------------------------------------------------------------------
// Health check (API connectivity)
// ---------------------------------------------------------------------------

/// Run a health check by verifying API connectivity.
pub async fn health_check() -> Result<()> {
    let l = lang();
    println!(
        "\n{}",
        t!(l, "🔧 Running health check...", "🔧 运行健康检查...").bold()
    );

    let mut table = Table::new();
    table.load_preset(UTF8_FULL);
    table.set_header(vec![
        t!(l, "Check", "检查项"),
        t!(l, "Status", "状态"),
        t!(l, "Detail", "详情"),
    ]);

    // 1. API configured?
    let api_ok = config::is_api_configured();
    table.add_row(vec![
        t!(l, "API config", "API 配置"),
        if api_ok { "✅" } else { "❌" },
        if api_ok {
            t!(l, "configured", "已配置")
        } else {
            t!(
                l,
                "not set — run `tunnel config set`",
                "未配置 — 请运行 `tunnel config set`"
            )
        },
    ]);

    // 2. Account configured?
    let account_ok = config::is_account_configured();
    table.add_row(vec![
        t!(l, "Account", "账户"),
        if account_ok { "✅" } else { "❌" },
        if account_ok {
            t!(l, "selected", "已选择")
        } else {
            t!(l, "not set", "未配置")
        },
    ]);

    // 3. Token valid?
    if api_ok {
        let cfg = config::load_api_config()?.unwrap_or_default();
        let token = cfg.api_token.as_deref().unwrap_or("");
        let verify =
            crate::client::CloudflareClient::verify_token(token, cfg.account_id.as_deref()).await;
        let (status, detail) = match verify {
            Ok(crate::client::TokenVerifyStatus::Valid) => ("✅", t!(l, "valid", "有效")),
            Ok(crate::client::TokenVerifyStatus::Invalid) => {
                ("❌", t!(l, "invalid or expired", "无效或已过期"))
            }
            _ => ("⚠️", t!(l, "inconclusive", "不确定")),
        };
        table.add_row(vec![t!(l, "API Token", "API Token"), status, detail]);
    }

    println!("{table}");
    Ok(())
}

/// Print debug information.
pub fn debug_mode() -> Result<()> {
    let l = lang();
    println!("\n{}", t!(l, "🐛 Debug Information", "🐛 调试信息").bold());

    println!(
        "{}: {}",
        t!(l, "Config path", "配置路径"),
        config::api_config_path()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|_| "unknown".to_string())
    );
    println!("{}: {}", t!(l, "Platform", "平台"), std::env::consts::OS);
    println!("{}: {}", t!(l, "Arch", "架构"), std::env::consts::ARCH);

    if let Ok(Some(cfg)) = config::load_api_config() {
        println!("API Token: {}", cfg.masked_token());
        println!(
            "Account ID: {}",
            cfg.account_id.as_deref().unwrap_or("not set")
        );
        println!("Zone: {}", cfg.zone_name.as_deref().unwrap_or("not set"));
    }

    Ok(())
}

/// Export the current configuration to stdout as JSON.
pub fn export_config() -> Result<()> {
    let l = lang();

    let api_cfg = config::load_api_config()?.unwrap_or_default();

    let export = serde_json::json!({
        "api_config": {
            "account_id": api_cfg.account_id,
            "zone_id": api_cfg.zone_id,
            "zone_name": api_cfg.zone_name,
            "language": api_cfg.language,
            // Intentionally omit api_token for security
        },
    });

    println!("{}", serde_json::to_string_pretty(&export)?);
    println!(
        "\n{}",
        t!(
            l,
            "⚠️  API token omitted for security. Re-configure with `tunnel config set`.",
            "⚠️  出于安全考虑，API Token 已省略。请通过 `tunnel config set` 重新配置。"
        )
        .yellow()
    );
    Ok(())
}
