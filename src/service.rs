use std::process::Command;

use anyhow::{anyhow, Context, Result};
use colored::Colorize;

use crate::client::CloudflareClient;
use crate::i18n::lang;
use crate::{t, tunnel};

const SERVICE_NAME: &str = "cloudflared";
const LAUNCHD_LABEL: &str = "com.cloudflare.cloudflared";
const HOMEBREW_LABEL: &str = "homebrew.mxcl.cloudflared";

/// Show system service status for cloudflared.
pub async fn status() -> Result<()> {
    let l = lang();
    ensure_cloudflared_installed()?;
    print_package_maintenance_hint();
    println!(
        "{}",
        t!(l, "🔎 Checking service status...", "🔎 正在检查服务状态...").bold()
    );

    match std::env::consts::OS {
        "linux" => run_and_print(
            Command::new("systemctl")
                .arg("status")
                .arg(SERVICE_NAME)
                .arg("--no-pager")
                .arg("-n")
                .arg("50"),
        ),
        "macos" => {
            let target = macos_find_loaded_target().ok_or_else(|| {
                anyhow!(t!(
                    l,
                    "cloudflared launchd service not loaded. Run `tunnel service install` first.",
                    "未检测到已加载的 cloudflared launchd 服务。请先运行 `tunnel service install`。"
                ))
            })?;
            let mut cmd = Command::new("launchctl");
            cmd.arg("print").arg(target);
            run_and_print(&mut cmd)
        }
        "windows" => run_and_print(Command::new("sc").arg("query").arg(SERVICE_NAME)),
        _ => Err(anyhow!(t!(
            l,
            "Service management is currently supported on Linux/macOS/Windows only.",
            "服务管理当前仅支持 Linux/macOS/Windows。"
        ))),
    }
}

/// Install and enable cloudflared service with a tunnel token.
pub async fn install(client: &CloudflareClient, tunnel_id: Option<String>) -> Result<()> {
    let l = lang();
    ensure_cloudflared_installed()?;
    print_package_maintenance_hint();
    let tunnel_id = match tunnel_id {
        Some(id) => id,
        None => match tunnel::select_tunnel(client).await? {
            Some(t_info) => t_info.id,
            None => return Ok(()),
        },
    };

    let token = client.get_tunnel_token(&tunnel_id).await?;
    println!(
        "{}",
        t!(
            l,
            "📦 Installing cloudflared service for selected tunnel...",
            "📦 正在为所选隧道安装 cloudflared 服务..."
        )
        .bold()
    );

    run_and_print(
        Command::new("cloudflared")
            .arg("service")
            .arg("install")
            .arg(token),
    )?;

    println!(
        "{} {} {}",
        "✅".green(),
        t!(l, "Service installed for tunnel", "服务已安装到隧道"),
        tunnel_id
    );
    Ok(())
}

/// Start cloudflared service.
pub fn start() -> Result<()> {
    let l = lang();
    ensure_cloudflared_installed()?;
    print_package_maintenance_hint();
    println!(
        "{}",
        t!(l, "▶️ Starting service...", "▶️ 正在启动服务...").bold()
    );
    run_control_cmd("start")
}

/// Stop cloudflared service.
pub fn stop() -> Result<()> {
    let l = lang();
    ensure_cloudflared_installed()?;
    print_package_maintenance_hint();
    println!(
        "{}",
        t!(l, "⏹ Stopping service...", "⏹ 正在停止服务...").bold()
    );
    run_control_cmd("stop")
}

/// Restart cloudflared service.
pub fn restart() -> Result<()> {
    let l = lang();
    ensure_cloudflared_installed()?;
    print_package_maintenance_hint();
    println!(
        "{}",
        t!(l, "🔄 Restarting service...", "🔄 正在重启服务...").bold()
    );
    run_control_cmd("restart")
}

