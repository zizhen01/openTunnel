use std::process::Command;

use anyhow::{anyhow, Context, Result};
use colored::Colorize;

use crate::client::CloudflareClient;
use crate::i18n::lang;
use crate::prompt;
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

    // Try installing; if it fails because a service already exists, offer to reinstall
    let output = Command::new("cloudflared")
        .arg("service")
        .arg("install")
        .arg(&token)
        .output()
        .context("failed to run cloudflared service install")?;

    if output.status.success() {
        if !output.stdout.is_empty() {
            print!("{}", String::from_utf8_lossy(&output.stdout));
        }
        println!(
            "{} {} {}",
            "✅".green(),
            t!(l, "Service installed for tunnel", "服务已安装到隧道"),
            tunnel_id
        );
        prompt_start_service()?;
        return Ok(());
    }

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let combined = format!("{stdout}{stderr}");

    if combined.contains("already installed") {
        println!(
            "{}",
            t!(
                l,
                "⚠️  cloudflared service is already installed for another tunnel.",
                "⚠️  cloudflared 服务已为其他隧道安装。"
            )
            .yellow()
        );

        let prompt_msg = t!(
            l,
            "Uninstall existing service and reinstall for the new tunnel?",
            "是否卸载现有服务并重新安装到新隧道？"
        );

        match prompt::confirm_opt(prompt_msg, true) {
            Some(true) => {
                println!(
                    "{}",
                    t!(
                        l,
                        "🗑️  Uninstalling existing cloudflared service...",
                        "🗑️  正在卸载现有 cloudflared 服务..."
                    )
                    .bold()
                );
                run_and_print(Command::new("cloudflared").arg("service").arg("uninstall"))?;

                println!(
                    "{}",
                    t!(
                        l,
                        "📦 Reinstalling cloudflared service...",
                        "📦 正在重新安装 cloudflared 服务..."
                    )
                    .bold()
                );
                run_and_print(
                    Command::new("cloudflared")
                        .arg("service")
                        .arg("install")
                        .arg(&token),
                )?;

                println!(
                    "{} {} {}",
                    "✅".green(),
                    t!(l, "Service reinstalled for tunnel", "服务已重新安装到隧道"),
                    tunnel_id
                );
                prompt_start_service()?;
            }
            _ => {
                println!(
                    "{}",
                    t!(
                        l,
                        "Aborted. Existing service remains unchanged.",
                        "已中止，现有服务保持不变。"
                    )
                );
            }
        }
    } else {
        // Unknown error — print output and fail
        if !stdout.is_empty() {
            print!("{stdout}");
        }
        if !stderr.is_empty() {
            eprint!("{stderr}");
        }
        return Err(anyhow!(
            "cloudflared service install failed (exit {})",
            output.status
        ));
    }

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

/// After a successful service install, offer to start immediately.
fn prompt_start_service() -> Result<()> {
    let l = lang();
    let msg = t!(l, "Start the service now?", "是否立刻启动服务？");
    if prompt::confirm_opt(msg, true) == Some(true) {
        println!(
            "{}",
            t!(l, "▶️ Starting service...", "▶️ 正在启动服务...").bold()
        );
        run_control_cmd("start")?;
        println!(
            "{} {}",
            "✅".green(),
            t!(
                l,
                "Service is running. Tunnel should become active shortly.",
                "服务已启动，隧道应很快变为 active。"
            )
        );
    }
    Ok(())
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
    println!(
        "{}",
        t!(
            l,
            "⚠️  cloudflared is not installed on this system.",
            "⚠️  当前系统未安装 cloudflared。"
        )
        .yellow()
        .bold()
    );

    let prompt_msg = t!(
        l,
        "Would you like to install cloudflared automatically?",
        "是否自动安装 cloudflared？"
    );

    match prompt::confirm_opt(prompt_msg, true) {
        Some(true) => install_cloudflared()?,
        _ => {
            return Err(anyhow!(t!(
                l,
                "cloudflared is required but not installed. Aborted.",
                "需要 cloudflared 但未安装，已中止。"
            )));
        }
    }

    // Verify installation succeeded
    if !cloudflared_installed() {
        return Err(anyhow!(t!(
            l,
            "cloudflared installation completed but binary not found in PATH. Please check your environment.",
            "cloudflared 安装流程已完成，但未在 PATH 中找到可执行文件。请检查环境配置。"
        )));
    }

    // Print installed version
    if let Ok(output) = Command::new("cloudflared").arg("--version").output() {
        if output.status.success() {
            let ver = String::from_utf8_lossy(&output.stdout);
            println!(
                "{} {} {}",
                "✅".green(),
                t!(l, "cloudflared installed:", "cloudflared 已安装:"),
                ver.trim()
            );
        }
    }

    Ok(())
}

