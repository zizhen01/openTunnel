use dialoguer::{theme::ColorfulTheme, Confirm, Input, Select};

/// Show a selection list and return the selected index.
/// Returns `None` when cancelled or when interaction fails.
pub fn select_opt<T: ToString>(prompt: &str, items: &[T], default: Option<usize>) -> Option<usize> {
    let theme = ColorfulTheme::default();
    let mut select = Select::with_theme(&theme).with_prompt(prompt).items(items);
    if let Some(d) = default {
        select = select.default(d);
    }
    select.interact_opt().ok().flatten()
}

/// Show a selection list and return the selected index.
/// Returns an error only when terminal interaction fails.
pub fn select_opt_result<T: ToString>(
    prompt: &str,
    items: &[T],
    default: Option<usize>,
) -> anyhow::Result<Option<usize>> {
    let theme = ColorfulTheme::default();
    let mut select = Select::with_theme(&theme).with_prompt(prompt).items(items);
    if let Some(d) = default {
        select = select.default(d);
    }
    Ok(select.interact_opt()?)
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
