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
        r#"
        CREATE TABLE IF NOT EXISTS splits (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            user TEXT NOT NULL,
            is_down BOOLEAN NOT NULL,
            is_elevator BOOLEAN NOT NULL,
            is_encumbered BOOLEAN,
            duration_ms INTEGER NOT NULL,
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP
        );
        "#,
    )
    .execute(pool)
    .await?;
    
    Ok(())
}

/// Get all splits from the database (ordered by most recent first, utilizes idx_splits_created_at)
pub async fn get_all_splits(pool: &SqlitePool) -> Result<Vec<Split>> {
    let rows = sqlx::query("SELECT id, user, is_down, is_elevator, is_encumbered, duration_ms, created_at FROM splits ORDER BY created_at DESC")
        .fetch_all(pool)
        .await?;

    let splits = rows
        .iter()
        .map(|row| Split {
            id: row.get(0),
            user: row.get(1),
            is_down: row.get(2),
            is_elevator: row.get(3),
            is_encumbered: row.get(4),
            duration_ms: row.get(5),
            created_at: row.get(6),
        })
        .collect();

    Ok(splits)
}

/// Get the most recent split from the database
pub async fn get_most_recent_split(pool: &SqlitePool) -> Result<Option<Split>> {
    let row = sqlx::query("SELECT id, user, is_down, is_elevator, is_encumbered, duration_ms, created_at FROM splits ORDER BY created_at DESC LIMIT 1")
        .fetch_optional(pool)
        .await?;

    match row {
        Some(row) => Ok(Some(Split {
            id: row.get(0),
            user: row.get(1),
            is_down: row.get(2),
            is_elevator: row.get(3),
            is_encumbered: row.get(4),
            duration_ms: row.get(5),
            created_at: row.get(6),
        })),
        None => Ok(None),
    }
}

/// Check if a split is a world record (WR) for its category
/// A WR is when no other entry exists with the same is_down, is_elevator, and is_encumbered status
/// with a better (lower) duration
pub async fn is_world_record(pool: &SqlitePool, split: &Split) -> Result<bool> {
    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM splits 
         WHERE is_down = ?1 AND is_elevator = ?2 AND is_encumbered = ?3 AND duration_ms < ?4"
    )
    .bind(split.is_down)
    .bind(split.is_elevator)
    .bind(split.is_encumbered)
    .bind(split.duration_ms)
    .fetch_one(pool)
    .await?;

    Ok(count == 0)
}

/// Insert a new split into the database
pub async fn insert_split(pool: &SqlitePool, data: &SplitData) -> Result<()> {
    
    sqlx::query(
        "INSERT INTO splits (user, is_down, is_elevator, is_encumbered, duration_ms) VALUES (?1, ?2, ?3, ?4, ?5)"
    )
    .bind(&data.user)
    .bind(data.is_down)
    .bind(data.is_elevator)
    .bind(data.is_encumbered)
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
            
            let encumbered_text = if !split.is_elevator {
                match split.is_encumbered {
                    Some(true) => " while encumbered",
                    Some(false) => " with nothing",
                    None => "",
                }
            } else {
                ""
            };
            
            format!(
                "Entry {}: {} went {} the {}{} in {} on {}",
                split.id, split.user, direction, method, encumbered_text, formatted_duration, split.created_at
            )
        })
        .collect::<Vec<String>>()
        .join("\n")
}

/// Format a single split for display, with optional WR decoration
pub fn format_single_split(split: &Split, is_wr: bool) -> String {
    let direction = if split.is_down { "down" } else { "up" };
    let method = if split.is_elevator { "elevator" } else { "stairs" };
    let formatted_duration = DurationValidator::format_duration(split.duration_ms);
    
    let encumbered_text = if !split.is_elevator {
        match split.is_encumbered {
            Some(true) => " while encumbered",
            Some(false) => " with nothing",
            None => "",
        }
    } else {
        ""
    };
    
    let content = format!(
        "{} went {} the {}{} in {} on {}",
        split.user, direction, method, encumbered_text, formatted_duration, split.created_at
    );
    
    if is_wr {
        format!("@here NEW WR! {} 🎉", content)
    } else {
        content
    }
}
