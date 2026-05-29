use anyhow::{Context, Result};
use sqlx::SqlitePool;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use tokio::sync::OnceCell;

static POOL: OnceCell<SqlitePool> = OnceCell::const_new();

pub(crate) async fn pool() -> &'static SqlitePool {
    POOL.get_or_init(|| async {
        build_pool()
            .await
            .expect("failed to initialize sqlite pool")
    })
    .await
}

#[cfg(not(test))]
async fn build_pool() -> Result<SqlitePool> {
    use std::str::FromStr;
    let url = std::env::var("DATABASE_URL")?;
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
    sqlx::migrate!("../../migrations-sqlite")
        .run(&pool)
        .await
        .context("failed to run migrations")?;
    Ok(pool)
}

/// Tests use a single-connection in-memory database. Each `nextest` test runs in
/// its own process, so the `OnceCell` gives each test a fresh, isolated db.
#[cfg(test)]
async fn build_pool() -> Result<SqlitePool> {
    let opts = SqliteConnectOptions::new().foreign_keys(true);
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(opts)
        .await
        .context("failed to open in-memory sqlite")?;
    sqlx::migrate!("../../migrations-sqlite")
        .run(&pool)
        .await
        .context("failed to run migrations")?;
    Ok(pool)
}
