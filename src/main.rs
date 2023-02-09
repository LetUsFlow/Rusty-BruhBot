use std::collections::HashSet;
use std::env;
use std::error::Error;
use std::sync::Arc;

use dotenvy::dotenv;
use once_cell::sync::Lazy;
use serenity::async_trait;
use serenity::framework::StandardFramework;
use serenity::model::application::command::Command;
use serenity::model::application::interaction::{Interaction, InteractionResponseType};
use serenity::model::gateway::Ready;
use serenity::model::prelude::command::CommandOptionType;
use serenity::model::prelude::{Member, Message, GuildId};
use serenity::prelude::*;
use songbird::tracks::TrackHandle;
use songbird::{Event, EventContext, SerenityInit, TrackEvent, Call, create_player};
use tracing::{error, warn};

mod command_manager;

static CONNECTIONS: Lazy<Mutex<HashSet<GuildId>>> = Lazy::new(|| Mutex::new(HashSet::new()));

struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        if let Interaction::ApplicationCommand(command) = interaction {
            let content = match command.data.name.as_str() {
                "bruh" => {
                    match command.data.options.get(0) {
                        Some(cdo,) => {
                            match command.member.clone() {
                                Some(author) => {
                                    match &cdo.value {
                                        Some(sound) => {
                                            let _status = play_sound(
                                                &ctx,
                                                &author,
                                                sound.as_str().unwrap_or("").to_string()
                                            ).await;
                
                                            ":sunglasses:".to_string()
                                        },
                                        None => "Something went terribly wrong here...".to_string(),
                                    }
                                },
                                None => "Hmm, This is not a guild. Everything is a lie...".to_string(),
                            }
                        },
                        None => "At some point in the future, this command will list you all available sound. But as of now, this message is shown, because the developer of this bot has been too lazy to implement a help message.".to_string()
                    }

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
        if let Some(guild_id) = msg.guild_id {
            if let Some(author) = ctx.cache.member(guild_id, msg.author.id) {
                play_sound(&ctx, &author, msg.content).await;
            }
        }
    }

    async fn ready(&self, ctx: Context, ready: Ready) {
        println!("{} is connected!", ready.user.name);

        Command::set_global_application_commands(&ctx.http, |commands| {
            commands
                .create_application_command(|command| { // Enable autocomplete for all sounds
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
        .await.ok();
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


    if !CONNECTIONS.lock().await.insert(guild_id) {
        return false;
    }

    // Cretae audio source
    let source = match songbird::ytdl(sound_uri).await {
        Ok(source) => source,
        Err(err) => {
            warn!("Err starting source: {err:?}");
            CONNECTIONS.lock().await.remove(&guild_id);
            return false;
        }
    };

    // Create audio handler
    let (audio, audio_handle) = create_player(source);

    let (handler_lock, _join_result) = manager.join(guild_id, connect_to).await;
    let mut handler = handler_lock.lock().await;

    // Add disconnect eventlistener
    handler.add_global_event(
        songbird::Event::Core(songbird::CoreEvent::DriverDisconnect),
        DriverDisconnectNotifier {audio_handle: audio_handle.clone(), guild_id}
    );

    // Start playing audio
    handler.play_only(audio);

    // Add track end eventlistener
    audio_handle.add_event(
        songbird::Event::Track(TrackEvent::End),
        TrackEndNotifier {
            handler_lock: handler_lock.clone(),
            guild_id
        },
    ).ok();

    true
}

struct TrackEndNotifier {
    handler_lock: Arc<Mutex<Call>>,
    guild_id: GuildId
}

#[async_trait]
impl songbird::events::EventHandler for TrackEndNotifier {
    async fn act(&self, _ctx: &EventContext<'_>) -> Option<Event> {
        let mut handler = self.handler_lock.lock().await;
        handler.leave().await.ok();
        CONNECTIONS.lock().await.remove(&self.guild_id);
        None
    }
}

struct DriverDisconnectNotifier {
    audio_handle: TrackHandle,
    guild_id: GuildId
}

#[async_trait]
impl songbird::events::EventHandler for DriverDisconnectNotifier {
    async fn act(&self, ctx: &EventContext<'_>) -> Option<Event> {
        if let EventContext::DriverDisconnect(_data) = ctx {
            //dbg!(data);
            self.audio_handle.stop().ok();
            CONNECTIONS.lock().await.remove(&self.guild_id);
        }
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
    env::var("POCKETBASE_API").expect("Expected POCKETBASE_API in the environment");

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

    tokio::signal::ctrl_c().await.ok();
    println!("Received Ctrl-C, shutting down.");

    Ok(())
}
