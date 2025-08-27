use axum::{Router, routing::get, routing::post};
use splits::{AppContext, AppState, Result, Config};
use splits::database::initialize_database;
use splits::discord::{Handler, create_discord_client};
use splits::handlers::{all_splits, new_split};
use sqlx::SqlitePool;
use std::sync::Arc;
use tokio::sync::Mutex;
use tower_http::services::ServeDir;

#[tokio::main]
async fn main() {
    if let Err(e) = run().await {
        eprintln!("Application error: {}", e);
        std::process::exit(1);
    }
}

async fn run() -> Result<()> {
    // Load configuration from multiple sources
    let config = Config::load()?;
    
    println!("Configuration loaded successfully");
    println!("Server will start on: {}", config.server_address());
    println!("Discord channel ID: {}", config.discord.channel_id);
    println!("Database URL: {}", config.database.url);

    // Initialize database pool with configuration
    let db_pool = SqlitePool::connect(&config.database.url).await?;
    
    // Initialize database tables
    initialize_database(&db_pool).await?;

    let shared_context = Arc::new(Mutex::new(AppContext { 
        discord_ctx: None,
        db_pool: db_pool.clone(),
    }));

    let app_state = AppState {
        context: shared_context.clone(),
        config: config.clone(),
    };

    let handler = Handler { context: shared_context.clone() };
    let mut client = create_discord_client(&config, handler).await?;

    // Run Discord client in a separate thread
    tokio::spawn(async move {
        if let Err(why) = client.start().await {
            eprintln!("Client error: {why:?}");
        }
    });

    let app = Router::new()
        .route("/api/v0/split/all", get(all_splits))
        .route("/api/v0/split/new", post(new_split))
        .with_state(app_state)
        .fallback_service(ServeDir::new(&config.server.static_dir));

    let listener = tokio::net::TcpListener::bind(&config.server_address()).await?;
    println!("listening on {}", listener.local_addr()?);
    axum::serve(listener, app).await?;
    
    Ok(())
}
