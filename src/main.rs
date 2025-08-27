use axum::extract::State;
use axum::Json;
use axum::{Router, routing::get, routing::post};
use rusqlite::{Connection, Result};
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

async fn send_split(ctx: &Context) {
    let builder = CreateMessage::new().content(all_splits().await);
    let message = ChannelId::new(1410126283555344396)
        .send_message(&ctx, builder)
        .await;
    if let Err(why) = message {
        eprintln!("Error sending message: {why:?}");
    };
}

#[tokio::main]
async fn main() {
    let shared_context = Arc::new(Mutex::new(AppContext { discord_ctx: None }));

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

    // Initialize database and create table if it doesn't exist
    let conn = get_connection().unwrap();
    conn.execute(
        "CREATE TABLE IF NOT EXISTS splits (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            user TEXT NOT NULL,
            is_down BOOLEAN NOT NULL,
            is_elevator BOOLEAN NOT NULL,
            duration_ms INTEGER NOT NULL,
            timestamp TEXT NOT NULL
        )",
        [],
    )
    .unwrap();

    let app = Router::new()
        .route("/api/v0/split/all", get(all_splits))
        .route("/api/v0/split/new", post(new_split))
        .with_state(shared_context.clone())
        .fallback_service(ServeDir::new("static"));

    let listener = tokio::net::TcpListener::bind("0.0.0.0:7758").await.unwrap();
    println!("listening on {}", listener.local_addr().unwrap());
    axum::serve(listener, app).await.unwrap();
}

#[derive(Debug)]
struct Split {
    id: i32,
    user: String,
    is_down: bool,
    is_elevator: bool,
    duration_ms: i32,
    timestamp: String,
}

fn get_connection() -> Result<Connection> {
    Connection::open("splits.db")
}

async fn all_splits() -> String {
    let conn = get_connection().unwrap();
    let mut stmt = conn
        .prepare("SELECT id, user, is_down, is_elevator, duration_ms, timestamp FROM splits")
        .unwrap();
    let splits = stmt
        .query_map([], |row| {
            Ok(Split {
                id: row.get(0)?,
                user: row.get(1)?,
                is_down: row.get(2)?,
                is_elevator: row.get(3)?,
                duration_ms: row.get(4)?,
                timestamp: row.get(5)?,
            })
        })
        .unwrap();

    let result = splits
        .map(|s| {
            let split = s.unwrap();
            let direction = if split.is_down { "down" } else { "up" };
            let method = if split.is_elevator {
                "elevator"
            } else {
                "stairs"
            };
            let seconds = split.duration_ms / 1000;
            let remaining_ms = split.duration_ms % 1000;
            let formatted_duration =
                format!("{}m{}s{}ms", seconds / 60, seconds % 60, remaining_ms);
            format!(
                "Entry {}: {} went {} the {} in {} on {}",
                split.id, split.user, direction, method, formatted_duration, split.timestamp
            )
        })
        .collect::<Vec<String>>()
        .join("\n");
    result
}

#[derive(Deserialize)]
struct SplitData {
    user: String,
    is_down: bool,
    is_elevator: bool,
    duration_ms: i32,
}

async fn new_split(State(context): State<Arc<Mutex<AppContext>>>, Json(data): Json<SplitData>) -> &'static str {
    let conn = get_connection().unwrap();
    conn.execute(
        "INSERT INTO splits (user, is_down, is_elevator, duration_ms, timestamp) VALUES (?1, ?2, ?3, ?4, datetime('now'))",
        (&data.user, data.is_down, data.is_elevator, data.duration_ms),
    ).unwrap();
    let ctx = context.lock().await;
    if let Some(discord_ctx) = &ctx.discord_ctx {
        send_split(discord_ctx).await;
    }
    "Data inserted!"
}
