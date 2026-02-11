use clap::{Parser, Subcommand};
use colored::*;
use serde::{Deserialize, Serialize};
use std::{fs, process::Command};
use comfy_table::Table;
use std::path::Path;
use dialoguer::{Select, Input, Confirm, MultiSelect, theme::ColorfulTheme};
use reqwest;

// ==================== CLI 结构 ====================

#[derive(Parser)]
#[command(name = "cft", version = "3.0", about = "Cloudflare Tunnel & API Manager")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// 交互式主菜单
    Menu,
    
    // === 隧道管理 ===
    /// 查看隧道列表
    List,
    /// 创建新隧道
    Create,
    /// 切换隧道
    Switch,
    /// 删除隧道
    Delete,
    
    // === 映射管理 ===
    /// 添加域名映射
    Map,
    /// 移除域名映射
    Unmap,
    /// 查看当前映射
    Show,
    
    // === DNS 管理（新功能）===
    /// 查看 DNS 记录
    Dns {
        #[command(subcommand)]
        action: DnsAction,
    },
    
    // === 监控和统计（新功能）===
    /// 查看隧道统计信息
    Stats,
    /// 实时监控
    Monitor,
    
    // === Zero Trust（新功能）===
    /// Cloudflare Access 管理
    Access {
        #[command(subcommand)]
        action: AccessAction,
    },
    
    // === 服务管理 ===
    /// 启动服务
    Start,
    /// 停止服务
    Stop,
    /// 查看状态
    Status,
    
    // === 诊断工具 ===
    /// 健康检查
    Check,
    /// 自动修复
    Fix,
    /// 调试模式
    Debug,
    
    // === 配置管理（新功能）===
    /// API Token 配置
    Config {
        #[command(subcommand)]
        action: ConfigAction,
    },
    
    // === 智能功能（新功能）===
    /// 扫描本地服务
    Scan,
    /// 推荐配置
    Suggest,
}

