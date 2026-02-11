use std::sync::OnceLock;

/// Supported languages.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lang {
    En,
    Zh,
}

static CURRENT_LANG: OnceLock<Lang> = OnceLock::new();

/// Initialise the global language.
/// Priority: CLI flag > `CFT_LANG` env > config file > system locale > default `En`.
pub fn init_lang(cli_flag: Option<&str>, config_lang: Option<&str>) {
    let lang = resolve_lang(cli_flag, config_lang);
    let _ = CURRENT_LANG.set(lang);
}

/// Return the active language (defaults to `En` if uninitialised).
pub fn lang() -> Lang {
    CURRENT_LANG.get().copied().unwrap_or(Lang::En)
}

fn resolve_lang(cli_flag: Option<&str>, config_lang: Option<&str>) -> Lang {
    // 1. CLI flag (highest priority)
    if let Some(flag) = cli_flag {
        if let Some(l) = parse_lang(flag) {
            return l;
        }
    }

    // 2. CFT_LANG environment variable
    if let Ok(env_val) = std::env::var("CFT_LANG") {
        if let Some(l) = parse_lang(&env_val) {
            return l;
        }
    }

    // 3. Config file preference
    if let Some(cfg) = config_lang {
        if let Some(l) = parse_lang(cfg) {
            return l;
        }
    }

    // 4. System locale
    if let Ok(locale) = std::env::var("LANG").or_else(|_| std::env::var("LC_ALL")) {
        let lower = locale.to_lowercase();
        if lower.starts_with("zh") {
            return Lang::Zh;
        }
    }

    // 5. Default
    Lang::En
}

fn parse_lang(s: &str) -> Option<Lang> {
    match s.to_lowercase().as_str() {
        "en" | "english" => Some(Lang::En),
        "zh" | "cn" | "chinese" | "中文" => Some(Lang::Zh),
        _ => None,
    }
}

/// Bilingual text selection macro.
///
/// ```
/// use tunnel::i18n::{Lang, t};
/// let lang = Lang::En;
/// assert_eq!(t!(lang, "Hello", "你好"), "Hello");
/// ```
#[macro_export]
macro_rules! t {
    ($lang:expr, $en:expr, $zh:expr) => {
        match $lang {
            $crate::i18n::Lang::En => $en,
            $crate::i18n::Lang::Zh => $zh,
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_known_languages() {
        assert_eq!(parse_lang("en"), Some(Lang::En));
        assert_eq!(parse_lang("zh"), Some(Lang::Zh));
        assert_eq!(parse_lang("cn"), Some(Lang::Zh));
        assert_eq!(parse_lang("English"), Some(Lang::En));
        assert_eq!(parse_lang("中文"), Some(Lang::Zh));
        assert_eq!(parse_lang("fr"), None);
    }

    #[test]
    fn t_macro_selects_correctly() {
        assert_eq!(t!(Lang::En, "Hello", "你好"), "Hello");
        assert_eq!(t!(Lang::Zh, "Hello", "你好"), "你好");
    }
}
