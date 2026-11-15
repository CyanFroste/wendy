use anyhow::Result;
use serde_json::Value as JsonValue;
use sqlx::SqlitePool;
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions, SqliteSynchronous};
use std::path::Path;

pub struct Store {
    name: String,
    db: Db,
}

impl Store {
    pub async fn entries(&self) -> Result<Vec<(String, JsonValue)>> {
        let data: Vec<(String, JsonValue)> =
            sqlx::query_as("SELECT key, json(value) FROM store WHERE name = ?")
                .bind(&self.name)
                .fetch_all(&self.db.pool)
                .await?;

        Ok(data)
    }

    pub async fn keys(&self) -> Result<Vec<String>> {
        let data: Vec<String> = sqlx::query_scalar("SELECT key FROM store WHERE name = ?")
            .bind(&self.name)
            .fetch_all(&self.db.pool)
            .await?;

        Ok(data)
    }

    pub async fn values(&self) -> Result<Vec<JsonValue>> {
        let data: Vec<JsonValue> =
            sqlx::query_scalar("SELECT json(value) FROM store WHERE name = ?")
                .bind(&self.name)
                .fetch_all(&self.db.pool)
                .await?;

        Ok(data)
    }

    pub async fn get(&self, key: impl AsRef<str>) -> Result<Option<JsonValue>> {
        let data: Option<JsonValue> =
            sqlx::query_scalar("SELECT json(value) FROM store WHERE name = ? AND key = ?")
                .bind(&self.name)
                .bind(key.as_ref())
                .fetch_optional(&self.db.pool)
                .await?;

        Ok(data)
    }

    pub async fn set(&self, key: impl AsRef<str>, value: JsonValue) -> Result<()> {
        sqlx::query("INSERT OR REPLACE INTO store (name, key, value) VALUES (?, ?, ?)")
            .bind(&self.name)
            .bind(key.as_ref())
            .bind(value)
            .execute(&self.db.pool)
            .await?;

        Ok(())
    }

    pub async fn delete(&self, key: impl AsRef<str>) -> Result<Option<JsonValue>> {
        let data = sqlx::query_scalar(
            "DELETE FROM store WHERE name = ? AND key = ? RETURNING json(value)",
        )
        .bind(&self.name)
        .bind(key.as_ref())
        .fetch_optional(&self.db.pool)
        .await?;

        Ok(data)
    }

    pub async fn clear(&self) -> Result<()> {
        sqlx::query("DELETE FROM store WHERE name = ?")
            .bind(&self.name)
            .execute(&self.db.pool)
            .await?;

        Ok(())
    }
}

pub struct DbState {
    db: Db,
}

impl DbState {
    pub fn new(db: Db) -> Self {
        Self { db }
    }

    pub fn get_store(&self, name: impl AsRef<str>, allowed: &[impl AsRef<str>]) -> Result<Store> {
        let name = name.as_ref();

        if !allowed.iter().any(|x| x.as_ref() == name) {
            return Err(anyhow::anyhow!("store not accessible"));
        }

        Ok(Store {
            name: name.to_string(),
            db: self.db.clone(),
        })
    }

    pub async fn init(&self) -> Result<()> {
        let store_sql = include_str!("scripts/store.sql");

        sqlx::raw_sql(store_sql).execute(&self.db.pool).await?;

        Ok(())
    }
}

#[derive(Clone)]
pub struct Db {
    pub pool: SqlitePool,
}

impl Db {
    pub fn new(path: impl AsRef<Path>) -> Self {
        let options = SqliteConnectOptions::new()
            .filename(path)
            .create_if_missing(true)
            .foreign_keys(true)
            .journal_mode(SqliteJournalMode::Wal)
            .synchronous(SqliteSynchronous::Normal);

        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .idle_timeout(None)
            .max_lifetime(None)
            .connect_lazy_with(options);

        Self { pool }
    }

    #[allow(unused)]
    pub const fn get_batch_size(&self, var_count: usize) -> usize {
        32_766 / var_count
    }
}
