use {
    crate::db::conn::get_conn,
    diesel_async::AsyncMigrationHarness,
    diesel_migrations::{EmbeddedMigrations, MigrationHarness, embed_migrations},
};

const MIGRATIONS: EmbeddedMigrations = embed_migrations!("../../migrations");

pub async fn run_migrations() {
    let conn = get_conn().await.unwrap();
    let mut harness = AsyncMigrationHarness::new(conn);

    harness.run_pending_migrations(MIGRATIONS).unwrap();
}
