use crate::config::Config;
use crate::database::{get_all_splits, format_splits};
use crate::models::SharedAppContext;
use serenity::async_trait;
use serenity::builder::CreateMessage;
use serenity::model::gateway::Ready;
use serenity::model::id::ChannelId;
use serenity::prelude::*;
use sqlx::SqlitePool;

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
pub async fn send_split_to_discord(ctx: &Context, pool: &SqlitePool, config: &Config) {
    match get_all_splits(pool).await {
        Ok(splits) => {
            let content = format_splits(&splits);
            let builder = CreateMessage::new().content(content);
            let message = ChannelId::new(config.discord.channel_id)
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
pub async fn create_discord_client(config: &Config, handler: Handler) -> Result<Client, serenity::Error> {
    let intents = GatewayIntents::GUILDS;
    Client::builder(&config.discord.token, intents)
        .event_handler(handler)
        .await
}
