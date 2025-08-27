use axum::{Router, routing::get, routing::post};
use splits::database::{create_sqlite_database_if_does_not_exist, initialize_database};
use splits::discord::{Handler, create_discord_client};
use splits::handlers::{all_splits, new_split};
use splits::signals::shutdown_signal;
use splits::{AppContext, AppState, Config, Result};
use sqlx::SqlitePool;
use std::sync::Arc;
use tokio::sync::Mutex;
use tower_http::services::ServeDir;
use tracing::{debug, error, info};

#[tokio::main]
async fn main() {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    if let Err(e) = run().await {
        error!("Application error: {}", e);
        std::process::exit(1);
    }
}

async fn run() -> Result<()> {
    // Load configuration
    let config = Config::load()?;
    info!("Configuration loaded successfully");

    // Debug logs for extra information
    debug!("Server will start on: {}", config.server_address());
    debug!("Discord channel ID: {}", config.discord.channel_id);
    debug!("Database URL: {}", config.database.url);

    // Create the DB if it doesn't exist already and db type is sqlite
    if config.database.url.starts_with("sqlite:") {
        create_sqlite_database_if_does_not_exist(&config.database.url)?;
    }

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

    let handler = Handler {
        context: shared_context.clone(),
    };
    let mut client = create_discord_client(&config, handler).await?;

    // Run Discord client in a separate thread
    tokio::spawn(async move {
        if let Err(why) = client.start().await {
            error!("Client error: {why:?}");
        }
    });

    let app = Router::new()
        .route("/api/v0/split/all", get(all_splits))
        .route("/api/v0/split/new", post(new_split))
        .with_state(app_state)
        .fallback_service(ServeDir::new(&config.server.static_dir));

    let listener = tokio::net::TcpListener::bind(&config.server_address()).await?;
    info!("listening on {}", listener.local_addr()?);
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;
    Ok(())
}
