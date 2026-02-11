use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "tunnel",
    version,
    about = "Cloudflare Tunnel Manager — manage tunnels, DNS, Access & more",
    long_about = "tunnel — an open-source CLI for managing Cloudflare Tunnels, DNS records,\n\
                   Zero Trust Access applications, and real-time monitoring.\n\n\
                   Run `tunnel` with no arguments to enter the interactive menu."
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,

    /// Language: en / zh
    #[arg(long, global = true)]
    pub lang: Option<String>,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Interactive menu / 交互式菜单
    Menu,

    // === Tunnel management ===
    /// List tunnels / 查看隧道列表
    List,
    /// Create a new tunnel / 创建新隧道
    Create {
        /// Tunnel name
        name: Option<String>,
    },
    /// Switch active tunnel / 切换隧道
    Switch,
    /// Delete a tunnel / 删除隧道
    Delete,

    // === Mapping management ===
    /// Add a domain mapping / 添加域名映射
    Map {
        /// Hostname, e.g. app.example.com
        hostname: Option<String>,
        /// Local service, e.g. http://localhost:3000
        service: Option<String>,
    },
    /// Remove a domain mapping / 移除域名映射
    Unmap {
        /// Hostname to remove
        hostname: Option<String>,
    },
    /// Show current mappings / 查看当前映射
    Show,

    // === DNS management ===
    /// DNS record management / DNS 记录管理
    Dns {
        #[command(subcommand)]
        action: DnsAction,
    },

    // === Monitoring ===
    /// Show tunnel statistics / 查看隧道统计
    Stats,
    /// Real-time monitoring / 实时监控
    Monitor,

    // === Zero Trust / Access ===
    /// Cloudflare Access management / Access 管理
    Access {
        #[command(subcommand)]
        action: AccessAction,
    },

    // === Service control ===
    /// Start cloudflared service / 启动服务
    Start,
    /// Stop cloudflared service / 停止服务
    Stop,
    /// Restart cloudflared service / 重启服务
    Restart,
    /// Show service status / 查看状态
    Status,

    // === Diagnostics ===
    /// Run health check / 健康检查
    Check,
    /// Debug mode / 调试模式
    Debug,

    // === Config ===
    /// API configuration / API 配置
    Config {
        #[command(subcommand)]
        action: ConfigAction,
    },

    // === Smart features ===
    /// Scan local services / 扫描本地服务
    Scan {
        /// Additional ports to scan (comma-separated)
        #[arg(long)]
        ports: Option<String>,
        /// Timeout in ms per port
        #[arg(long, default_value = "500")]
        timeout: u64,
    },
}

#[derive(Subcommand)]
pub enum DnsAction {
    /// List DNS records / 列出 DNS 记录
    List,
    /// Add a DNS record / 添加 DNS 记录
    Add {
        /// Record name (e.g. app)
        #[arg(long)]
        name: Option<String>,
        /// Record type: CNAME, A, AAAA, TXT, etc.
        #[arg(long, name = "type")]
        record_type: Option<String>,
        /// Record content / target
        #[arg(long)]
        content: Option<String>,
        /// Proxy through Cloudflare
        #[arg(long, default_value = "true")]
        proxied: bool,
    },
    /// Delete a DNS record / 删除 DNS 记录
    Delete {
        /// Record ID to delete
        id: Option<String>,
    },
    /// Sync tunnel routes to DNS / 同步隧道路由到 DNS
    Sync,
}

#[derive(Subcommand)]
pub enum AccessAction {
    /// List Access applications / 查看 Access 应用
    List,
    /// Create a new Access application / 创建新应用
    Create {
        /// Application name
        name: Option<String>,
        /// Application domain
        #[arg(long)]
        domain: Option<String>,
    },
    /// Delete an Access application / 删除应用
    Delete {
        /// Application ID
        id: Option<String>,
    },
    /// Manage access policies / 管理访问策略
    Policy {
        /// Application ID
        app_id: Option<String>,
    },
}

#[derive(Subcommand)]
pub enum ConfigAction {
    /// Set API token and account/zone / 设置 API Token
    Set,
    /// Show current configuration / 查看当前配置
    Show,
    /// Test API connection / 测试 API 连接
    Test,
    /// Clear saved configuration / 清除配置
    Clear,
    /// Set preferred language / 设置语言
    Lang {
        /// Language code: en / zh
        code: String,
    },
}
