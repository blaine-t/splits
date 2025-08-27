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
    let builder = CreateMessage::new().content(all_splits_internal(pool).await);
    let message = ChannelId::new(1410126283555344396)
        .send_message(&ctx, builder)
        .await;
    if let Err(why) = message {
        eprintln!("Error sending message: {why:?}");
    };
}

#[tokio::main]
async fn main() {
    // Initialize database pool
    let db_pool = SqlitePool::connect("sqlite:splits.db").await.unwrap();
    
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
    .await
    .unwrap();

    let shared_context = Arc::new(Mutex::new(AppContext { 
        discord_ctx: None,
        db_pool: db_pool.clone(),
    }));

    let token = env::var("DISCORD_TOKEN").expect("Expected a token in the environment");

    let intents = GatewayIntents::GUILDS;
    let handler = Handler { context: shared_context.clone() };
    let mut client = Client::builder(&token, intents)
        .event_handler(handler)
        .await
        .expect("Error creating client");

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

    let listener = tokio::net::TcpListener::bind("0.0.0.0:7758").await.unwrap();
    println!("listening on {}", listener.local_addr().unwrap());
    axum::serve(listener, app).await.unwrap();
}

async fn all_splits_internal(pool: &SqlitePool) -> String {
    let rows = sqlx::query("SELECT id, user, is_down, is_elevator, duration_ms, timestamp FROM splits")
        .fetch_all(pool)
        .await
        .unwrap();

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
    result
}

async fn all_splits(State(context): State<Arc<Mutex<AppContext>>>) -> String {
    let ctx = context.lock().await;
    all_splits_internal(&ctx.db_pool).await
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
    
    sqlx::query(
        "INSERT INTO splits (user, is_down, is_elevator, duration_ms, timestamp) VALUES (?1, ?2, ?3, ?4, datetime('now'))"
    )
    .bind(&data.user)
    .bind(data.is_down)
    .bind(data.is_elevator)
    .bind(data.duration_ms)
    .execute(&ctx.db_pool)
    .await
    .unwrap();
    
    if let Some(discord_ctx) = &ctx.discord_ctx {
        send_split(discord_ctx, &ctx.db_pool).await;
    }
    "Data inserted!"
}
