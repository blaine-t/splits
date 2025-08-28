use crate::config::Config;
use crate::database::{get_most_recent_split, format_single_split, is_world_record};
use crate::models::SharedAppContext;
use crate::commands::{Data, Error, commands};
use poise::serenity_prelude as serenity;
use serenity::async_trait;
use serenity::builder::CreateMessage;
use serenity::model::gateway::Ready;
use serenity::model::id::ChannelId;
use serenity::prelude::*;
use sqlx::SqlitePool;
use tracing::{error, info};

pub struct Handler {
    pub context: SharedAppContext,
}

#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, _ctx: Context, ready: Ready) {
        info!("{} bot is connected to Discord!", ready.user.name);
        let mut context = self.context.lock().await;
        context.discord_ctx = Some(_ctx.clone());
    }
}

/// Send splits information to Discord
pub async fn send_split_to_discord(ctx: &Context, pool: &SqlitePool, config: &Config) {
    match get_most_recent_split(pool).await {
        Ok(Some(split)) => {
            // Check if this split is a world record
            match is_world_record(pool, &split).await {
                Ok(is_wr) => {
                    let content = format_single_split(&split, is_wr);
                    let builder = CreateMessage::new().content(content);
                    let message = ChannelId::new(config.discord.channel_id)
                        .send_message(ctx, builder)
                        .await;
                    if let Err(why) = message {
                        error!("Error sending message: {why:?}");
                    }
                }
                Err(e) => {
                    error!("Error checking if split is world record: {}", e);
                }
            }
        }
        Ok(None) => {
            error!("No splits found in database");
        }
        Err(e) => {
            error!("Error getting most recent split for Discord: {}", e);
        }
    }
}

/// Create and configure Discord client with poise framework
pub async fn create_discord_client(config: &Config, handler: Handler) -> Result<serenity::Client, Box<dyn std::error::Error + Send + Sync>> {
    let intents = GatewayIntents::GUILDS | GatewayIntents::GUILD_MESSAGES;
    
    let context_clone = handler.context.clone();
    
    let framework = poise::Framework::builder()
        .options(poise::FrameworkOptions {
            commands: commands(),
            event_handler: |ctx, event, framework, data| {
                Box::pin(event_handler(ctx, event, framework, data))
            },
            ..Default::default()
        })
        .setup(move |ctx, _ready, framework| {
            Box::pin(async move {
                info!("Bot is ready! Registering slash commands...");
                poise::builtins::register_globally(ctx, &framework.options().commands).await?;
                Ok(Data {
                    db_pool: context_clone.lock().await.db_pool.clone(),
                })
            })
        })
        .build();

    let client = serenity::ClientBuilder::new(&config.discord.token, intents)
        .event_handler(handler)
        .framework(framework)
        .await?;

    Ok(client)
}

async fn event_handler(
    _ctx: &serenity::Context,
    event: &serenity::FullEvent,
    _framework: poise::FrameworkContext<'_, Data, Error>,
    _data: &Data,
) -> Result<(), Error> {
    match event {
        serenity::FullEvent::Ready { data_about_bot } => {
            info!("{} bot is connected to Discord!", data_about_bot.user.name);
        }
        _ => {}
    }
    Ok(())
}
