use std::env;

/// OAuth 2.0 authorization-code settings used by `/api/v1/auth/*`.
///
/// Any provider that exposes an authorization URL, a token URL and a JSON
/// user-info URL works (GitHub, GitLab, Google, Keycloak, Authentik, ...).
#[derive(Clone, Debug)]
pub struct OAuthConfig {
    pub client_id: String,
    pub client_secret: String,
    pub auth_url: String,
    pub token_url: String,
    pub userinfo_url: String,
    pub scopes: String,
    pub redirect_url: String,
    /// JSON field of the user-info response that holds the e-mail address.
    pub email_field: String,
    /// JSON field of the user-info response used as the preferred username.
    pub username_field: String,
}

#[derive(Clone, Debug)]
pub struct Config {
    pub database_url: String,
    pub host: String,
    pub port: u16,
    /// Externally reachable base URL (used for OAuth redirects).
    pub public_url: String,
    /// Create an account the first time an unknown user logs in through OAuth.
    pub auto_register: bool,
    pub session_ttl_hours: i64,
    /// If set and the user table is empty, an admin with this e-mail is created
    /// on startup and its API key is printed once.
    pub bootstrap_admin_email: Option<String>,
    pub bootstrap_admin_username: Option<String>,
    pub oauth: Option<OAuthConfig>,
}

fn env_opt(key: &str) -> Option<String> {
    env::var(key).ok().filter(|v| !v.trim().is_empty())
}

fn env_bool(key: &str, default: bool) -> Result<bool, String> {
    match env_opt(key) {
        None => Ok(default),
        Some(v) => match v.trim().to_ascii_lowercase().as_str() {
            "1" | "true" | "yes" | "on" => Ok(true),
            "0" | "false" | "no" | "off" => Ok(false),
            other => Err(format!("{key}: expected a boolean, got '{other}'")),
        },
    }
}

impl Config {
    /// Sensible local defaults: SQLite file in the working directory, no OAuth.
    pub fn local() -> Self {
        Self {
            database_url: "sqlite://sqlite.db?mode=rwc".to_string(),
            host: "127.0.0.1".to_string(),
            port: 8080,
            public_url: "http://127.0.0.1:8080".to_string(),
            auto_register: true,
            session_ttl_hours: 24 * 30,
            bootstrap_admin_email: None,
            bootstrap_admin_username: None,
            oauth: None,
        }
    }

    /// Build the configuration from environment variables (see README).
    pub fn from_env() -> Result<Self, String> {
        let mut cfg = Self::local();

        if let Some(v) = env_opt("DATABASE_URL") {
            cfg.database_url = v;
        }
        if let Some(v) = env_opt("WARP_HOST") {
            cfg.host = v;
        }
        if let Some(v) = env_opt("WARP_PORT") {
            cfg.port = v
                .parse()
                .map_err(|_| format!("WARP_PORT: expected a port number, got '{v}'"))?;
        }
        cfg.public_url = env_opt("WARP_PUBLIC_URL")
            .map(|u| u.trim_end_matches('/').to_string())
            .unwrap_or_else(|| format!("http://{}:{}", cfg.host, cfg.port));
        cfg.auto_register = env_bool("WARP_AUTO_REGISTER", cfg.auto_register)?;
        if let Some(v) = env_opt("WARP_SESSION_TTL_HOURS") {
            cfg.session_ttl_hours = v
                .parse()
                .map_err(|_| format!("WARP_SESSION_TTL_HOURS: expected an integer, got '{v}'"))?;
        }
        cfg.bootstrap_admin_email = env_opt("WARP_BOOTSTRAP_ADMIN_EMAIL");
        cfg.bootstrap_admin_username = env_opt("WARP_BOOTSTRAP_ADMIN_USERNAME");

        if let Some(client_id) = env_opt("OAUTH_CLIENT_ID") {
            let required = |key: &str| {
                env_opt(key).ok_or_else(|| format!("{key} is required when OAUTH_CLIENT_ID is set"))
            };
            cfg.oauth = Some(OAuthConfig {
                client_id,
                client_secret: required("OAUTH_CLIENT_SECRET")?,
                auth_url: required("OAUTH_AUTH_URL")?,
                token_url: required("OAUTH_TOKEN_URL")?,
                userinfo_url: required("OAUTH_USERINFO_URL")?,
                scopes: env_opt("OAUTH_SCOPES").unwrap_or_else(|| "openid email profile".into()),
                redirect_url: env_opt("OAUTH_REDIRECT_URL")
                    .unwrap_or_else(|| format!("{}/api/v1/auth/o/callback", cfg.public_url)),
                email_field: env_opt("OAUTH_EMAIL_FIELD").unwrap_or_else(|| "email".into()),
                username_field: env_opt("OAUTH_USERNAME_FIELD")
                    .unwrap_or_else(|| "preferred_username".into()),
            });
        }

        Ok(cfg)
    }

    pub fn session_ttl(&self) -> chrono::Duration {
        chrono::Duration::hours(self.session_ttl_hours.max(1))
    }
}
