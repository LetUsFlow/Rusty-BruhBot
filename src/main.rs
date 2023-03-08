use std::env;
use std::error::Error;
use std::sync::Arc;

use dotenvy::dotenv;

use serenity::framework::StandardFramework;
use serenity::prelude::*;

use songbird::SerenityInit;

use tracing::{error, info};

mod command_manager;
mod events;
mod player;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    dotenv().ok();

    // Enable logging
    tracing_subscriber::fmt::init();

    // Configure the client with your Discord bot token in the environment.
    let token = env::var("DISCORD_TOKEN").expect("Expected DISCORD_TOKEN in the environment");

    // Build client
    let intents = GatewayIntents::non_privileged() | GatewayIntents::MESSAGE_CONTENT;
    let mut client = Client::builder(token, intents)
        .framework(StandardFramework::new())
        .event_handler(events::Handler {
            connections: Arc::default(),
            commands: command_manager::CommandManager::new().await,
        })
        .register_songbird()
        .await
        .expect("Error creating client");

    // Start the bot
    if let Err(why) = client.start().await {
        error!("Client error: {why:?}");
    }

    tokio::signal::ctrl_c().await.ok();
    info!("Received Ctrl-C, shutting down.");

    Ok(())
}
