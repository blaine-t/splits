use crate::database::{format_splits, get_all_splits, insert_split};
use crate::discord::send_split_to_discord;
use crate::models::{AppState, SplitData};
use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use tracing::{debug, error, info, warn};

/// HTTP handler to get all splits
pub async fn all_splits(State(app_state): State<AppState>) -> String {
    let ctx = app_state.context.lock().await;
    match get_all_splits(&ctx.db_pool).await {
        Ok(splits) => {
            debug!("Sending {} splits to client", splits.len());
            format_splits(&splits)
        }
        Err(e) => {
            error!("Error getting splits: {}", e);
            "Error retrieving splits".to_string()
        }
    }
}

/// HTTP handler to create a new split with validation
pub async fn new_split(State(app_state): State<AppState>, Json(data): Json<SplitData>) -> Response {
    // Validate the input data using configuration
    if let Err(validation_error) = data.validate(&app_state.config.validation) {
        warn!("Validation error: {}", validation_error);
        return (
            StatusCode::BAD_REQUEST,
            format!("Validation failed: {}", validation_error),
        )
            .into_response();
    }

    let ctx = app_state.context.lock().await;

    match insert_split(&ctx.db_pool, &data).await {
        Ok(_) => {
            info!("New split: {:?}", data);

            if let Some(discord_ctx) = &ctx.discord_ctx {
                send_split_to_discord(discord_ctx, &ctx.db_pool, &app_state.config).await;
            }

            (StatusCode::CREATED, "Data inserted successfully!").into_response()
        }
        Err(e) => {
            error!("Error inserting split: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, "Error inserting data").into_response()
        }
    }
}