/// Show recent cloudflared service logs.
pub fn logs(lines: usize) -> Result<()> {
    let l = lang();
    ensure_cloudflared_installed()?;
    print_package_maintenance_hint();
    let lines = lines.max(1);
    println!(
        "{} {}",
        t!(l, "📜 Showing recent logs:", "📜 显示最近日志:").bold(),
        lines
    );

    match std::env::consts::OS {
        "linux" => run_and_print(
            Command::new("journalctl")
                .arg("-u")
                .arg(SERVICE_NAME)
                .arg("-n")
                .arg(lines.to_string())
                .arg("--no-pager"),
        ),
        "macos" => run_and_print(
            Command::new("log")
                .arg("show")
                .arg("--last")
                .arg("10m")
                .arg("--predicate")
                .arg(format!("process == \"{SERVICE_NAME}\""))
                .arg("--style")
                .arg("compact"),
        ),
        "windows" => {
            let ps = format!(
                "Get-WinEvent -LogName System -MaxEvents {max} | \
                 Where-Object {{ $_.ProviderName -eq 'Service Control Manager' -and $_.Message -like '*{svc}*' }} | \
                 Select-Object -First {take} TimeCreated, Id, LevelDisplayName, Message | \
                 Format-Table -AutoSize",
                max = lines.saturating_mul(10),
                svc = SERVICE_NAME,
                take = lines
            );
            run_and_print(
                Command::new("powershell")
                    .arg("-NoProfile")
                    .arg("-Command")
                    .arg(ps),
            )
        }
        _ => Err(anyhow!(t!(
            l,
            "Service logs are currently supported on Linux/macOS/Windows only.",
            "服务日志当前仅支持 Linux/macOS/Windows。"
        ))),
    }
}

fn run_control_cmd(action: &str) -> Result<()> {
    let l = lang();
    match std::env::consts::OS {
        "linux" => run_and_print(
            Command::new("systemctl")
                .arg(action)
                .arg(SERVICE_NAME)
                .arg("--no-pager"),
        ),
        "macos" => {
            let target = macos_find_loaded_target();
            match action {
                "start" => {
                    if let Some(target) = target {
                        let mut cmd = Command::new("launchctl");
                        cmd.arg("kickstart").arg("-k").arg(target);
                        run_and_print(&mut cmd)
                    } else if let Some((domain, plist)) = macos_bootstrap_source() {
                        let mut bootstrap = Command::new("launchctl");
                        bootstrap.arg("bootstrap").arg(domain).arg(plist);
                        run_and_print(&mut bootstrap)?;

                        if let Some(loaded) = macos_find_loaded_target() {
                            let mut kickstart = Command::new("launchctl");
                            kickstart.arg("kickstart").arg("-k").arg(loaded);
                            run_and_print(&mut kickstart)
                        } else {
                            Err(anyhow!(
                                "launchd service bootstrap succeeded but no service target found"
                            ))
                        }
                    } else {
                        Err(anyhow!(
                            "no cloudflared plist found in common launchd paths"
                        ))
                    }
                }
                "stop" => {
                    let target = target
                        .ok_or_else(|| anyhow!("no loaded cloudflared launchd service found"))?;
                    let mut cmd = Command::new("launchctl");
                    cmd.arg("bootout").arg(target);
                    run_and_print(&mut cmd)
                }
                "restart" => {
                    if let Some(target) = target {
                        let mut cmd = Command::new("launchctl");
                        cmd.arg("kickstart").arg("-k").arg(target);
                        run_and_print(&mut cmd)
                    } else {
                        run_control_cmd("start")
                    }
                }
                _ => Err(anyhow!("unsupported action: {action}")),
            }
        }
        "windows" => {
            let mut cmd = Command::new("sc");
            match action {
                "start" | "stop" => {
                    cmd.arg(action).arg(SERVICE_NAME);
                    run_and_print(&mut cmd)
                }
                "restart" => {
                    let mut stop_cmd = Command::new("sc");
                    stop_cmd.arg("stop").arg(SERVICE_NAME);
                    run_and_print(&mut stop_cmd)?;

                    let mut start_cmd = Command::new("sc");
                    start_cmd.arg("start").arg(SERVICE_NAME);
                    run_and_print(&mut start_cmd)
                }
                _ => Err(anyhow!("unsupported action: {action}")),
            }
        }
        _ => Err(anyhow!(t!(
            l,
            "Service control is currently supported on Linux/macOS/Windows only.",
            "服务控制当前仅支持 Linux/macOS/Windows。"
        ))),
    }
}

fn run_and_print(cmd: &mut Command) -> Result<()> {
    let output = cmd.output().context("failed to execute command")?;
    if !output.stdout.is_empty() {
        print!("{}", String::from_utf8_lossy(&output.stdout));
    }
    if !output.stderr.is_empty() {
        eprint!("{}", String::from_utf8_lossy(&output.stderr));
    }
    if output.status.success() {
        Ok(())
    } else {
        Err(anyhow!("command exited with status {}", output.status))
    }
}

