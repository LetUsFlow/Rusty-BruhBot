use std::env;
use std::error::Error;
use std::sync::Arc;

use dotenvy::dotenv;
use serenity::async_trait;
use serenity::framework::StandardFramework;
use serenity::model::application::command::Command;
use serenity::model::application::interaction::{Interaction, InteractionResponseType};
use serenity::model::gateway::Ready;
use serenity::model::prelude::command::CommandOptionType;
use serenity::model::prelude::{Member, Message};
use serenity::prelude::*;
use songbird::{Event, EventContext, SerenityInit, TrackEvent, Call};
use tracing::{error, warn};

mod command_manager;

struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        if let Interaction::ApplicationCommand(command) = interaction {
            let content = match command.data.name.as_str() {
                "bruh" => {
                    if let Some(cdo) = command.data.options.get(0) {
                        if let Some(sound) = &cdo.value {
                            println!(
                                "{}",
                                play_sound(
                                    &ctx,
                                    &command.member.clone().unwrap(),
                                    sound.as_str().unwrap().to_string()
                                )
                                .await
                            );
                            println!("Play {sound}");
                        }
                    } else {
                        println!("Play bruh");
                    }
                    //":sunglasses:".to_string()
                    "currently being implemented :(".to_string()
                }
                "ping" => "Hey, I'm alive! Temporarily, at least...".to_string(),
                _ => "i donbt know dis command uwu :(".to_string(),
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

async fn play_sound(ctx: &Context, author: &Member, sound: String) -> bool {
    let sound_uri = command_manager::get_sound_uri(&sound).await;
    let sound_uri = match sound_uri {
        Some(sound_uri) => sound_uri,
        None => return false,
    };

    let guild = match ctx.cache.guild(author.guild_id) {
        Some(guild) => guild,
        None => {
            warn!("Cannot find guild in cache: {}", author.guild_id);
            return false;
        }
    };
    let guild_id = author.guild_id;

    let channel_id = guild
        .voice_states
        .get(&author.user.id)
        .and_then(|voice_state| voice_state.channel_id);

    let connect_to = match channel_id {
        Some(channel) => channel,
        None => {
            warn!("Cannot find channel: {channel_id:?}");
            return false;
        }
    };

    let manager = songbird::get(ctx)
        .await
        .expect("Songbird Voice client placed in at initialisation")
        .clone();

    println!("Joins voicechannel");
    let handler_lock = manager.join(guild_id, connect_to).await.0;
    let mut handler = handler_lock.lock().await;

    let source = match songbird::ytdl(sound_uri).await {
        Ok(source) => source,
        Err(err) => {
            warn!("Err starting source: {err:?}");
            return false;
        }
    };

    println!("Plays sound");
    let track = handler.play_source(source);

    let _ = track.add_event(
        songbird::Event::Track(TrackEvent::End),
        TrackEndNotifier {
            handler_lock: handler_lock.clone(),
        },
    );

    true
}

struct TrackEndNotifier {
    handler_lock: Arc<Mutex<Call>>,
}

#[async_trait]
impl songbird::events::EventHandler for TrackEndNotifier {
    async fn act(&self, _ctx: &EventContext<'_>) -> Option<Event> {
        let mut handler = self.handler_lock.lock().await;
        let _ = handler.leave().await;
        println!("Leaves");
        None
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    dotenv().ok();

    // Enable logging
    tracing_subscriber::fmt::init();

    // Configure the client with your Discord bot token in the environment.
    let token = env::var("DISCORD_TOKEN").expect("Expected DISCORD_TOKEN in the environment");
    let _ = env::var("POCKETBASE_API").expect("Expected POCKETBASE_API in the environment");

    // Load commands and start command updater
    command_manager::setup().await;

    // Build client
    let intents = GatewayIntents::non_privileged() | GatewayIntents::MESSAGE_CONTENT;
    let mut client = Client::builder(token, intents)
        .framework(StandardFramework::new())
        .event_handler(Handler)
        .register_songbird()
        .await
        .expect("Error creating client");

    // Start the bot
    if let Err(why) = client.start().await {
        error!("Client error: {why:?}");
    }

    let _ = tokio::signal::ctrl_c().await;
    println!("Received Ctrl-C, shutting down.");

    Ok(())
}

