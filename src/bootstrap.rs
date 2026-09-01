//! First-run helpers: creating the initial admin account and API keys.

use crate::{
    config::Config,
    db::{Db, api_keys, users},
    error::ApiResult,
    models::{
        auth::ApiKeyMdl,
        core::{Role, UserMdl},
    },
};

/// Create a user and an API key for them in one transaction.
pub async fn create_user_with_key(
    pool: &Db,
    email: &str,
    username: Option<&str>,
    role: Role,
    key_name: &str,
) -> ApiResult<(UserMdl, ApiKeyMdl)> {
    let mut tx = pool.begin().await?;
    let user = users::create(
        &mut tx,
        email,
        username,
        role,
        users::UsernamePolicy::Strict,
    )
    .await?;
    let key = api_keys::create(&mut *tx, user.id, key_name).await?;
    tx.commit().await?;
    Ok((user, key))
}

/// If `WARP_BOOTSTRAP_ADMIN_EMAIL` is configured and no users exist yet,
/// create that admin and print its API key to stdout (once).
pub async fn ensure_bootstrap_admin(pool: &Db, config: &Config) -> ApiResult<()> {
    let Some(email) = config.bootstrap_admin_email.as_deref() else {
        return Ok(());
    };
    if users::count(pool).await? > 0 {
        return Ok(());
    }
    let (user, key) = create_user_with_key(
        pool,
        email,
        config.bootstrap_admin_username.as_deref(),
        Role::Admin,
        "bootstrap",
    )
    .await?;
    tracing::info!(user = %user.username, email = %user.email, "created bootstrap admin");
    println!(
        "Created bootstrap admin '{}' <{}>.\nAPI key (shown once, store it now): {}",
        user.username, user.email, key.key
    );
    Ok(())
}
