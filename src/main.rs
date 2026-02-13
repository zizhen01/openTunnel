mod access;
mod cli;
mod client;
mod config;
mod dns;
mod error;
mod i18n;
mod menu;
mod monitor;
mod prompt;
mod scan;
mod tools;
mod tunnel;

use clap::Parser;
use colored::Colorize;

use cli::{AccessAction, AccountAction, Cli, Commands, ConfigAction, DnsAction};
use error::Result;
use i18n::lang;

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    // Initialise i18n from CLI flag + saved config
    let config_lang = config::load_api_config()
        .ok()
        .flatten()
        .and_then(|c| c.language.clone());
    i18n::init_lang(cli.lang.as_deref(), config_lang.as_deref());

    if let Err(e) = run(cli).await {
        eprintln!("{} {:#}", "error:".red().bold(), e);
        std::process::exit(1);
    }
}

async fn run(cli: Cli) -> Result<()> {
    match cli.command {
        None | Some(Commands::Menu) => menu::interactive_menu().await,

        // Tunnel management
        Some(Commands::List) => {
            let client = require_client()?;
            tunnel::list_tunnels(&client).await
        }
        Some(Commands::Create { name }) => {
            let client = require_client()?;
            tunnel::create_tunnel(&client, name).await
        }
        Some(Commands::Delete) => {
            let client = require_client()?;
            tunnel::delete_tunnel(&client).await
        }
        Some(Commands::Token { id }) => {
            let client = require_client()?;
            tunnel::get_token(&client, id).await
        }

        // Mapping management (remotely-managed via API)
        Some(Commands::Map {
            tunnel: tid,
            hostname,
            service,
        }) => {
            let client = require_client()?;
            tunnel::add_mapping(&client, tid, hostname, service).await
        }
        Some(Commands::Unmap {
            tunnel: tid,
            hostname,
        }) => {
            let client = require_client()?;
            tunnel::remove_mapping(&client, tid, hostname).await
        }
        Some(Commands::Show { id }) => {
            let client = require_client()?;
            tunnel::show_mappings(&client, id).await
        }

        // DNS
        Some(Commands::Dns { action }) => {
            let client = require_client_with_zone()?;
            match action {
                DnsAction::List => dns::list_records(&client).await,
                DnsAction::Add {
                    name,
                    record_type,
                    content,
                    proxied,
                } => dns::add_record(&client, name, record_type, content, proxied).await,
                DnsAction::Delete { id } => dns::delete_record(&client, id).await,
                DnsAction::Sync { tunnel: tid } => dns::sync_tunnel_routes(&client, tid).await,
            }
        }

        // Access
        Some(Commands::Access { action }) => {
            let client = require_client()?;
            match action {
                AccessAction::List => access::list_apps(&client).await,
                AccessAction::Create { name, domain } => {
                    access::create_app(&client, name, domain).await
                }
                AccessAction::Delete { id } => access::delete_app(&client, id).await,
                AccessAction::Policy { app_id } => access::manage_policies(&client, app_id).await,
            }
        }

        // Config
        Some(Commands::Config { action }) => match action {
            ConfigAction::Set => menu::run_config_set_wizard().await,
            ConfigAction::Account { action } => match action {
                AccountAction::List => menu::list_accounts().await,
                AccountAction::Set { id } => menu::set_account(id).await,
            },
            ConfigAction::Show => {
                print_api_config();
                Ok(())
            }
            ConfigAction::Test => {
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
                match client::CloudflareClient::verify_token(token, cfg.account_id.as_deref())
                    .await?
                {
                    client::TokenVerifyStatus::Valid => {
                        println!(
                            "{} {}",
                            "✅".green(),
                            t!(l, "API connection successful.", "API 连接正常。")
                        );
                    }
                    client::TokenVerifyStatus::Invalid => {
                        println!(
                            "{} {}",
                            "❌".red(),
                            t!(l, "API connection failed.", "API 连接失败。")
                        );
                    }
                    client::TokenVerifyStatus::Unknown => {
                        println!(
                            "{} {}",
                            "⚠️".yellow(),
                            t!(
                                l,
                                "Token verification inconclusive. Check permissions.",
                                "Token 校验不明确，请检查权限设置。"
                            )
                        );
                    }
                }
                Ok(())
            }
            ConfigAction::Clear => {
                config::clear_api_config()?;
                let l = lang();
                println!(
                    "{} {}",
                    "✅".green(),
                    t!(l, "Configuration cleared.", "配置已清除。")
                );
                Ok(())
            }
            ConfigAction::Lang { code } => {
                let mut cfg = config::load_api_config()?.unwrap_or_default();
                cfg.language = Some(code.clone());
                config::save_api_config(&cfg)?;
                let l = lang();
                println!(
                    "{} {} {}",
                    "✅".green(),
                    t!(l, "Language set to", "语言已设置为"),
                    code
                );
                Ok(())
            }
        },

        // Smart features
        Some(Commands::Scan { ports, timeout }) => scan::scan_local_services(ports, timeout).await,
    }
}

fn require_client() -> Result<client::CloudflareClient> {
    let cfg = config::require_api_config()?;
    client::CloudflareClient::from_config(&cfg)
}

fn require_client_with_zone() -> Result<client::CloudflareClient> {
    let cfg = config::require_zone_config()?;
    client::CloudflareClient::from_config(&cfg)
}

fn print_api_config() {
    let l = lang();
    match config::load_api_config() {
        Ok(Some(cfg)) => {
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
        _ => {
            println!(
                "⚠️ {}",
                t!(l, "API not configured.", "API 未配置。").yellow()
            );
        }
    }
}