/// Automatically install cloudflared on the current platform.
fn install_cloudflared() -> Result<()> {
    let l = lang();
    println!(
        "{}",
        t!(
            l,
            "📦 Installing cloudflared...",
            "📦 正在安装 cloudflared..."
        )
        .bold()
    );

    match std::env::consts::OS {
        "linux" => install_cloudflared_linux(),
        "macos" => install_cloudflared_macos(),
        "windows" => install_cloudflared_windows(),
        other => Err(anyhow!(
            "{} {other}",
            t!(
                l,
                "Automatic installation is not supported on this platform:",
                "不支持在此平台自动安装："
            )
        )),
    }
}

/// Install cloudflared on Linux by downloading the official binary.
fn install_cloudflared_linux() -> Result<()> {
    let l = lang();
    let arch = std::env::consts::ARCH;
    let arch_suffix = match arch {
        "x86_64" => "amd64",
        "aarch64" => "arm64",
        "arm" => "arm",
        _ => {
            return Err(anyhow!(
                "{} {arch}",
                t!(
                    l,
                    "Unsupported architecture for automatic cloudflared installation:",
                    "不支持自动安装 cloudflared 的架构："
                )
            ))
        }
    };

    let url = format!(
        "https://github.com/cloudflare/cloudflared/releases/latest/download/cloudflared-linux-{arch_suffix}"
    );
    let install_path = "/usr/local/bin/cloudflared";

    println!(
        "  {} {} -> {}",
        t!(l, "Downloading", "下载中"),
        url,
        install_path
    );

    // Download with curl (universally available on modern Linux)
    let status = Command::new("sudo")
        .args(["curl", "-fsSL", "-o", install_path, &url])
        .status()
        .context(t!(
            l,
            "failed to run curl. Is curl installed?",
            "运行 curl 失败，是否已安装 curl？"
        ))?;

    if !status.success() {
        return Err(anyhow!(t!(
            l,
            "Failed to download cloudflared binary.",
            "下载 cloudflared 二进制文件失败。"
        )));
    }

    // Make executable
    let status = Command::new("sudo")
        .args(["chmod", "+x", install_path])
        .status()
        .context("chmod failed")?;

    if !status.success() {
        return Err(anyhow!(t!(
            l,
            "Failed to set executable permission on cloudflared.",
            "设置 cloudflared 可执行权限失败。"
        )));
    }

    println!(
        "  {} {}",
        "✅".green(),
        t!(
            l,
            "cloudflared binary installed to /usr/local/bin/cloudflared",
            "cloudflared 已安装到 /usr/local/bin/cloudflared"
        )
    );

    Ok(())
}

