use crate::error::Result;
use crate::models::{Split, SplitData};
use crate::validation::DurationValidator;
use sqlx::{SqlitePool, Row};
use tracing::{debug, warn};

/// Create a sqlite database if the given file name doesn't exist
pub fn create_sqlite_database_if_does_not_exist(url: &String) -> Result<()> {
    // Create database parent directory if it doesn't exist
    let db_path = url.strip_prefix("sqlite:").unwrap_or(&url);
    if let Some(parent) = std::path::Path::new(db_path).parent() {
        std::fs::create_dir_all(parent)?;
    }
    // Create the database file itself
    if !std::path::Path::new(db_path).exists() {
        std::fs::File::create(db_path)?;
        warn!("Creating database at {db_path} as it didn't already exist!")
    } else {
        debug!("Database already exists at {db_path}");
    }

    Ok(())
}

/// Initialize the database and create tables if they don't exist
pub async fn initialize_database(pool: &SqlitePool) -> Result<()> {
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS splits (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            user TEXT NOT NULL,
            is_down BOOLEAN NOT NULL,
            is_elevator BOOLEAN NOT NULL,
            duration_ms INTEGER NOT NULL,
            timestamp TEXT NOT NULL
        )"
    )
    .execute(pool)
    .await?;
    debug!("Initialized database");
    Ok(())
}

/// Get all splits from the database
pub async fn get_all_splits(pool: &SqlitePool) -> Result<Vec<Split>> {
    let rows = sqlx::query("SELECT id, user, is_down, is_elevator, duration_ms, timestamp FROM splits")
        .fetch_all(pool)
        .await?;

    let splits = rows
        .iter()
        .map(|row| Split {
            id: row.get(0),
            user: row.get(1),
            is_down: row.get(2),
            is_elevator: row.get(3),
            duration_ms: row.get(4),
            timestamp: row.get(5),
        })
        .collect();

    Ok(splits)
}

/// Insert a new split into the database
pub async fn insert_split(pool: &SqlitePool, data: &SplitData) -> Result<()> {
    
    sqlx::query(
        "INSERT INTO splits (user, is_down, is_elevator, duration_ms, timestamp) VALUES (?1, ?2, ?3, ?4, datetime('now'))"
    )
    .bind(&data.user)
    .bind(data.is_down)
    .bind(data.is_elevator)
    .bind(data.duration_ms)
    .execute(pool)
    .await?;
    
    Ok(())
}

/// Format splits for display
pub fn format_splits(splits: &[Split]) -> String {
    splits
        .iter()
        .map(|split| {
            let direction = if split.is_down { "down" } else { "up" };
            let method = if split.is_elevator { "elevator" } else { "stairs" };
            let formatted_duration = DurationValidator::format_duration(split.duration_ms);
            format!(
                "Entry {}: {} went {} the {} in {} on {}",
                split.id, split.user, direction, method, formatted_duration, split.timestamp
            )
        })
        .collect::<Vec<String>>()
        .join("\n")
}