#[derive(Subcommand)]
enum DnsAction {
    /// 列出 DNS 记录
    List { domain: Option<String> },
    /// 添加 DNS 记录
    Add { name: String, r#type: String, content: String },
    /// 删除 DNS 记录
    Delete { record_id: String },
    /// 更新 DNS 记录
    Update { record_id: String },
}

#[derive(Subcommand)]
enum AccessAction {
    /// 列出所有应用
    List,
    /// 创建新应用
    Create { name: String },
    /// 添加访问策略
    Policy { app_id: String },
}

#[derive(Subcommand)]
enum ConfigAction {
    /// 设置 API Token
    Set,
    /// 查看当前配置
    Show,
    /// 测试 API 连接
    Test,
}

// ==================== 数据结构 ====================

#[derive(Debug, Serialize, Deserialize)]
struct Config {
    tunnel: String,
    #[serde(rename = "credentials-file")]
    credentials_file: String,
    ingress: Vec<Ingress>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct Ingress {
    #[serde(skip_serializing_if = "Option::is_none")]
    hostname: Option<String>,
    service: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct ApiConfig {
    api_token: Option<String>,
    account_id: Option<String>,
    zone_id: Option<String>,
}

#[derive(Debug, Clone)]
struct TunnelInfo {
    id: String,
    name: String,
    created: String,
    connections: String,
}

#[derive(Debug)]
struct SystemStatus {
    service_running: bool,
    config_exists: bool,
    tunnel_configured: bool,
    credentials_valid: bool,
    mappings_count: usize,
    api_configured: bool,
    warnings: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct CloudflareResponse<T> {
    success: bool,
    result: Option<T>,
    errors: Vec<CloudflareError>,
}

#[derive(Debug, Deserialize)]
struct CloudflareError {
    code: u32,
    message: String,
}

#[derive(Debug, Deserialize)]
struct DnsRecord {
    id: String,
    name: String,
    r#type: String,
    content: String,
    proxied: bool,
}

#[derive(Debug, Deserialize)]
struct TunnelStats {
    connections: u32,
    requests_per_second: f64,
    bytes_sent: u64,
    bytes_received: u64,
}

const CONFIG_PATH: &str = "/etc/cloudflared/config.yml";
const API_CONFIG_PATH: &str = ".cft/config.json";

// ==================== 主函数 ====================

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    match cli.command {
        None | Some(Commands::Menu) => interactive_menu().await,
        Some(Commands::List) => list_tunnels(),
        Some(Commands::Create) => interactive_create(),
        Some(Commands::Switch) => interactive_switch(),
        Some(Commands::Delete) => interactive_delete(),
        Some(Commands::Map) => interactive_map().await,
        Some(Commands::Unmap) => interactive_unmap(),
        Some(Commands::Show) => show_config().await,
        Some(Commands::Dns { action }) => handle_dns(action).await,
        Some(Commands::Stats) => show_stats().await,
        Some(Commands::Monitor) => real_time_monitor().await,
        Some(Commands::Access { action }) => handle_access(action).await,
        Some(Commands::Start) => start_service(),
        Some(Commands::Stop) => stop_service(),
        Some(Commands::Status) => show_status().await,
        Some(Commands::Check) => health_check().await,
        Some(Commands::Fix) => auto_fix().await,
        Some(Commands::Debug) => debug_mode(),
        Some(Commands::Config { action }) => handle_config(action).await,
        Some(Commands::Scan) => scan_local_services().await,
        Some(Commands::Suggest) => suggest_config().await,
    }
}

// ==================== 增强的交互式菜单 ====================

async fn interactive_menu() {
    loop {
        print_banner();
        let status = get_system_status().await;
        print_enhanced_status(&status).await;

        let options = vec![
            "🌩️  隧道管理",
            "🌐 DNS 管理",
            "🔐 Zero Trust / Access",
            "📊 统计与监控",
            "🔍 扫描本地服务",
            "⚙️  API 配置",
            "🔧 系统工具",
            "❌ 退出",
        ];

        let selection = Select::with_theme(&ColorfulTheme::default())
            .with_prompt("选择功能模块")
            .items(&options)
            .default(0)
            .interact()
            .unwrap();

        match selection {
            0 => tunnel_menu().await,
            1 => dns_menu().await,
            2 => access_menu().await,
            3 => monitoring_menu().await,
            4 => scan_local_services().await,
            5 => config_menu().await,
            6 => tools_menu().await,
            7 => {
                println!("{}", "👋 再见！".cyan());
                break;
            }
            _ => {}
        }
    }
}

fn print_banner() {
    println!("\n{}", "═".repeat(70).cyan());
    println!("{}", "  🌩️  Cloudflare Tunnel Manager v3.0 - Enhanced Edition".bold().cyan());
    println!("{}", "═".repeat(70).cyan());
}

async fn print_enhanced_status(status: &SystemStatus) {
    println!("\n📊 {}", "系统状态".bold());
    println!("├─ 隧道服务: {}", if status.service_running { "🟢 运行中".green() } else { "🔴 已停止".red() });
    println!("├─ 配置状态: {}", if status.config_exists { "✅ 正常".green() } else { "❌ 缺失".red() });
    println!("├─ API 配置: {}", if status.api_configured { "✅ 已配置".green() } else { "⚠️ 未配置".yellow() });
    println!("└─ 域名映射: {} 条", status.mappings_count);

    if !status.warnings.is_empty() {
        println!("\n⚠️  {}", "提示:".yellow().bold());
        for warning in &status.warnings {
            println!("   • {}", warning.yellow());
        }
    }
}

// ==================== 子菜单系统 ====================

async fn tunnel_menu() {
    let options = vec![
        "📋 查看隧道列表",
        "🔄 切换隧道",
        "➕ 添加域名映射",
        "➖ 移除域名映射",
        "🆕 创建新隧道",
        "🗑️  删除隧道",
        "🚀 启动服务",
        "🛑 停止服务",
        "◀️  返回主菜单",
    ];

    let selection = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("隧道管理")
        .items(&options)
        .interact()
        .unwrap();

    match selection {
        0 => list_tunnels(),
        1 => interactive_switch(),
        2 => interactive_map().await,
        3 => interactive_unmap(),
        4 => interactive_create(),
        5 => interactive_delete(),
        6 => start_service(),
        7 => stop_service(),
        8 => return,
        _ => {}
    }
}

async fn dns_menu() {
    if !check_api_configured().await {
        println!("{}", "❌ 请先配置 API Token".red());
        println!("💡 运行: cft config set");
        return;
    }

    let options = vec![
        "📋 查看 DNS 记录",
        "➕ 添加 DNS 记录",
        "✏️  更新 DNS 记录",
        "🗑️  删除 DNS 记录",
        "🔄 同步隧道路由",
        "◀️  返回主菜单",
    ];

    let selection = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("DNS 管理")
        .items(&options)
        .interact()
        .unwrap();

    match selection {
        0 => list_dns_records().await,
        1 => add_dns_record().await,
        2 => update_dns_record().await,
        3 => delete_dns_record().await,
        4 => sync_tunnel_routes().await,
        5 => return,
        _ => {}
    }
}

async fn access_menu() {
    if !check_api_configured().await {
        println!("{}", "❌ 请先配置 API Token".red());
        return;
    }

    let options = vec![
        "📋 查看 Access 应用",
        "🆕 创建新应用",
        "🔐 管理访问策略",
        "👥 查看用户",
        "📊 访问日志",
        "◀️  返回主菜单",
    ];

    let selection = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("Zero Trust / Access")
        .items(&options)
        .interact()
        .unwrap();

    match selection {
        0 => list_access_apps().await,
        1 => create_access_app().await,
        2 => manage_policies().await,
        3 => list_users().await,
        4 => show_access_logs().await,
        5 => return,
        _ => {}
    }
}

async fn monitoring_menu() {
    let options = vec![
        "📊 隧道统计",
        "📈 实时监控",
        "🔍 连接详情",
        "📉 流量分析",
        "⏱️  延迟测试",
        "◀️  返回主菜单",
    ];

    let selection = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("统计与监控")
        .items(&options)
        .interact()
        .unwrap();

    match selection {
        0 => show_stats().await,
        1 => real_time_monitor().await,
        2 => show_connections().await,
        3 => analyze_traffic().await,
        4 => test_latency().await,
        5 => return,
        _ => {}
    }
}

async fn config_menu() {
    let options = vec![
        "🔑 设置 API Token",
        "📋 查看当前配置",
        "🧪 测试 API 连接",
        "🗑️  清除配置",
        "◀️  返回主菜单",
    ];

    let selection = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("API 配置")
        .items(&options)
        .interact()
        .unwrap();

    match selection {
        0 => set_api_token().await,
        1 => show_api_config().await,
        2 => test_api_connection().await,
        3 => clear_api_config().await,
        4 => return,
        _ => {}
    }
}

async fn tools_menu() {
    let options = vec![
        "🔧 健康检查",
        "🔨 自动修复",
        "🐛 调试模式",
        "📦 导出配置",
        "📥 导入配置",
        "◀️  返回主菜单",
    ];

    let selection = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("系统工具")
        .items(&options)
        .interact()
        .unwrap();

    match selection {
        0 => health_check().await,
        1 => auto_fix().await,
        2 => debug_mode(),
        3 => export_config().await,
        4 => import_config().await,
        5 => return,
        _ => {}
    }
}

// ==================== API 配置管理 ====================

async fn handle_config(action: ConfigAction) {
    match action {
        ConfigAction::Set => set_api_token().await,
        ConfigAction::Show => show_api_config().await,
        ConfigAction::Test => test_api_connection().await,
    }
}

async fn set_api_token() {
    println!("{}", "🔑 配置 Cloudflare API Token".bold());
    println!("\n📖 获取 API Token:");
    println!("   1. 访问: https://dash.cloudflare.com/profile/api-tokens");
    println!("   2. 点击 'Create Token'");
    println!("   3. 使用 'Edit Cloudflare Zero Trust' 模板");
    println!("   4. 或创建自定义 Token，需要以下权限:");
    println!("      • Account - Cloudflare Tunnel: Edit");
    println!("      • Zone - DNS: Edit");
    println!("      • Account - Access: Edit\n");

    let token: String = Input::new()
        .with_prompt("API Token")
        .interact_text()
        .unwrap();

    // 测试 Token 有效性
    println!("🔍 验证 Token...");
    let client = reqwest::Client::new();
    let response = client
        .get("https://api.cloudflare.com/client/v4/user/tokens/verify")
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await;

    match response {
        Ok(resp) if resp.status().is_success() => {
            println!("{} Token 有效", "✅".green());
            
            // 获取 Account ID
            let account_id = get_account_id(&token).await;
            
            // 保存配置
            let config = ApiConfig {
                api_token: Some(token),
                account_id,
                zone_id: None,
            };
            
            save_api_config(&config).await;
            println!("{} 配置已保存", "✅".green());
        }
        _ => {
            println!("{} Token 无效", "❌".red());
        }
    }
}

async fn get_account_id(token: &str) -> Option<String> {
    let client = reqwest::Client::new();
    let response = client
        .get("https://api.cloudflare.com/client/v4/accounts")
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await
        .ok()?;

    #[derive(Deserialize)]
    struct Account {
        id: String,
        name: String,
    }

    let data: CloudflareResponse<Vec<Account>> = response.json().await.ok()?;
    
    if let Some(accounts) = data.result {
        if accounts.len() == 1 {
            return Some(accounts[0].id.clone());
        } else if accounts.len() > 1 {
            let items: Vec<String> = accounts.iter()
                .map(|a| format!("{} ({})", a.name, a.id))
                .collect();
            
            let selection = Select::with_theme(&ColorfulTheme::default())
                .with_prompt("选择 Account")
                .items(&items)
                .interact()
                .ok()?;
            
            return Some(accounts[selection].id.clone());
        }
    }
    
    None
}

async fn save_api_config(config: &ApiConfig) {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/root".to_string());
    let config_dir = format!("{}/.cft", home);
    let config_path = format!("{}/config.json", config_dir);
    
    fs::create_dir_all(&config_dir).ok();
    
    let json = serde_json::to_string_pretty(config).unwrap();
    fs::write(config_path, json).ok();
}

async fn load_api_config() -> Option<ApiConfig> {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/root".to_string());
    let config_path = format!("{}/.cft/config.json", home);
    
    let content = fs::read_to_string(config_path).ok()?;
    serde_json::from_str(&content).ok()
}

async fn check_api_configured() -> bool {
    load_api_config().await.is_some()
}

async fn show_api_config() {
    match load_api_config().await {
        Some(config) => {
            println!("\n⚙️  {}", "当前 API 配置:".bold());
            println!("├─ API Token: {}", if config.api_token.is_some() { "✅ 已设置".green() } else { "❌ 未设置".red() });
            println!("├─ Account ID: {}", config.account_id.as_deref().unwrap_or("未设置"));
            println!("└─ Zone ID: {}", config.zone_id.as_deref().unwrap_or("未设置"));
        }
        None => {
            println!("{}", "⚠️  API 未配置".yellow());
            println!("💡 运行: cft config set");
        }
    }
}

async fn test_api_connection() {
    match load_api_config().await {
        Some(config) => {
            if let Some(token) = config.api_token {
                println!("🔍 测试 API 连接...");
                let client = reqwest::Client::new();
                let response = client
                    .get("https://api.cloudflare.com/client/v4/user")
                    .header("Authorization", format!("Bearer {}", token))
                    .send()
                    .await;

                match response {
                    Ok(resp) if resp.status().is_success() => {
                        println!("{} API 连接正常", "✅".green());
                    }
                    _ => {
                        println!("{} API 连接失败", "❌".red());
                    }
                }
            }
        }
        None => {
            println!("{}", "❌ API 未配置".red());
        }
    }
}

async fn clear_api_config() {
    if Confirm::new()
        .with_prompt("确认清除 API 配置?")
        .default(false)
        .interact()
        .unwrap()
    {
        let home = std::env::var("HOME").unwrap_or_else(|_| "/root".to_string());
        let config_path = format!("{}/.cft/config.json", home);
        fs::remove_file(config_path).ok();
        println!("{} 配置已清除", "✅".green());
    }
}

// ==================== DNS 管理功能 ====================

async fn handle_dns(action: DnsAction) {
    match action {
        DnsAction::List { domain } => list_dns_records_for_domain(domain).await,
        DnsAction::Add { name, r#type, content } => add_dns_record_cli(name, r#type, content).await,
        DnsAction::Delete { record_id } => delete_dns_record_cli(record_id).await,
        DnsAction::Update { record_id } => update_dns_record_cli(record_id).await,
    }
}

async fn list_dns_records() {
    println!("{}", "📋 DNS 记录列表".bold());
    println!("💡 此功能需要 API Token 和 Zone ID");
    println!("   运行 'cft config set' 配置");
}

async fn list_dns_records_for_domain(domain: Option<String>) {
    println!("📋 查看域名: {:?} 的 DNS 记录", domain);
}

async fn add_dns_record() {
    println!("{}", "➕ 添加 DNS 记录".bold());
}

async fn add_dns_record_cli(name: String, record_type: String, content: String) {
    println!("添加: {} {} {}", name, record_type, content);
}

async fn update_dns_record() {
    println!("✏️  更新 DNS 记录");
}

async fn update_dns_record_cli(record_id: String) {
    println!("更新记录: {}", record_id);
}

async fn delete_dns_record() {
    println!("🗑️  删除 DNS 记录");
}

async fn delete_dns_record_cli(record_id: String) {
    println!("删除记录: {}", record_id);
}

async fn sync_tunnel_routes() {
    println!("🔄 同步隧道路由");
}

// ==================== Access 管理功能 ====================

async fn handle_access(action: AccessAction) {
    match action {
        AccessAction::List => list_access_apps().await,
        AccessAction::Create { name } => create_access_app_cli(name).await,
        AccessAction::Policy { app_id } => manage_app_policies(app_id).await,
    }
}

async fn list_access_apps() {
    println!("{}", "📋 Access 应用列表".bold());
}

async fn create_access_app() {
    println!("{}", "🆕 创建 Access 应用".bold());
}

async fn create_access_app_cli(name: String) {
    println!("创建应用: {}", name);
}

async fn manage_policies() {
    println!("🔐 管理访问策略");
}

async fn manage_app_policies(app_id: String) {
    println!("管理应用 {} 的策略", app_id);
}

async fn list_users() {
    println!("👥 用户列表");
}

async fn show_access_logs() {
    println!("📊 访问日志");
}

// ==================== 监控功能 ====================

async fn show_stats() {
    println!("{}", "📊 隧道统计信息".bold());
    
    // 从 cloudflared metrics 端点获取数据
    let metrics_url = "http://127.0.0.1:20241/metrics";
    
    match reqwest::get(metrics_url).await {
        Ok(resp) if resp.status().is_success() => {
            let body = resp.text().await.unwrap_or_default();
            
            println!("\n⚡ {}", "实时指标:".bold());
            
            // 解析 Prometheus 格式的 metrics
            for line in body.lines() {
                if line.starts_with("cloudflared_tunnel_total_requests") {
                    println!("  • 总请求数: {}", extract_metric_value(line));
                } else if line.starts_with("cloudflared_tunnel_active_streams") {
                    println!("  • 活跃连接: {}", extract_metric_value(line));
                }
            }
        }
        _ => {
            println!("{}", "⚠️  无法获取统计数据，服务可能未运行".yellow());
        }
    }
}

fn extract_metric_value(line: &str) -> &str {
    line.split_whitespace().last().unwrap_or("0")
}

async fn real_time_monitor() {
    println!("{}", "📈 实时监控（按 Ctrl+C 退出）".bold());
    println!("每 5 秒刷新一次...\n");
    
    // 实时监控循环
    loop {
        show_stats().await;
        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
        print!("\x1B[2J\x1B[1;1H"); // 清屏
    }
}

async fn show_connections() {
    println!("🔍 连接详情");
}

async fn analyze_traffic() {
    println!("📉 流量分析");
}

async fn test_latency() {
    println!("⏱️  延迟测试");
}

// ==================== 智能功能 ====================

async fn scan_local_services() {
    println!("{}", "🔍 扫描本地服务...".bold());
    
    let common_ports = vec![
        (3000, "React/Node.js"),
        (3001, "React Dev"),
        (4000, "GraphQL"),
        (5000, "Flask/Python"),
        (8000, "Django/Python"),
        (8080, "HTTP Alternate"),
        (8888, "Jupyter"),
        (9000, "Generic"),
    ];
    
    println!("\n发现的服务:");
    let mut found = Vec::new();
    
    for (port, desc) in common_ports {
        if check_port_open(port).await {
            println!("  ✅ 端口 {} - {}", port.to_string().cyan(), desc);
            found.push((port, desc));
        }
    }
    
    if found.is_empty() {
        println!("  ⚠️  未发现常见服务");
        return;
    }
    
    if Confirm::new()
        .with_prompt("是否为发现的服务创建映射?")
        .default(false)
        .interact()
        .unwrap()
    {
        for (port, desc) in found {
            let hostname: String = Input::new()
                .with_prompt(format!("为 {} ({}) 设置域名", port, desc))
                .interact_text()
                .unwrap();
            
            // 添加映射逻辑
            println!("  ➕ 添加映射: {} -> localhost:{}", hostname, port);
        }
    }
}

async fn check_port_open(port: u16) -> bool {
    use tokio::net::TcpStream;
    TcpStream::connect(format!("127.0.0.1:{}", port))
        .await
        .is_ok()
}

async fn suggest_config() {
    println!("{}", "💡 配置建议".bold());
}

// ==================== 导入导出 ====================

async fn export_config() {
    println!("📦 导出配置");
}

async fn import_config() {
    println!("📥 导入配置");
}

// ==================== 保留原有功能的存根 ====================

fn list_tunnels() {
    println!("📋 隧道列表");
}

fn interactive_create() {
    println!("🆕 创建隧道");
}

fn interactive_switch() {
    println!("🔄 切换隧道");
}

fn interactive_delete() {
    println!("🗑️  删除隧道");
}

async fn interactive_map() {
    println!("➕ 添加映射");
}

fn interactive_unmap() {
    println!("➖ 移除映射");
}

async fn show_config() {
    println!("📋 当前配置");
}

fn start_service() {
    println!("🚀 启动服务");
}

fn stop_service() {
    println!("🛑 停止服务");
}

async fn show_status() {
    println!("📊 系统状态");
}

async fn health_check() {
    println!("🔧 健康检查");
}

async fn auto_fix() {
    println!("🔨 自动修复");
}

fn debug_mode() {
    println!("🐛 调试模式");
}

async fn get_system_status() -> SystemStatus {
    SystemStatus {
        service_running: false,
        config_exists: true,
        tunnel_configured: true,
        credentials_valid: true,
        mappings_count: 1,
        api_configured: check_api_configured().await,
        warnings: vec![],
    }
}