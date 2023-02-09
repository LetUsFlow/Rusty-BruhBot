use std::collections::HashMap;
use std::env;
use std::error::Error;
use std::sync::Arc;
use std::time::Duration;

use dotenvy::dotenv;
use serenity::framework::StandardFramework;
use songbird::Call;
use tokio::time::sleep;
use once_cell::sync::Lazy;
use serenity::prelude::*;
use serenity::async_trait;
use serenity::model::application::command::Command;
use serenity::model::application::interaction::{Interaction, InteractionResponseType};
use serenity::model::gateway::Ready;
use serenity::model::prelude::command::CommandOptionType;
use serenity::model::prelude::{Message, Member};
use songbird::{SerenityInit, TrackEvent, EventContext, Event};
use tracing::error;
use tracing::info;
use tracing::warn;

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
                            println!("{}", play_sound(&ctx, &command.member.clone().unwrap(), sound.as_str().unwrap().to_string()).await);
                            println!("Play {sound}");
                        }
                    } else {
                        println!("Play bruh");
                    }
                    //":sunglasses:".to_string()
                    "currently being implemented :(".to_string()
                },
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

async fn get_sound_uri(sound: &String) -> Option<String> {
    let commands = COMMANDS.lock().await;

    commands.get(sound).cloned()
}

async fn play_sound(ctx: &Context, author: &Member, sound: String) -> bool {
    let sound_uri = get_sound_uri(&sound).await;
    let sound_uri = match sound_uri {
        Some(sound_uri) => sound_uri,
        None => return false,
    };

    let guild = match ctx.cache.guild(author.guild_id) {
        Some(guild) => guild,
        None => {
            warn!("Cannot find guild in cache: {}", author.guild_id);
            return false;
        },
    };
    let guild_id = author.guild_id;

    let channel_id = guild
        .voice_states.get(&author.user.id)
        .and_then(|voice_state| voice_state.channel_id);

    let connect_to = match channel_id {
        Some(channel) => channel,
        None => {
            warn!("Cannot find channel: {channel_id:?}");
            return false;
        }
    };

    let manager = songbird::get(ctx).await
        .expect("Songbird Voice client placed in at initialisation.").clone();

    println!("Joins voicechannel");
    let handler_lock = manager.join(guild_id, connect_to).await.0;
    let mut handler = handler_lock.lock().await;

    let source = match songbird::ytdl(sound_uri).await {
        Ok(source) => source,
        Err(err) => {
            warn!("Err starting source: {err:?}");
            return false;
        },
    };

    println!("Plays sound");
    let track = handler.play_source(source);

    let _ = track.add_event(songbird::Event::Track(TrackEvent::End), TrackEndNotifier {handler_lock: handler_lock.clone()});

    true
}

struct TrackEndNotifier {
    handler_lock: Arc<Mutex<Call>>
}

#[async_trait]
impl songbird::events::EventHandler for TrackEndNotifier {
    async fn act(&self, _ctx: &EventContext<'_>) -> Option<Event> {
        let mut handler = self.handler_lock.lock().await;
        let _ = handler.leave().await;
        println!("left from event");
        None
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    dotenv().ok();

    tracing_subscriber::fmt::init();

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
            sleep(Duration::from_secs(60)).await;
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
        GatewayIntents::non_privileged() | GatewayIntents::MESSAGE_CONTENT
    )
    .framework(StandardFramework::new())
    .event_handler(Handler)
    .register_songbird()
    .await
    .expect("Error creating client");

    // Finally, start a single shard, and start listening to events.
    //
    // Shards will automatically attempt to reconnect, and will perform
    // exponential backoff until it reconnects.
    if let Err(why) = client.start().await {
        error!("Client error: {why:?}");
    }

    let _ = tokio::signal::ctrl_c().await;
    println!("Received Ctrl-C, shutting down.");

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
