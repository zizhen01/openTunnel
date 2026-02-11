use std::process::Command as ShellCommand;

use anyhow::Context;
use colored::Colorize;
use comfy_table::{presets::UTF8_FULL, Table};

use crate::config;
use crate::error::Result;
use crate::i18n::lang;
use crate::t;

// ---------------------------------------------------------------------------
// System status
// ---------------------------------------------------------------------------

/// Aggregated system health.
pub struct SystemStatus {
    pub service_running: bool,
    pub config_exists: bool,
    pub tunnel_name: Option<String>,
    pub mappings_count: usize,
    pub api_configured: bool,
    pub cloudflared_installed: bool,
    pub warnings: Vec<String>,
}

/// Collect real system status by inspecting the environment.
pub fn get_system_status() -> SystemStatus {
    let l = lang();

    let cloudflared_installed = is_cloudflared_installed();
    let service_running = is_service_running();
    let config_path = config::tunnel_config_path();
    let config_exists = config_path.exists();
    let api_configured = config::is_api_configured();

    let (tunnel_name, mappings_count) = match config::load_tunnel_config() {
        Ok(cfg) => {
            let count = cfg.ingress.iter().filter(|r| r.hostname.is_some()).count();
            (Some(cfg.tunnel.clone()), count)
        }
        Err(_) => (None, 0),
    };

    let mut warnings = Vec::new();

    if !cloudflared_installed {
        warnings.push(
            t!(
                l,
                "cloudflared is not installed or not in PATH",
                "cloudflared 未安装或不在 PATH 中"
            )
            .to_string(),
        );
    }
    if !config_exists {
        warnings.push(t!(l, "Tunnel config file not found", "隧道配置文件不存在").to_string());
    }
    if !api_configured {
        warnings.push(
            t!(
                l,
                "API not configured. Run `tunnel config set`",
                "API 未配置，请运行 `tunnel config set`"
            )
            .to_string(),
        );
    }

    SystemStatus {
        service_running,
        config_exists,
        tunnel_name,
        mappings_count,
        api_configured,
        cloudflared_installed,
        warnings,
    }
}

/// Pretty-print the system status block.
pub fn print_status(status: &SystemStatus) {
    let l = lang();

    println!("\n{}", t!(l, "📊 System Status", "📊 系统状态").bold());

    let yn = |b: bool| -> colored::ColoredString {
        if b {
            t!(l, "🟢 running", "🟢 运行中").green()
        } else {
            t!(l, "🔴 stopped", "🔴 已停止").red()
        }
    };
    let ok = |b: bool| -> colored::ColoredString {
        if b {
            t!(l, "✅ yes", "✅ 是").green()
        } else {
            t!(l, "❌ no", "❌ 否").red()
        }
    };

    println!(
        "├─ {}: {}",
        t!(l, "cloudflared", "cloudflared"),
        ok(status.cloudflared_installed)
    );
    println!(
        "├─ {}: {}",
        t!(l, "Service", "服务"),
        yn(status.service_running)
    );
    println!(
        "├─ {}: {}",
        t!(l, "Config", "配置"),
        ok(status.config_exists)
    );
    println!("├─ {}: {}", t!(l, "API", "API"), ok(status.api_configured));
    if let Some(name) = &status.tunnel_name {
        println!("├─ {}: {}", t!(l, "Tunnel", "隧道"), name.cyan());
    }
    println!(
        "└─ {}: {}",
        t!(l, "Mappings", "映射"),
        status.mappings_count
    );

    if !status.warnings.is_empty() {
        println!("\n⚠️  {}", t!(l, "Warnings:", "提示:").yellow().bold());
        for w in &status.warnings {
            println!("   • {}", w.yellow());
        }
    }
}

// ---------------------------------------------------------------------------
// Service control
// ---------------------------------------------------------------------------

/// Start the cloudflared service.
pub fn start_service() -> Result<()> {
    let l = lang();
    println!(
        "{}",
        t!(
            l,
            "Starting cloudflared service...",
            "正在启动 cloudflared 服务..."
        )
        .bold()
    );
    run_service_command("start")
}

/// Stop the cloudflared service.
pub fn stop_service() -> Result<()> {
    let l = lang();
    println!(
        "{}",
        t!(
            l,
            "Stopping cloudflared service...",
            "正在停止 cloudflared 服务..."
        )
        .bold()
    );
    run_service_command("stop")
}

/// Restart the cloudflared service.
pub fn restart_service() -> Result<()> {
    let l = lang();
    println!(
        "{}",
        t!(
            l,
            "Restarting cloudflared service...",
            "正在重启 cloudflared 服务..."
        )
        .bold()
    );
    run_service_command("restart")
}

/// Show detailed service status.
pub fn show_service_status() -> Result<()> {
    let l = lang();

    if cfg!(target_os = "macos") {
        let output = ShellCommand::new("launchctl")
            .args(["list"])
            .output()
            .context("failed to run launchctl")?;
        let stdout = String::from_utf8_lossy(&output.stdout);
        let found = stdout.lines().any(|line| line.contains("cloudflared"));
        if found {
            println!(
                "{} {}",
                "🟢".green(),
                t!(
                    l,
                    "cloudflared is registered with launchctl",
                    "cloudflared 已注册到 launchctl"
                )
            );
        } else {
            println!(
                "{} {}",
                "🔴".red(),
                t!(
                    l,
                    "cloudflared is not registered with launchctl",
                    "cloudflared 未注册到 launchctl"
                )
            );
        }
    } else {
        // Linux: systemctl status
        let output = ShellCommand::new("systemctl")
            .args(["status", "cloudflared", "--no-pager"])
            .output()
            .context("failed to run systemctl")?;
        let stdout = String::from_utf8_lossy(&output.stdout);
        println!("{stdout}");
    }
    Ok(())
}

