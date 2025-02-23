use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode};
use sqlx::{SqlitePool, migrate};
use std::str::FromStr;
use std::{env, io, net};
use tokio::net::TcpListener;

#[derive(Debug)]
pub struct Config {
    port: u16,
    db_url: String,
    max_upload_size_mb: usize,
}

fn get_var(name: &str) -> String {
    env::var(name).unwrap_or_else(|_| panic!("`{name}` missing"))
}

impl Config {
    pub fn read_from_env() -> Self {
        let port = get_var("PORT").parse().unwrap();
        let db_url = get_var("DATABASE_URL");
        let max_upload_size_mb = get_var("MAX_UPLOAD_SIZE_MB").parse().unwrap();
        Self {
            port,
            db_url,
            max_upload_size_mb,
        }
    }

    pub async fn listener(&self) -> io::Result<TcpListener> {
        TcpListener::bind(net::SocketAddr::from(([127, 0, 0, 1], self.port))).await
    }

    pub async fn connection_pool(&self) -> sqlx::Result<SqlitePool> {
        let opt = SqliteConnectOptions::from_str(&self.db_url)?
            .create_if_missing(true)
            .journal_mode(SqliteJournalMode::Wal);
        let pool = SqlitePool::connect_with(opt).await?;

        migrate!().run(&pool).await?;
        Ok(pool)
    }

    pub fn max_upload_size(&self) -> usize {
        1000 * 1000 * self.max_upload_size_mb
    }

    #[allow(dead_code)] // used for testing
    pub fn test_config(max_upload_size_mb: usize) -> Self {
        Self {
            port: 0,
            db_url: ":memory:".to_string(),
            max_upload_size_mb,
        }
    }
}
