use anyhow::Result;
use sqlx::{postgres::PgPoolOptions, PgPool};
use std::time::Duration;

#[derive(Clone)]
pub struct Database {
    pool: PgPool,
}

impl Database {
    pub async fn new(database_url: &str) -> Result<Self> {
        let pool = PgPoolOptions::new()
            .max_connections(5)
            .min_connections(1)
            .acquire_timeout(Duration::from_secs(3))
            .connect(database_url)
            .await?;

        Ok(Database { pool })
    }

    pub fn get_pool(&self) -> &PgPool {
        &self.pool
    }

    pub async fn run_migrations(&self) -> Result<()> {
        sqlx::migrate!("./migrations")
            .run(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn ping(&self) -> Result<bool> {
        let result: i32 = sqlx::query_scalar("SELECT 1")
            .fetch_one(&self.pool)
            .await?;
        Ok(result == 1)
    }
}

// 测试模块
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore]
    async fn test_database_connection() {
        let db_url = std::env::var("DATABASE_URL")
            .unwrap_or_else(|_| "postgres://localhost/berry_test".to_string());
        let db = Database::new(&db_url).await.unwrap();
        assert!(db.ping().await.unwrap());
    }
}