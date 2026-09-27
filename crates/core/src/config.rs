use std::env;

#[derive(Clone, Debug)]
pub struct Config {
    /// Port the SMTP server listens on.
    pub smtp_port: u16,
    /// Port the HTTP API + web UI listens on.
    pub http_port: u16,
    /// Bind address for both servers.
    pub bind_addr: String,
    /// Path to the SQLite database file. Use ":memory:" for a non-persistent store.
    pub db_path: String,
    /// Maximum number of messages retained before the oldest are pruned. 0 = unlimited.
    pub max_messages: u64,
    /// Hostname advertised in the SMTP EHLO/HELO greeting.
    pub smtp_hostname: String,
    /// Time-to-live in seconds before messages/SMS are auto-pruned. 0 = disabled.
    pub ttl_seconds: u64,
}

impl Config {
    pub fn from_env() -> Self {
        Self {
            smtp_port: env_u16("POSTWIRE_SMTP_PORT", "SMTP_PORT", 1025),
            http_port: env_u16("POSTWIRE_HTTP_PORT", "HTTP_PORT", 8025),
            bind_addr: env_setting("POSTWIRE_BIND_ADDR", "BIND_ADDR", "0.0.0.0"),
            db_path: env_setting("POSTWIRE_DB_PATH", "DB_PATH", "postwire.db"),
            max_messages: env_setting("POSTWIRE_MAX_MESSAGES", "MAX_MESSAGES", "1000")
                .parse()
                .unwrap_or(1000),
            smtp_hostname: env_setting("POSTWIRE_SMTP_HOSTNAME", "SMTP_HOSTNAME", "postwire"),
            ttl_seconds: env_setting("POSTWIRE_TTL_SECONDS", "TTL_SECONDS", "0")
                .parse()
                .unwrap_or(0),
        }
    }
}

fn env_setting(postwire_key: &str, legacy_key: &str, default: &str) -> String {
    resolve_setting(
        env::var(postwire_key).ok().as_deref(),
        env::var(legacy_key).ok().as_deref(),
        default,
    )
}

fn env_u16(postwire_key: &str, legacy_key: &str, default: u16) -> u16 {
    env_setting(postwire_key, legacy_key, &default.to_string())
        .parse()
        .ok()
        .unwrap_or(default)
}

fn resolve_setting(postwire: Option<&str>, legacy: Option<&str>, default: &str) -> String {
    postwire
        .or(legacy)
        .map(str::to_string)
        .unwrap_or_else(|| default.to_string())
}

#[cfg(test)]
mod tests {
    use super::resolve_setting;

    #[test]
    fn postwire_setting_takes_precedence_over_legacy_setting() {
        assert_eq!(resolve_setting(Some("new"), Some("old"), "default"), "new");
    }

    #[test]
    fn legacy_setting_is_used_when_postwire_setting_is_missing() {
        assert_eq!(resolve_setting(None, Some("old"), "default"), "old");
    }

    #[test]
    fn default_is_used_when_both_settings_are_missing() {
        assert_eq!(resolve_setting(None, None, "default"), "default");
    }
}
