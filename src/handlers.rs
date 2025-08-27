use crate::database::{get_all_splits, format_splits, insert_split};
use crate::discord::send_split_to_discord;
use crate::models::{AppState, SplitData};
use axum::extract::State;
use axum::Json;

/// HTTP handler to get all splits
pub async fn all_splits(State(app_state): State<AppState>) -> String {
    let ctx = app_state.context.lock().await;
    match get_all_splits(&ctx.db_pool).await {
        Ok(splits) => format_splits(&splits),
        Err(e) => {
            eprintln!("Error getting splits: {}", e);
            "Error retrieving splits".to_string()
        }
    }
}

/// HTTP handler to create a new split
pub async fn new_split(
    State(app_state): State<AppState>, 
    Json(data): Json<SplitData>
) -> &'static str {
    let ctx = app_state.context.lock().await;
    
    match insert_split(&ctx.db_pool, &data).await {
        Ok(_) => {
            if let Some(discord_ctx) = &ctx.discord_ctx {
                send_split_to_discord(discord_ctx, &ctx.db_pool, &app_state.config).await;
            }
            "Data inserted!"
        }
        Err(e) => {
            eprintln!("Error inserting split: {}", e);
            "Error inserting data"
        }
    }
}
