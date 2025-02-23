use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode};
use sqlx::{SqlitePool, migrate};
use std::str::FromStr;

pub async fn get_con_pool(db_path: &str) -> sqlx::Result<SqlitePool> {
    let opt = SqliteConnectOptions::from_str(db_path)?
        .create_if_missing(true)
        .journal_mode(SqliteJournalMode::Wal);
    let pool = SqlitePool::connect_with(opt).await?;

    migrate!().run(&pool).await?;
    Ok(pool)
}
