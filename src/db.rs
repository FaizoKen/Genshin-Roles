use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;

pub async fn create_pool(database_url: &str) -> PgPool {
    PgPoolOptions::new()
        .max_connections(10)
        .connect(database_url)
        .await
        .expect("Failed to connect to PostgreSQL")
}

pub async fn run_migrations(pool: &PgPool) {
    sqlx::raw_sql(include_str!("../migrations/001_initial_schema.sql"))
        .execute(pool)
        .await
        .expect("Failed to run migrations");
}
