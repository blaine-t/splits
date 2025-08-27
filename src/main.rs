use axum::extract::State;
use axum::Json;
use axum::{Router, routing::get, routing::post};
use sqlx::{SqlitePool, Row};
use serde::Deserialize;
use serenity::async_trait;
use serenity::builder::CreateMessage;
use serenity::model::gateway::Ready;
use serenity::model::id::ChannelId;
use serenity::prelude::*;
use std::env;
use std::sync::Arc;
use tower_http::services::ServeDir;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),
    #[error("Discord error: {0}")]
    Discord(#[from] serenity::Error),
    #[error("Environment variable error: {0}")]
    EnvVar(#[from] std::env::VarError),
    #[error("Network error: {0}")]
    Network(#[from] std::io::Error),
}

type Result<T> = std::result::Result<T, AppError>;

#[derive(Clone)]
struct AppContext {
    discord_ctx: Option<Context>,
    db_pool: SqlitePool,
}

struct Handler {
    context: Arc<Mutex<AppContext>>,
}

#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, _ctx: Context, ready: Ready) {
        println!("{} is connected!", ready.user.name);
        let mut context = self.context.lock().await;
        context.discord_ctx = Some(_ctx.clone());
    }
}

async fn send_split(ctx: &Context, pool: &SqlitePool) {
    match all_splits_internal(pool).await {
        Ok(content) => {
            let builder = CreateMessage::new().content(content);
            let message = ChannelId::new(1410126283555344396)
                .send_message(&ctx, builder)
                .await;
            if let Err(why) = message {
                eprintln!("Error sending message: {why:?}");
            }
        }
        Err(e) => {
            eprintln!("Error getting splits for Discord: {}", e);
        }
    }
}

#[tokio::main]
async fn main() {
    if let Err(e) = run().await {
        eprintln!("Application error: {}", e);
        std::process::exit(1);
    }
}

async fn run() -> Result<()> {
    // Initialize database pool
    let db_pool = SqlitePool::connect("sqlite:splits.db").await?;
    
    // Create table if it doesn't exist
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
    .execute(&db_pool)
    .await?;

    let shared_context = Arc::new(Mutex::new(AppContext { 
        discord_ctx: None,
        db_pool: db_pool.clone(),
    }));

    let token = env::var("DISCORD_TOKEN")?;

    let intents = GatewayIntents::GUILDS;
    let handler = Handler { context: shared_context.clone() };
    let mut client = Client::builder(&token, intents)
        .event_handler(handler)
        .await?;

    // Run Discord client in a separate thread
    tokio::spawn(async move {
        if let Err(why) = client.start().await {
            eprintln!("Client error: {why:?}");
        }
    });

    let app = Router::new()
        .route("/api/v0/split/all", get(all_splits))
        .route("/api/v0/split/new", post(new_split))
        .with_state(shared_context.clone())
        .fallback_service(ServeDir::new("static"));

    let listener = tokio::net::TcpListener::bind("0.0.0.0:7758").await?;
    println!("listening on {}", listener.local_addr()?);
    axum::serve(listener, app).await?;
    
    Ok(())
}

async fn all_splits_internal(pool: &SqlitePool) -> Result<String> {
    let rows = sqlx::query("SELECT id, user, is_down, is_elevator, duration_ms, timestamp FROM splits")
        .fetch_all(pool)
        .await?;

    let result = rows
        .iter()
        .map(|row| {
            let id: i32 = row.get(0);
            let user: String = row.get(1);
            let is_down: bool = row.get(2);
            let is_elevator: bool = row.get(3);
            let duration_ms: i32 = row.get(4);
            let timestamp: String = row.get(5);
            
            let direction = if is_down { "down" } else { "up" };
            let method = if is_elevator { "elevator" } else { "stairs" };
            let seconds = duration_ms / 1000;
            let remaining_ms = duration_ms % 1000;
            let formatted_duration =
                format!("{}m{}s{}ms", seconds / 60, seconds % 60, remaining_ms);
            format!(
                "Entry {}: {} went {} the {} in {} on {}",
                id, user, direction, method, formatted_duration, timestamp
            )
        })
        .collect::<Vec<String>>()
        .join("\n");
    Ok(result)
}

async fn all_splits(State(context): State<Arc<Mutex<AppContext>>>) -> String {
    let ctx = context.lock().await;
    match all_splits_internal(&ctx.db_pool).await {
        Ok(splits) => splits,
        Err(e) => {
            eprintln!("Error getting splits: {}", e);
            "Error retrieving splits".to_string()
        }
    }
}

#[derive(Deserialize)]
struct SplitData {
    user: String,
    is_down: bool,
    is_elevator: bool,
    duration_ms: i32,
}

async fn new_split(State(context): State<Arc<Mutex<AppContext>>>, Json(data): Json<SplitData>) -> &'static str {
    let ctx = context.lock().await;
    
    match sqlx::query(
        "INSERT INTO splits (user, is_down, is_elevator, duration_ms, timestamp) VALUES (?1, ?2, ?3, ?4, datetime('now'))"
    )
    .bind(&data.user)
    .bind(data.is_down)
    .bind(data.is_elevator)
    .bind(data.duration_ms)
    .execute(&ctx.db_pool)
    .await {
        Ok(_) => {
            if let Some(discord_ctx) = &ctx.discord_ctx {
                send_split(discord_ctx, &ctx.db_pool).await;
            }
            "Data inserted!"
        }
        Err(e) => {
            eprintln!("Error inserting split: {}", e);
            "Error inserting data"
        }
    }
}
