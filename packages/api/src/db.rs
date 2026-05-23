use anyhow::{Context, Result};
use sqlx::SqlitePool;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use std::str::FromStr;
use tokio::sync::OnceCell;
static POOL: OnceCell<SqlitePool> = OnceCell::const_new();
async fn build_pool() -> Result<SqlitePool> {
    let url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "sqlite://./dev/cookit.db?mode=rwc".to_string());
    if let Some(path) = url.strip_prefix("sqlite://") {
        let path = path.split('?').next().unwrap_or(path);
        if let Some(parent) = std::path::Path::new(path).parent()
            && !parent.as_os_str().is_empty()
        {
            std::fs::create_dir_all(parent).ok();
        }
    }
    let opts = SqliteConnectOptions::from_str(&url)
        .context("invalid DATABASE_URL")?
        .foreign_keys(true)
        .create_if_missing(true);
    let pool = SqlitePoolOptions::new()
        .max_connections(8)
        .connect_with(opts)
        .await
        .context("failed to connect to sqlite")?;
    sqlx::migrate!("../../migrations")
        .run(&pool)
        .await
        .context("failed to run migrations")?;
    Ok(pool)
}
pub async fn pool() -> &'static SqlitePool {
    POOL.get_or_init(|| async {
        build_pool()
            .await
            .expect("failed to initialize sqlite pool")
    })
    .await
}
