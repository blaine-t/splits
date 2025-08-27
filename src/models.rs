use serde::Deserialize;
use serenity::prelude::Context;
use sqlx::SqlitePool;
use std::sync::Arc;
use tokio::sync::Mutex;
use crate::config::Config;
use crate::validation::{UsernameValidator, DurationValidator, FieldValidator, ValidationResult};

#[derive(Debug)]
pub struct Split {
    pub id: i32,
    pub user: String,
    pub is_down: bool,
    pub is_elevator: bool,
    pub duration_ms: i32,
    pub timestamp: String,
}

#[derive(Deserialize, Debug, Clone)]
pub struct SplitData {
    pub user: String,
    pub is_down: bool,
    pub is_elevator: bool,
    pub duration_ms: i32,
}

impl SplitData {
    /// Validate all fields in the SplitData with configuration
    pub fn validate(&self, config: &crate::config::ValidationConfig) -> ValidationResult<()> {
        // Validate username
        UsernameValidator::validate(&self.user, config)?;

        // Validate duration
        DurationValidator::validate(self.duration_ms, config)?;

        // Validate boolean fields
        FieldValidator::validate_boolean(self.is_down, "is_down")?;
        FieldValidator::validate_boolean(self.is_elevator, "is_elevator")?;

        Ok(())
    }

    /// Create a validated SplitData instance with configuration
    pub fn new(user: String, is_down: bool, is_elevator: bool, duration_ms: i32, config: &crate::config::ValidationConfig) -> ValidationResult<Self> {
        let split_data = SplitData {
            user,
            is_down,
            is_elevator,
            duration_ms,
        };
        
        split_data.validate(config)?;
        Ok(split_data)
    }

    /// Get formatted duration for display
    pub fn formatted_duration(&self) -> String {
        DurationValidator::format_duration(self.duration_ms)
    }
}

#[derive(Clone)]
pub struct AppContext {
    pub discord_ctx: Option<Context>,
    pub db_pool: SqlitePool,
}

#[derive(Clone)]
pub struct AppState {
    pub context: Arc<Mutex<AppContext>>,
    pub config: Config,
}

pub type SharedAppContext = Arc<Mutex<AppContext>>;
