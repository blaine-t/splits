use crate::database::{get_all_splits, format_splits};
use crate::models::SharedAppContext;
use serenity::async_trait;
use serenity::builder::CreateMessage;
use serenity::model::gateway::Ready;
use serenity::model::id::ChannelId;
use serenity::prelude::*;
use sqlx::SqlitePool;

const DISCORD_CHANNEL_ID: u64 = 1410126283555344396;

pub struct Handler {
    pub context: SharedAppContext,
}

#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, _ctx: Context, ready: Ready) {
        println!("{} is connected!", ready.user.name);
        let mut context = self.context.lock().await;
        context.discord_ctx = Some(_ctx.clone());
    }
}

/// Send splits information to Discord
pub async fn send_split_to_discord(ctx: &Context, pool: &SqlitePool) {
    match get_all_splits(pool).await {
        Ok(splits) => {
            let content = format_splits(&splits);
            let builder = CreateMessage::new().content(content);
            let message = ChannelId::new(DISCORD_CHANNEL_ID)
                .send_message(ctx, builder)
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

/// Create and configure Discord client
pub async fn create_discord_client(token: &str, handler: Handler) -> Result<Client, serenity::Error> {
    let intents = GatewayIntents::GUILDS;
    Client::builder(token, intents)
        .event_handler(handler)
        .await
}
