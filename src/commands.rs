use crate::database::{get_world_records, format_world_records};
use sqlx::SqlitePool;

pub type Error = Box<dyn std::error::Error + Send + Sync>;
pub type Context<'a> = poise::Context<'a, Data, Error>;

// User data passed to all command functions
pub struct Data {
    pub db_pool: SqlitePool,
}

/// Display the world records board showing the best time in each category
#[poise::command(slash_command, rename = "wrboard")]
pub async fn world_records_board(
    ctx: Context<'_>,
) -> Result<(), Error> {
    // Defer the response since database queries might take a moment
    ctx.defer().await?;

    // Get world records from the database
    let world_records = get_world_records(&ctx.data().db_pool).await
        .map_err(|e| format!("Database error: {}", e))?;

    // Format the world records for display
    let response = format_world_records(&world_records);

    // Send the response
    ctx.send(poise::CreateReply::default().content(response)).await?;

    Ok(())
}

/// Display the slowest board showing the worst time in each category
#[poise::command(slash_command, rename = "slowboard")]
pub async fn slowest_board(
    ctx: Context<'_>,
) -> Result<(), Error> {
    ctx.defer().await?;
    let slowest_records = crate::database::get_slowest_records(&ctx.data().db_pool).await
        .map_err(|e| format!("Database error: {}", e))?;
    let response = crate::database::format_world_records(&slowest_records);
    ctx.send(poise::CreateReply::default().content(response)).await?;
    Ok(())
}

/// Register all slash commands
pub fn commands() -> Vec<poise::Command<Data, Error>> {
    vec![
        world_records_board(),
        slowest_board(),
    ]
}