fn run_service_command(action: &str) -> Result<()> {
    let l = lang();
    let output = if cfg!(target_os = "macos") {
        let plist = "com.cloudflare.cloudflared";
        match action {
            "start" => ShellCommand::new("launchctl")
                .args(["start", plist])
                .output(),
            "stop" => ShellCommand::new("launchctl")
                .args(["stop", plist])
                .output(),
            "restart" => {
                let _ = ShellCommand::new("launchctl")
                    .args(["stop", plist])
                    .output();
                std::thread::sleep(std::time::Duration::from_secs(1));
                ShellCommand::new("launchctl")
                    .args(["start", plist])
                    .output()
            }
            _ => unreachable!(),
        }
    } else {
        ShellCommand::new("sudo")
            .args(["systemctl", action, "cloudflared"])
            .output()
    }
    .context(t!(
        l,
        "failed to execute service command",
        "执行服务命令失败"
    ))?;

    if output.status.success() {
        println!("{} {}", "✅".green(), t!(l, "Done.", "完成。"));
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        println!(
            "{} {}: {}",
            "❌".red(),
            t!(l, "Failed", "失败"),
            stderr.trim()
        );
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Health check
// ---------------------------------------------------------------------------

/// Run a comprehensive health check.
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

    // 1. cloudflared installed?
    let installed = is_cloudflared_installed();
    let version = get_cloudflared_version().unwrap_or_else(|| "-".to_string());
    table.add_row(vec![
        "cloudflared",
        if installed { "✅" } else { "❌" },
        &version,
    ]);

    // 2. Service running?
    let running = is_service_running();
    table.add_row(vec![
        t!(l, "Service", "服务"),
        if running { "✅" } else { "❌" },
        if running {
            t!(l, "running", "运行中")
        } else {
            t!(l, "stopped", "已停止")
        },
    ]);

    // 3. Config file?
    let cfg_path = config::tunnel_config_path();
    let cfg_exists = cfg_path.exists();
    table.add_row(vec![
        t!(l, "Config file", "配置文件"),
        if cfg_exists { "✅" } else { "❌" },
        &cfg_path.display().to_string(),
    ]);

    // 4. API configured?
    let api_ok = config::is_api_configured();
    table.add_row(vec![
        t!(l, "API config", "API 配置"),
        if api_ok { "✅" } else { "⚠️" },
        if api_ok {
            t!(l, "configured", "已配置")
        } else {
            t!(l, "not set", "未配置")
        },
    ]);

    // 5. Metrics endpoint reachable?
    let metrics_ok = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(3))
        .build()
        .ok()
        .map(|c| {
            tokio::runtime::Handle::current()
                .block_on(async { c.get("http://127.0.0.1:20241/metrics").send().await.is_ok() })
        })
        .unwrap_or(false);

    table.add_row(vec![
        t!(l, "Metrics endpoint", "指标端点"),
        if metrics_ok { "✅" } else { "⚠️" },
        "127.0.0.1:20241",
    ]);

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
        config::tunnel_config_path().display()
    );
    println!(
        "{}: {}",
        t!(l, "API config path", "API 配置路径"),
        config::api_config_path()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|_| "unknown".to_string())
    );
    println!("{}: {}", t!(l, "Platform", "平台"), std::env::consts::OS);
    println!("{}: {}", t!(l, "Arch", "架构"), std::env::consts::ARCH);

    if let Some(v) = get_cloudflared_version() {
        println!("cloudflared: {}", v);
    }

    // Print tunnel config if available
    if let Ok(cfg) = config::load_tunnel_config() {
        println!("\n{}: {}", t!(l, "Active tunnel", "当前隧道"), cfg.tunnel);
        println!(
            "{}: {}",
            t!(l, "Ingress rules", "入口规则"),
            cfg.ingress.len()
        );
    }

    Ok(())
}

/// Export the current configuration to stdout as JSON.
pub fn export_config() -> Result<()> {
    let l = lang();

    let api_cfg = config::load_api_config()?.unwrap_or_default();
    let tunnel_cfg = config::load_tunnel_config().ok();

    let export = serde_json::json!({
        "api_config": {
            "account_id": api_cfg.account_id,
            "zone_id": api_cfg.zone_id,
            "zone_name": api_cfg.zone_name,
            "language": api_cfg.language,
            // Intentionally omit api_token for security
        },
        "tunnel_config": tunnel_cfg,
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

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn is_cloudflared_installed() -> bool {
    ShellCommand::new("cloudflared")
        .arg("version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn get_cloudflared_version() -> Option<String> {
    let output = ShellCommand::new("cloudflared")
        .arg("version")
        .output()
        .ok()?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let version = stdout.trim().lines().next()?.to_string();
    Some(version)
}

fn is_service_running() -> bool {
    if cfg!(target_os = "macos") {
        ShellCommand::new("pgrep")
            .args(["-x", "cloudflared"])
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    } else {
        ShellCommand::new("systemctl")
            .args(["is-active", "--quiet", "cloudflared"])
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }
}
