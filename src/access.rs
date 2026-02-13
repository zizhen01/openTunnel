use colored::Colorize;
use comfy_table::{presets::UTF8_FULL, Table};

use crate::client::{
    AccessPolicy, CloudflareClient, CreateAccessApp, PolicyEmail, PolicyEmailDomain, PolicyRule,
};
use crate::error::Result;
use crate::i18n::lang;
use crate::prompt;
use crate::t;

fn short_id(id: Option<&str>) -> String {
    id.unwrap_or("-").chars().take(8).collect()
}

// ---------------------------------------------------------------------------
// List Access applications
// ---------------------------------------------------------------------------

pub async fn list_apps(client: &CloudflareClient) -> Result<()> {
    let l = lang();
    println!(
        "{}",
        t!(
            l,
            "Fetching Access applications...",
            "获取 Access 应用列表..."
        )
        .bold()
    );

    let apps = client.list_access_apps().await?;

    if apps.is_empty() {
        println!(
            "{}",
            t!(l, "No Access applications found.", "未找到 Access 应用。")
        );
        return Ok(());
    }

    let mut table = Table::new();
    table.load_preset(UTF8_FULL);
    table.set_header(vec![
        t!(l, "Name", "名称"),
        t!(l, "Domain", "域名"),
        t!(l, "Type", "类型"),
        "ID",
    ]);

    for app in &apps {
        let id_display = short_id(app.id.as_deref());
        table.add_row(vec![
            &app.name,
            &app.domain,
            app.app_type.as_deref().unwrap_or("-"),
            &id_display,
        ]);
    }

    println!("{table}");
    println!(
        "\n{} {}",
        t!(l, "Total:", "共:"),
        apps.len().to_string().cyan()
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// Create Access application
// ---------------------------------------------------------------------------

pub async fn create_app(
    client: &CloudflareClient,
    name: Option<String>,
    domain: Option<String>,
) -> Result<()> {
    let l = lang();

    let name = match name {
        Some(n) => n,
        None => match prompt::input_opt(t!(l, "Application name", "应用名称"), false, None) {
            Some(v) => v,
            None => return Ok(()),
        },
    };

    let domain = match domain {
        Some(d) => d,
        None => match prompt::input_opt(
            t!(
                l,
                "Application domain (e.g. app.example.com)",
                "应用域名 (如 app.example.com)"
            ),
            false,
            None,
        ) {
            Some(v) => v,
            None => return Ok(()),
        },
    };

    let session_options = vec!["24h", "12h", "6h", "1h", "30m"];
    let sel = prompt::select_opt(
        t!(l, "Session duration", "会话时长"),
        &session_options,
        Some(0),
    )
    .unwrap_or(0);

    let app = CreateAccessApp {
        name: name.clone(),
        domain: domain.clone(),
        app_type: "self_hosted".to_string(),
        session_duration: session_options.get(sel).unwrap_or(&"24h").to_string(),
    };

    println!(
        "{}",
        t!(
            l,
            "Creating Access application...",
            "正在创建 Access 应用..."
        )
        .bold()
    );
    let created = client.create_access_app(&app).await?;

    println!(
        "{} {} '{}' @ {}",
        "✅".green(),
        t!(l, "Application created:", "应用已创建:"),
        name,
        domain.cyan()
    );

    // Offer to create a basic policy
    let add_policy = prompt::confirm_opt(
        t!(l, "Add an access policy now?", "现在添加访问策略?"),
        true,
    )
    .unwrap_or(false);

    if add_policy {
        if let Some(app_id) = &created.id {
            create_policy_interactive(client, app_id).await?;
        }
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Delete Access application
// ---------------------------------------------------------------------------

pub async fn delete_app(client: &CloudflareClient, id: Option<String>) -> Result<()> {
    let l = lang();

    let app_id = match id {
        Some(id) => id,
        None => {
            let apps = client.list_access_apps().await?;
            if apps.is_empty() {
                println!(
                    "{}",
                    t!(l, "No applications to delete.", "没有可删除的应用。")
                );
                return Ok(());
            }
            let items: Vec<String> = apps
                .iter()
                .map(|a| format!("{} ({})", a.name, a.domain))
                .collect();

            let sel = prompt::select_opt(
                t!(l, "Select application to delete", "选择要删除的应用"),
                &items,
                None,
            );

            match sel {
                Some(i) => match apps.get(i).and_then(|a| a.id.clone()) {
                    Some(app_id) => app_id,
                    None => {
                        println!(
                            "{} {}",
                            "❌".red(),
                            t!(
                                l,
                                "Selected application has no valid ID.",
                                "所选应用缺少有效 ID。"
                            )
                        );
                        return Ok(());
                    }
                },
                None => return Ok(()),
            }
        }
    };

    let confirmed = prompt::confirm_opt(
        t!(
            l,
            "Are you sure? This will remove all associated policies.",
            "确认删除? 这将移除所有关联的策略。"
        ),
        false,
    )
    .unwrap_or(false);

    if !confirmed {
        return Ok(());
    }

    client.delete_access_app(&app_id).await?;
    println!(
        "{} {}",
        "✅".green(),
        t!(l, "Application deleted.", "应用已删除。")
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// Manage policies
// ---------------------------------------------------------------------------

pub async fn manage_policies(client: &CloudflareClient, app_id: Option<String>) -> Result<()> {
    let l = lang();

    let app_id = match app_id {
        Some(id) => id,
        None => {
            let apps = client.list_access_apps().await?;
            if apps.is_empty() {
                println!("{}", t!(l, "No applications found.", "未找到应用。"));
                return Ok(());
            }
            let items: Vec<String> = apps
                .iter()
                .map(|a| format!("{} ({})", a.name, a.domain))
                .collect();

            let sel = prompt::select_opt(t!(l, "Select application", "选择应用"), &items, None);

            match sel {
                Some(i) => match apps.get(i).and_then(|a| a.id.clone()) {
                    Some(app_id) => app_id,
                    None => {
                        println!(
                            "{} {}",
                            "❌".red(),
                            t!(
                                l,
                                "Selected application has no valid ID.",
                                "所选应用缺少有效 ID。"
                            )
                        );
                        return Ok(());
                    }
                },
                None => return Ok(()),
            }
        }
    };

    // List existing policies
    let policies = client.list_access_policies(&app_id).await?;

    if policies.is_empty() {
        println!(
            "{}",
            t!(
                l,
                "No policies configured. Creating one...",
                "未配置策略，正在创建..."
            )
        );
        return create_policy_interactive(client, &app_id).await;
    }

    let mut table = Table::new();
    table.load_preset(UTF8_FULL);
    table.set_header(vec![t!(l, "Name", "名称"), t!(l, "Decision", "决策"), "ID"]);

    for p in &policies {
        let id_display = short_id(p.id.as_deref());
        table.add_row(vec![&p.name, &p.decision, &id_display]);
    }

    println!("{table}");

    let add_more =
        prompt::confirm_opt(t!(l, "Add another policy?", "添加新策略?"), false).unwrap_or(false);

    if add_more {
        create_policy_interactive(client, &app_id).await?;
    }

    Ok(())
}

/// Interactive policy creation wizard.
async fn create_policy_interactive(client: &CloudflareClient, app_id: &str) -> Result<()> {
    let l = lang();

    let name = match prompt::input_opt(t!(l, "Policy name", "策略名称"), false, Some("Allow")) {
        Some(v) => v,
        None => return Ok(()),
    };

    let decisions = vec!["allow", "deny", "bypass"];
    let dec_sel = prompt::select_opt(t!(l, "Decision", "决策"), &decisions, Some(0)).unwrap_or(0);

    let rule_types = vec![
        t!(
            l,
            "Email (e.g. user@example.com)",
            "邮箱地址 (如 user@example.com)"
        ),
        t!(
            l,
            "Email domain (e.g. example.com)",
            "邮箱域名 (如 example.com)"
        ),
        t!(l, "Everyone", "所有人"),
    ];

    let rule_sel =
        prompt::select_opt(t!(l, "Include rule", "包含规则"), &rule_types, Some(0)).unwrap_or(0);

    let include = match rule_sel {
        0 => {
            let email = match prompt::input_opt(t!(l, "Email address", "邮箱地址"), false, None)
            {
                Some(v) => v,
                None => return Ok(()),
            };
            vec![PolicyRule {
                email: Some(PolicyEmail { email }),
                email_domain: None,
                everyone: None,
            }]
        }
        1 => {
            let mut domain = match prompt::input_opt(
                t!(l, "Email domain", "邮箱域名"),
                false,
                Some("example.com"),
            ) {
                Some(v) => v,
                None => return Ok(()),
            };
            // Strip leading @ or extract domain from full email
            if let Some(at_pos) = domain.find('@') {
                domain = domain[at_pos + 1..].to_string();
            }
            vec![PolicyRule {
                email: None,
                email_domain: Some(PolicyEmailDomain { domain }),
                everyone: None,
            }]
        }
        _ => vec![PolicyRule {
            email: None,
            email_domain: None,
            everyone: Some(serde_json::json!({})),
        }],
    };

    let policy = AccessPolicy {
        id: None,
        name,
        decision: decisions.get(dec_sel).unwrap_or(&"allow").to_string(),
        include,
        exclude: vec![],
        require: vec![],
    };

    client.create_access_policy(app_id, &policy).await?;
    println!(
        "{} {}",
        "✅".green(),
        t!(l, "Policy created.", "策略已创建。")
    );
    Ok(())
}