fn ensure_cloudflared_installed() -> Result<()> {
    if cloudflared_installed() {
        return Ok(());
    }

    let l = lang();
    let hint = match std::env::consts::OS {
        "macos" => {
            if brew_installed() {
                t!(
                    l,
                    "cloudflared is not installed. Install with Homebrew: `brew install cloudflared`.",
                    "未检测到 cloudflared。请优先使用 Homebrew 安装：`brew install cloudflared`。"
                )
            } else {
                t!(
                    l,
                    "cloudflared is not installed. Install Homebrew first, then run: `brew install cloudflared`.",
                    "未检测到 cloudflared。请先安装 Homebrew，再执行：`brew install cloudflared`。"
                )
            }
        }
        "linux" => t!(
            l,
            "cloudflared is not installed. Install it first (for example: `sudo apt install cloudflared`).",
            "未检测到 cloudflared。请先安装（例如：`sudo apt install cloudflared`）。"
        ),
        "windows" => t!(
            l,
            "cloudflared is not installed. Install it first (for example: `winget install Cloudflare.cloudflared`).",
            "未检测到 cloudflared。请先安装（例如：`winget install Cloudflare.cloudflared`）。"
        ),
        _ => t!(
            l,
            "cloudflared is not installed in PATH.",
            "PATH 中未检测到 cloudflared。"
        ),
    };

    Err(anyhow!("{hint}"))
}

fn print_package_maintenance_hint() {
    if std::env::consts::OS == "macos" && brew_has_cloudflared() {
        let l = lang();
        println!(
            "{}",
            t!(
                l,
                "ℹ️ Homebrew-managed cloudflared detected. Prefer `brew upgrade cloudflared` for updates.",
                "ℹ️ 检测到 Homebrew 管理的 cloudflared。更新请优先使用 `brew upgrade cloudflared`。"
            )
            .cyan()
        );
    }
}

fn cloudflared_installed() -> bool {
    Command::new("cloudflared")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn brew_installed() -> bool {
    Command::new("brew")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn brew_has_cloudflared() -> bool {
    if !brew_installed() {
        return false;
    }

    match Command::new("brew")
        .arg("list")
        .arg("--versions")
        .arg("cloudflared")
        .output()
    {
        Ok(output) => {
            output.status.success() && !String::from_utf8_lossy(&output.stdout).trim().is_empty()
        }
        Err(_) => false,
    }
}

fn macos_find_loaded_target() -> Option<String> {
    let uid = macos_uid()?;
    let labels = [LAUNCHD_LABEL, HOMEBREW_LABEL];
    let domains = [
        "system".to_string(),
        format!("gui/{uid}"),
        format!("user/{uid}"),
    ];
    for domain in domains {
        for label in labels {
            let target = format!("{domain}/{label}");
            if let Ok(output) = Command::new("launchctl").arg("print").arg(&target).output() {
                if output.status.success() {
                    return Some(target);
                }
            }
        }
    }
    None
}

fn macos_bootstrap_source() -> Option<(String, String)> {
    let uid = macos_uid()?;
    let home = dirs::home_dir()?;

    let mut candidates: Vec<(String, String)> = vec![(
        "system".to_string(),
        "/Library/LaunchDaemons/com.cloudflare.cloudflared.plist".to_string(),
    )];
    candidates.push((
        format!("gui/{uid}"),
        home.join("Library/LaunchAgents/com.cloudflare.cloudflared.plist")
            .display()
            .to_string(),
    ));
    candidates.push((
        format!("gui/{uid}"),
        home.join("Library/LaunchAgents/homebrew.mxcl.cloudflared.plist")
            .display()
            .to_string(),
    ));

    candidates
        .into_iter()
        .find(|(_, plist)| std::path::Path::new(plist).exists())
}

fn macos_uid() -> Option<String> {
    if let Ok(uid) = std::env::var("UID") {
        if !uid.trim().is_empty() {
            return Some(uid);
        }
    }
    let output = Command::new("id").arg("-u").output().ok()?;
    if !output.status.success() {
        return None;
    }
    let uid = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if uid.is_empty() {
        None
    } else {
        Some(uid)
    }
}
