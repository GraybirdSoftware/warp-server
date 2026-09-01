use std::net::TcpListener;

use color_eyre::eyre::{Result, bail, eyre};
use warp_server::{
    bootstrap,
    config::Config,
    db::{self, Db, api_keys, users},
    models::core::Role,
    startup, telemetry,
};

const USAGE: &str = "\
warp-server — self-hosted Binary Ninja WARP server

USAGE:
    warp-server [serve]                          run the HTTP server (default)
    warp-server admin create <email> [username]  create an admin and print an API key
    warp-server admin key <email> [key-name]     mint a new API key for an existing user
    warp-server admin promote <email>            grant the Admin role to a user
    warp-server help

Configuration is read from the environment (and .env); see README.md.";

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();
    color_eyre::install()?;
    telemetry::init("info,sqlx=warn");

    let config = Config::from_env().map_err(|e| eyre!(e))?;
    let args: Vec<String> = std::env::args().skip(1).collect();

    match args.first().map(String::as_str) {
        None | Some("serve") => serve(config).await,
        Some("admin") => admin(config, &args[1..]).await,
        Some("help" | "--help" | "-h") => {
            println!("{USAGE}");
            Ok(())
        }
        Some(other) => bail!("unknown command '{other}'\n\n{USAGE}"),
    }
}

async fn connect(config: &Config) -> Result<Db> {
    db::connect_and_migrate(&config.database_url)
        .await
        .map_err(|e| eyre!("failed to open database '{}': {e}", config.database_url))
}

async fn serve(config: Config) -> Result<()> {
    let pool = connect(&config).await?;
    bootstrap::ensure_bootstrap_admin(&pool, &config).await?;

    let address = format!("{}:{}", config.host, config.port);
    let listener = TcpListener::bind(&address)?;
    tracing::info!(
        "listening on http://{address} (public URL {}, OAuth {})",
        config.public_url,
        if config.oauth.is_some() {
            "enabled"
        } else {
            "disabled"
        }
    );

    startup::run(listener, pool, config)?.await?;
    Ok(())
}

async fn admin(config: Config, args: &[String]) -> Result<()> {
    let pool = connect(&config).await?;
    match args {
        [cmd, email, rest @ ..] if cmd == "create" => {
            let username = rest.first().map(String::as_str);
            let (user, key) =
                bootstrap::create_user_with_key(&pool, email, username, Role::Admin, "cli").await?;
            println!(
                "Created admin '{}' <{}> (id {}).",
                user.username, user.email, user.id
            );
            println!("API key (shown once, store it now): {}", key.key);
        }
        [cmd, email, rest @ ..] if cmd == "key" => {
            let user = users::find_by_email(&pool, email)
                .await?
                .ok_or_else(|| eyre!("no user with email '{email}'"))?;
            let name = rest.first().map(String::as_str).unwrap_or("cli");
            let key = api_keys::create(&pool, user.id, name).await?;
            println!(
                "API key '{}' for '{}' (shown once): {}",
                key.name, user.username, key.key
            );
        }
        [cmd, email] if cmd == "promote" => {
            let user = users::find_by_email(&pool, email)
                .await?
                .ok_or_else(|| eyre!("no user with email '{email}'"))?;
            sqlx::query("UPDATE users SET role = ? WHERE id = ?")
                .bind(Role::Admin.as_str())
                .bind(user.id)
                .execute(&pool)
                .await?;
            println!("'{}' is now an admin.", user.username);
        }
        _ => bail!("invalid admin command\n\n{USAGE}"),
    }
    Ok(())
}
