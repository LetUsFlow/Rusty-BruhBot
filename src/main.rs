use std::collections::HashMap;
use std::env;
use std::error::Error;
use std::time::Duration;

use dotenvy::dotenv;
use log::{error, warn, Level, info};
use once_cell::sync::Lazy;
use serenity::async_trait;
use serenity::model::application::command::Command;
use serenity::model::application::interaction::{Interaction, InteractionResponseType};
use serenity::model::gateway::Ready;
use serenity::model::prelude::command::CommandOptionType;
use serenity::model::prelude::Message;
use serenity::prelude::*;
use tokio::time::sleep;

mod supabase_adapter;

struct Handler;

static COMMANDS: Lazy<Mutex<HashMap<String, String>>> = Lazy::new(|| Mutex::new(HashMap::new()));

#[async_trait]
impl EventHandler for Handler {
    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        if let Interaction::ApplicationCommand(command) = interaction {
            let content = match command.data.name.as_str() {
                "bruh" => {
                    if let Some(cdo) = command.data.options.get(0) {
                        if let Some(sound) = &cdo.value {
                            println!("Play {sound}");
                        }
                    } else {
                        println!("Play bruh");
                    }
                    //":sunglasses:".to_string()
                    "not implemented :(".to_string()
                }
                "ping" => "Hey, I'm alive! Temporarily, at least...".to_string(),
                _ => "not implemented :(".to_string(),
            };

            if let Err(why) = command
                .create_interaction_response(&ctx.http, |response| {
                    response
                        .kind(InteractionResponseType::ChannelMessageWithSource)
                        .interaction_response_data(|message| message.content(content))
                })
                .await
            {
                warn!("Cannot respond to slash command: {why}");
            }
        }
    }

    async fn message(&self, ctx: Context, msg: Message) {
        if msg.content == "!ping" {
            if let Err(why) = msg.channel_id.say(&ctx.http, "Pong!").await {
                warn!("Error sending message: {why:?}");
            }
        }
    }

    async fn ready(&self, ctx: Context, ready: Ready) {
        println!("{} is connected!", ready.user.name);

        let _ = Command::set_global_application_commands(&ctx.http, |commands| {
            commands
                .create_application_command(|command| {
                    command
                        .name("bruh")
                        .description("Play a sound")
                        .create_option(|option| {
                            option
                                .name("sound")
                                .description("Name of sound")
                                .kind(CommandOptionType::String)
                        })
                })
                .create_application_command(|command| {
                    command.name("ping").description("A ping command")
                })
        })
        .await;
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    dotenv().ok();

    simple_logger::init_with_level(Level::Warn).unwrap();

    // Configure the client with your Discord bot token in the environment.
    let token = env::var("DISCORD_TOKEN").expect("Expected DISCORD_TOKEN in the environment");
    let api = env::var("POCKETBASE_API").expect("Expected POCKETBASE_API in the environment");

    COMMANDS.lock().await.extend(
        get_command_data()
            .await
            .unwrap_or_else(|_| panic!("Could not load command data from database: {api}")),
    );

    tokio::spawn(async move {
        loop {
            sleep(Duration::from_secs(10)).await;
            let command_data = get_command_data().await;

            match command_data {
                Ok(data) => {
                    let mut commands = COMMANDS.lock().await;

                    commands.clear();
                    commands.extend(data);
                    info!("Successfully updated command data");
                }
                Err(err) => {
                    warn!("Failed updating command data: {err}");
                }
            }
        }
    });

    // Build our client.
    let mut client = Client::builder(
        token,
        GatewayIntents::GUILD_MESSAGES | GatewayIntents::MESSAGE_CONTENT,
    )
    .event_handler(Handler)
    .await
    .expect("Error creating client");

    // Finally, start a single shard, and start listening to events.
    //
    // Shards will automatically attempt to reconnect, and will perform
    // exponential backoff until it reconnects.
    if let Err(why) = client.start().await {
        error!("Client error: {why:?}");
    }

    Ok(())
}

async fn get_command_data() -> Result<HashMap<String, String>, reqwest::Error> {
    let mut res = HashMap::new();

    let api = env::var("POCKETBASE_API").unwrap();
    let source = supabase_adapter::get_full_list(&api, "sounds").await?;

    for item in source.items {
        res.insert(
            item.command,
            format!(
                "{api}/api/files/{}/{}/{}",
                item.collectionId, item.id, item.audio
            ),
        );
    }
    Ok(res)
}
