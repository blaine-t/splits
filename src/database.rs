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
    let count: i64 = if split.is_elevator {
        // For elevator splits, ignore is_encumbered (it's always None)
        sqlx::query_scalar(
            "SELECT COUNT(*) FROM splits 
             WHERE is_down = ?1 AND is_elevator = ?2 AND duration_ms < ?3"
        )
        .bind(split.is_down)
        .bind(split.is_elevator)
        .bind(split.duration_ms)
        .fetch_one(pool)
        .await?
    } else {
        // For stairs splits, include is_encumbered in comparison
        sqlx::query_scalar(
            "SELECT COUNT(*) FROM splits 
             WHERE is_down = ?1 AND is_elevator = ?2 AND is_encumbered = ?3 AND duration_ms < ?4"
        )
        .bind(split.is_down)
        .bind(split.is_elevator)
        .bind(split.is_encumbered)
        .bind(split.duration_ms)
        .fetch_one(pool)
        .await?
    };

    Ok(count == 0)
}

/// Check if the split data matches the user's most recent entry duration
async fn is_duplicate_entry(pool: &SqlitePool, data: &SplitData) -> Result<bool> {
    let last_duration: Option<i32> = sqlx::query_scalar(
        "SELECT duration_ms FROM splits WHERE user = ?1 ORDER BY created_at DESC LIMIT 1"
    )
    .bind(&data.user)
    .fetch_optional(pool)
    .await?;

    Ok(last_duration == Some(data.duration_ms))
}

/// Insert a new split into the database
pub async fn insert_split(pool: &SqlitePool, data: &SplitData) -> Result<()> {
    // Check if this is a duplicate of the user's last entry
    if is_duplicate_entry(pool, data).await? {
        warn!("Ignoring duplicate entry for user {} with duration {}ms", data.user, data.duration_ms);
        return Err(crate::AppError::DuplicateEntry);
    }
    
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
        "{} went {} the {}{} in {}",
        split.user, direction, method, encumbered_text, formatted_duration
    );
    
    if is_wr {
        format!("@here NEW WR! {} 🎉", content)
    } else {
        content
    }
}

/// Get the world record (best time) for each category
pub async fn get_world_records(pool: &SqlitePool) -> Result<Vec<Split>> {
    let mut world_records = Vec::new();

    // Define all possible categories
    let categories = [
        // Elevator categories (is_encumbered is ignored for elevators)
        (true, true, None),   // down elevator
        (false, true, None),  // up elevator
        
        // Stairs categories
        (true, false, Some(true)),   // down stairs encumbered
        (true, false, Some(false)),  // down stairs not encumbered
        (false, false, Some(true)),  // up stairs encumbered
        (false, false, Some(false)), // up stairs not encumbered
    ];

    for (is_down, is_elevator, is_encumbered) in categories {
        let row = if is_elevator {
            // For elevator splits, ignore is_encumbered
            sqlx::query("SELECT id, user, is_down, is_elevator, is_encumbered, duration_ms, created_at FROM splits 
                        WHERE is_down = ?1 AND is_elevator = ?2 
                        ORDER BY duration_ms ASC LIMIT 1")
                .bind(is_down)
                .bind(is_elevator)
                .fetch_optional(pool)
                .await?
        } else {
            // For stairs splits, include is_encumbered
            sqlx::query("SELECT id, user, is_down, is_elevator, is_encumbered, duration_ms, created_at FROM splits 
                        WHERE is_down = ?1 AND is_elevator = ?2 AND is_encumbered = ?3 
                        ORDER BY duration_ms ASC LIMIT 1")
                .bind(is_down)
                .bind(is_elevator)
                .bind(is_encumbered)
                .fetch_optional(pool)
                .await?
        };

        if let Some(row) = row {
            world_records.push(Split {
                id: row.get(0),
                user: row.get(1),
                is_down: row.get(2),
                is_elevator: row.get(3),
                is_encumbered: row.get(4),
                duration_ms: row.get(5),
                created_at: row.get(6),
            });
        }
    }

    Ok(world_records)
}

/// Get the slowest record (worst time) for each category
pub async fn get_slowest_records(pool: &SqlitePool) -> Result<Vec<Split>> {
    let mut slowest_records = Vec::new();
    let categories = [
        (true, true, None),
        (false, true, None),
        (true, false, Some(true)),
        (true, false, Some(false)),
        (false, false, Some(true)),
        (false, false, Some(false)),
    ];
    for (is_down, is_elevator, is_encumbered) in categories {
        let row = if is_elevator {
            sqlx::query("SELECT id, user, is_down, is_elevator, is_encumbered, duration_ms, created_at FROM splits \
                        WHERE is_down = ?1 AND is_elevator = ?2 \
                        ORDER BY duration_ms DESC LIMIT 1")
                .bind(is_down)
                .bind(is_elevator)
                .fetch_optional(pool)
                .await?
        } else {
            sqlx::query("SELECT id, user, is_down, is_elevator, is_encumbered, duration_ms, created_at FROM splits \
                        WHERE is_down = ?1 AND is_elevator = ?2 AND is_encumbered = ?3 \
                        ORDER BY duration_ms DESC LIMIT 1")
                .bind(is_down)
                .bind(is_elevator)
                .bind(is_encumbered)
                .fetch_optional(pool)
                .await?
        };
        if let Some(row) = row {
            slowest_records.push(Split {
                id: row.get(0),
                user: row.get(1),
                is_down: row.get(2),
                is_elevator: row.get(3),
                is_encumbered: row.get(4),
                duration_ms: row.get(5),
                created_at: row.get(6),
            });
        }
    }
    Ok(slowest_records)
}

/// Format world records for display
pub fn format_world_records(world_records: &[Split]) -> String {
    if world_records.is_empty() {
        return "No world records found.".to_string();
    }

    let mut formatted = String::from("**World Records Board:**\n");
    
    for split in world_records {
        let direction = if split.is_down { "Down" } else { "Up" };
        let method = if split.is_elevator { "Elevator" } else { "Stairs" };
        let formatted_duration = DurationValidator::format_duration(split.duration_ms);
        
        let category = if split.is_elevator {
            format!("{} {}", direction, method)
        } else {
            let encumbered_text = match split.is_encumbered {
                Some(true) => " (Encumbered)",
                Some(false) => " (No Items)",
                None => "",
            };
            format!("{} {}{}", direction, method, encumbered_text)
        };
        
        formatted.push_str(&format!(
            "**{}**: {} - {} ({})\n",
            category, split.user, formatted_duration, split.created_at
        ));
    }

    formatted
}
