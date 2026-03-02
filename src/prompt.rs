use dialoguer::{theme::ColorfulTheme, Confirm, Input, Select};

/// Show a selection list and return the selected index.
/// Appends a "← Back (ESC)" item; returns `None` when that item is chosen or ESC is pressed.
pub fn select_opt<T: ToString>(prompt: &str, items: &[T], default: Option<usize>) -> Option<usize> {
    let theme = ColorfulTheme::default();
    let mut all: Vec<String> = items.iter().map(|i| i.to_string()).collect();
    all.push("← Back (ESC)".to_string());
    let back_idx = all.len() - 1;

    let mut select = Select::with_theme(&theme).with_prompt(prompt).items(&all);
    if let Some(d) = default {
        select = select.default(d);
    }
    match select.interact_opt().ok().flatten() {
        Some(i) if i == back_idx => None,
        other => other,
    }
}

/// Show a selection list and return the selected index.
/// Appends a "← Back (ESC)" item; returns `Ok(None)` when that item is chosen or ESC is pressed.
pub fn select_opt_result<T: ToString>(
    prompt: &str,
    items: &[T],
    default: Option<usize>,
) -> anyhow::Result<Option<usize>> {
    let theme = ColorfulTheme::default();
    let mut all: Vec<String> = items.iter().map(|i| i.to_string()).collect();
    all.push("← Back (ESC)".to_string());
    let back_idx = all.len() - 1;

    let mut select = Select::with_theme(&theme).with_prompt(prompt).items(&all);
    if let Some(d) = default {
        select = select.default(d);
    }
    Ok(match select.interact_opt()? {
        Some(i) if i == back_idx => None,
        other => other,
    })
}

/// Show a confirmation prompt.
/// Returns `Some(bool)` when answered, `None` when cancelled or on interaction failure.
pub fn confirm_opt(prompt: &str, default: bool) -> Option<bool> {
    Confirm::with_theme(&ColorfulTheme::default())
        .with_prompt(prompt)
        .default(default)
        .interact_opt()
        .ok()
        .flatten()
}

/// Show a text input prompt.
/// Returns `None` when cancelled or on interaction failure.
pub fn input_opt(prompt: &str, allow_empty: bool, initial: Option<&str>) -> Option<String> {
    let theme = ColorfulTheme::default();
    let mut input = Input::<String>::with_theme(&theme).with_prompt(prompt);
    if allow_empty {
        input = input.allow_empty(true);
    }
    if let Some(v) = initial {
        input = input.with_initial_text(v);
    }
    input.interact_text().ok()
}

/// Wait for the user to press Enter.
pub fn pause(prompt: &str) {
    use std::io::{self, Write};
    print!("{}", prompt);
    let _ = io::stdout().flush();
    let _ = io::stdin().read_line(&mut String::new());
}
