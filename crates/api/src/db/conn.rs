use {
    crate::config,
    anyhow::Result,
    diesel_async::{
        AsyncPgConnection,
        pooled_connection::{
            AsyncDieselConnectionManager,
            deadpool::{Object, Pool},
        },
    },
    std::sync::LazyLock,
};

pub type DbPool = Pool<AsyncPgConnection>;
pub type DbConn = Object<AsyncPgConnection>;

static POOL: LazyLock<Pool<AsyncPgConnection>> = LazyLock::new(|| build_pool().unwrap());

pub async fn get_conn() -> Result<DbConn> {
    let conn = POOL.get().await?;

    Ok(conn)
}

fn build_pool() -> Result<Pool<AsyncPgConnection>> {
    let url = &config::config().database_url;
    let config = AsyncDieselConnectionManager::<AsyncPgConnection>::new(url.to_string());
    let pool = Pool::builder(config).build()?;
    Ok(pool)
}
