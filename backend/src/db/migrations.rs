// backend/src/db/migrations.rs
use sqlx::{postgres::PgPoolOptions, PgPool};
use std::time::Duration;
use tracing::info;

pub async fn create_pool() -> PgPool {
    let db_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    create_pool_with_url(&db_url).await
}

pub async fn create_pool_with_url(db_url: &str) -> PgPool {
    let pool = PgPoolOptions::new()
        .max_connections(30) 
        .min_connections(5)
        .acquire_timeout(Duration::from_secs(30)) 
        .idle_timeout(Duration::from_secs(360)) 
        .test_before_acquire(true) 
        .connect(db_url)
        .await
        .expect("Failed to connect to the database");

    info!("Running database migrations...");
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("Failed to run database migrations");
    info!("Database migrations completed.");

    pool
}

