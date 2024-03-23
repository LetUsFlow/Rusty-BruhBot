use std::{collections::HashSet, sync::Arc};

use serenity::{
    async_trait,
    builder::{
        AutocompleteChoice, CreateAutocompleteResponse, CreateCommand, CreateCommandOption,
        CreateInteractionResponse, CreateInteractionResponseMessage,
    },
    client::Context,
    model::{
        application::{
            Command, CommandDataOption, CommandDataOptionValue, CommandOptionType, Interaction,
        },
        channel::Message,
        gateway::Ready,
        id::GuildId,
    },
};
use songbird::{tracks::TrackHandle, Call, Event, EventContext};
use tokio::sync::Mutex;
use tracing::{info, warn};

use crate::command_manager::CommandManager;
use crate::player::{play_sound, PlayStatus};

#[derive(Default)]
pub struct DiscordHandler {
    connections: Arc<Mutex<HashSet<GuildId>>>,
    command_manager: CommandManager,
}

#[async_trait]
impl serenity::client::EventHandler for DiscordHandler {
    async fn ready(&self, ctx: Context, ready: Ready) {
        info!("{} is connected!", ready.user.name);

        Command::create_global_command(
            &ctx.http,
            CreateCommand::new("bruhelp").description("BruhBot Help"),
        )
        .await
        .expect("Created global bruhelp command");
        Command::create_global_command(
            &ctx.http,
            CreateCommand::new("brelp").description("BruhBot Help"),
        )
        .await
        .expect("Created global brelp command");

        Command::create_global_command(
            &ctx.http,
            CreateCommand::new("bruh")
                .description("Play a sound")
                .add_option(
                    CreateCommandOption::new(CommandOptionType::String, "sound", "Name of sound")
                        .set_autocomplete(true)
                        .required(true),
                ),
        )
        .await
        .expect("Created global bruh command");
    }

    async fn message(&self, ctx: Context, msg: Message) {
        let author_id = msg.author.id;
        let content = msg.content.trim().to_lowercase();
        if let Some(guild_id) = msg.guild_id {
            play_sound(
                &ctx,
                guild_id,
                author_id,
                self.command_manager
                    .get_sound_uri(&content.trim().to_lowercase())
                    .await,
                self.connections.clone(),
            )
            .await;
        }
    }

    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        match interaction {
            Interaction::Command(command) => {
                let content = match command.data.name.as_str() {
                    "bruh" => {
                        let cdo = command.data.options.first();

                        match (cdo, command.member.clone()) {
                            (
                                Some(CommandDataOption {
                                    value: CommandDataOptionValue::String(sound),
                                    ..
                                }),
                                Some(author),
                            ) => {
                                let sound_name = sound.trim().to_lowercase();

                                // command in guild
                                let play_status = play_sound(
                                    &ctx,
                                    author.guild_id,
                                    author.user.id,
                                    self.command_manager.get_sound_uri(&sound_name).await,
                                    self.connections.clone(),
                                )
                                .await;

                                match play_status {
                                    PlayStatus::AlreadyPlaying => "Already playing a sound, just wait a sec! :)".into(),
                                    PlayStatus::SoundNotFound => "This sound doesn't exist :skull:".into(),
                                    PlayStatus::StartedPlaying => format!("Started playing {} :sunglasses:", sound_name),
                                    PlayStatus::VoiceChannelNotFound => "You are not joined any voice channel on this server :clown:".into(),
                                    PlayStatus::GuildNotFound => "Could not find guild :person_shrugging:".into(),
                                    PlayStatus::JoinError => "Could not join voice channel :person_shrugging:".into(),
                                }
                            }
                            (_, None) => {
                                // command in dms
                                "You shouldn't be here, I shouldn't be here, we both know it..."
                                    .into()
                            }
                            _ => self.command_manager.get_human_readable_command_list().await,
                        }
                    }
                    "bruhelp" | "brelp" => {
                        self.command_manager.get_human_readable_command_list().await
                    }
                    _ => "i donbt know dis command uwu :(".into(),
                };

                if let Err(why) = command
                    .create_response(
                        &ctx.http,
                        CreateInteractionResponse::Message(
                            CreateInteractionResponseMessage::new().content(content),
                        ),
                    )
                    .await
                {
                    warn!("Cannot respond to slash command: {why}");
                }
            }
            Interaction::Autocomplete(autocomplete) => {
                let command_text = autocomplete.data.options[0].value.as_str().unwrap();

                let options = self
                    .command_manager
                    .get_commands()
                    .await
                    .into_iter()
                    .filter(|cmd| cmd.starts_with(command_text))
                    .take(25)
                    .map(|cmd| AutocompleteChoice::new(cmd.clone(), cmd.clone()))
                    .collect::<Vec<AutocompleteChoice>>();

                if let Err(why) = autocomplete
                    .create_response(
                        &ctx.http,
                        CreateInteractionResponse::Autocomplete(
                            CreateAutocompleteResponse::new().set_choices(options),
                        ),
                    )
                    .await
                {
                    warn!("Cannot respond to slash command autocompletion request: {why}");
                }
            }
            _ => {}
        }
    }
}

pub struct TrackEndNotifier {
    handler_lock: Arc<Mutex<Call>>,
}

impl TrackEndNotifier {
    pub fn new(handler_lock: Arc<Mutex<Call>>) -> Self {
        Self { handler_lock }
    }
}

#[async_trait]
impl songbird::events::EventHandler for TrackEndNotifier {
    async fn act(&self, _ctx: &EventContext<'_>) -> Option<Event> {
        let mut handler = self.handler_lock.lock().await;
        handler.leave().await.ok();
        None
    }
}

pub struct DriverDisconnectNotifier {
    audio_handle: TrackHandle,
    guild_id: GuildId,
    connections: Arc<Mutex<HashSet<GuildId>>>,
}

impl DriverDisconnectNotifier {
    pub fn new(
        audio_handle: TrackHandle,
        guild_id: GuildId,
        connections: Arc<Mutex<HashSet<GuildId>>>,
    ) -> Self {
        Self {
            audio_handle,
            guild_id,
            connections,
        }
    }
}

#[async_trait]
impl songbird::events::EventHandler for DriverDisconnectNotifier {
    async fn act(&self, ctx: &EventContext<'_>) -> Option<Event> {
        if let EventContext::DriverDisconnect(_data) = ctx {
            self.audio_handle.stop().ok();
            self.connections.lock().await.remove(&self.guild_id);
        }
        None
    }
}
