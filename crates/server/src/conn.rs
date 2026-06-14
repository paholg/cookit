use {
    crate::{config, error::DatabasePoolSnafu},
    diesel_async::{
        AsyncPgConnection,
        pooled_connection::{
            AsyncDieselConnectionManager,
            deadpool::{Object, Pool},
        },
    },
    snafu::ResultExt,
    std::sync::LazyLock,
};

pub type DbPool = Pool<AsyncPgConnection>;
pub type DbConn = Object<AsyncPgConnection>;

pub(crate) static POOL: LazyLock<Pool<AsyncPgConnection>> = LazyLock::new(build_pool);

pub async fn get_conn() -> crate::Result<DbConn> {
    let conn = POOL.get().await.context(DatabasePoolSnafu)?;

    Ok(conn)
}

fn build_pool() -> Pool<AsyncPgConnection> {
    let url = &config::config().database_url;
    let config = AsyncDieselConnectionManager::<AsyncPgConnection>::new(url.to_string());

    Pool::builder(config).build().unwrap()
}