/// Install cloudflared on macOS via Homebrew (preferred) or direct download.
fn install_cloudflared_macos() -> Result<()> {
    let l = lang();

    if brew_installed() {
        println!(
            "  {}",
            t!(l, "Installing via Homebrew...", "通过 Homebrew 安装中...")
        );
        let status = Command::new("brew")
            .args(["install", "cloudflared"])
            .status()
            .context("failed to run brew")?;

        if !status.success() {
            return Err(anyhow!(t!(
                l,
                "Homebrew installation of cloudflared failed.",
                "通过 Homebrew 安装 cloudflared 失败。"
            )));
        }
        return Ok(());
    }

    // Fallback: direct binary download
    let arch = std::env::consts::ARCH;
    let arch_suffix = match arch {
        "x86_64" => "amd64",
        "aarch64" => "arm64",
        _ => {
            return Err(anyhow!(
                "{} {arch}. {}",
                t!(l, "Unsupported architecture:", "不支持的架构："),
                t!(
                    l,
                    "Please install Homebrew first, then run: brew install cloudflared",
                    "请先安装 Homebrew，再执行：brew install cloudflared"
                )
            ))
        }
    };

    let url = format!(
        "https://github.com/cloudflare/cloudflared/releases/latest/download/cloudflared-darwin-{arch_suffix}.tgz"
    );
    let tmp_dir = std::env::temp_dir().join("cloudflared-install");
    let tmp_dir_str = tmp_dir.display().to_string();
    let install_path = "/usr/local/bin/cloudflared";

    println!("  {} {}", t!(l, "Downloading", "下载中"), url);

    // Create temp dir, download, extract
    let _ = std::fs::create_dir_all(&tmp_dir);

    let status = Command::new("curl")
        .args(["-fsSL", "-o"])
        .arg(tmp_dir.join("cloudflared.tgz").display().to_string())
        .arg(&url)
        .status()
        .context("failed to run curl")?;

    if !status.success() {
        let _ = std::fs::remove_dir_all(&tmp_dir);
        return Err(anyhow!(t!(
            l,
            "Failed to download cloudflared.",
            "下载 cloudflared 失败。"
        )));
    }

    let status = Command::new("tar")
        .args(["-xzf"])
        .arg(tmp_dir.join("cloudflared.tgz").display().to_string())
        .arg("-C")
        .arg(&tmp_dir_str)
        .status()
        .context("failed to extract archive")?;

    if !status.success() {
        let _ = std::fs::remove_dir_all(&tmp_dir);
        return Err(anyhow!(t!(
            l,
            "Failed to extract cloudflared archive.",
            "解压 cloudflared 归档文件失败。"
        )));
    }

    let status = Command::new("sudo")
        .arg("cp")
        .arg(tmp_dir.join("cloudflared").display().to_string())
        .arg(install_path)
        .status()
        .context("failed to copy binary")?;

    if !status.success() {
        let _ = std::fs::remove_dir_all(&tmp_dir);
        return Err(anyhow!(t!(
            l,
            "Failed to install cloudflared to /usr/local/bin.",
            "安装 cloudflared 到 /usr/local/bin 失败。"
        )));
    }

    let _ = Command::new("sudo")
        .args(["chmod", "+x", install_path])
        .status();

    let _ = std::fs::remove_dir_all(&tmp_dir);

    println!(
        "  {} {}",
        "✅".green(),
        t!(
            l,
            "cloudflared binary installed to /usr/local/bin/cloudflared",
            "cloudflared 已安装到 /usr/local/bin/cloudflared"
        )
    );

    Ok(())
}

/// Install cloudflared on Windows via winget.
fn install_cloudflared_windows() -> Result<()> {
    let l = lang();
    println!(
        "  {}",
        t!(l, "Installing via winget...", "通过 winget 安装中...")
    );

    let status = Command::new("winget")
        .args([
            "install",
            "--id",
            "Cloudflare.cloudflared",
            "--accept-source-agreements",
            "--accept-package-agreements",
        ])
        .status()
        .context(t!(
            l,
            "failed to run winget. Is winget available?",
            "运行 winget 失败，是否已安装 winget？"
        ))?;

    if !status.success() {
        return Err(anyhow!(t!(
            l,
            "winget installation of cloudflared failed. You can also download manually from https://github.com/cloudflare/cloudflared/releases",
            "通过 winget 安装 cloudflared 失败。也可以从 https://github.com/cloudflare/cloudflared/releases 手动下载。"
        )));
    }

    Ok(())
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
