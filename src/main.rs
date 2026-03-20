use std::net::TcpListener;

#[tokio::main]
async fn main() -> std::io::Result<()> {

    color_eyre::install().unwrap();

    let subscriber = get_subscriber(
        "debug".to_string(),  // default log level
        std::io::stdout,     // log to stdout
    );
    
    tracing::subscriber::set_global_default(subscriber)
        .expect("Failed to set subscriber");
    
    let pool = sqlx::sqlite::SqlitePool::connect("sqlite://sqlite.db?mode=rwc")
        .await
        .unwrap();

    sqlx::migrate!("./migrations").run(&pool).await.unwrap();

    let port = 8080;
    let address = format!("127.0.0.1:{}", port);
    let listener = TcpListener::bind(address)?;

    warp_server::startup::run(listener, pool)?.await?;

    Ok(())
    
}

use actix_web::web;
use tracing::Subscriber;
use tracing_subscriber::{EnvFilter, fmt::MakeWriter};

pub fn get_subscriber<Sink>(
    env_filter: String,
    sink: Sink,
) -> impl Subscriber + Sync + Send
where
    Sink: for<'a> MakeWriter<'a> + Send + Sync + 'static,
{
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(env_filter));
    
    tracing_subscriber::fmt()
        .with_writer(sink)
        .with_env_filter(env_filter)
        .finish()
}