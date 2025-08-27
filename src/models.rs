use serde::Deserialize;
use serenity::prelude::Context;
use sqlx::SqlitePool;
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Debug)]
pub struct Split {
    pub id: i32,
    pub user: String,
    pub is_down: bool,
    pub is_elevator: bool,
    pub duration_ms: i32,
    pub timestamp: String,
}

#[derive(Deserialize)]
pub struct SplitData {
    pub user: String,
    pub is_down: bool,
    pub is_elevator: bool,
    pub duration_ms: i32,
}

#[derive(Clone)]
pub struct AppContext {
    pub discord_ctx: Option<Context>,
    pub db_pool: SqlitePool,
}

pub type SharedAppContext = Arc<Mutex<AppContext>>;
