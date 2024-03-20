use serenity::{model::gateway::GatewayIntents, Client};
use songbird::SerenityInit;
use tracing::error;

mod command_manager;
mod events;
mod player;

#[tokio::main]
async fn main() {
    // Enable logging
    tracing_subscriber::fmt::init();

    // Configure the client with your Discord bot token in the environment.
    let token = dotenvy::var("DISCORD_TOKEN").expect("Expected DISCORD_TOKEN in the environment");

    // Build client
    let intents = GatewayIntents::non_privileged() | GatewayIntents::MESSAGE_CONTENT;
    let mut client = Client::builder(token, intents)
        .event_handler(events::DiscordHandler::default())
        .register_songbird()
        .await
        .expect("Error creating client");

    // Start the bot
    if let Err(why) = client.start().await {
        error!("Client error: {why:?}");
    }
}
