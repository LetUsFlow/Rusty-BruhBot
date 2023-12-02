use std::collections::HashSet;
use std::sync::Arc;

use parking_lot::Mutex;
use serenity::all::CommandDataOptionValue;
use serenity::async_trait;
use serenity::builder::{CreateCommand, CreateCommandOption, CreateInteractionResponseMessage, CreateInteractionResponse};
use serenity::model::application::CommandDataOption;
use serenity::model::application::{Command, CommandOptionType};
use serenity::model::application::Interaction;
use serenity::model::gateway::Ready;
use serenity::model::prelude::{GuildId, Message};
use serenity::prelude::{Context, EventHandler};
use songbird::tracks::TrackHandle;
use songbird::{Call, Event, EventContext};
use tracing::{info, warn};

use crate::command_manager::CommandManager;
use crate::player::play_sound;

pub struct DiscordHandler {
    pub connections: Arc<Mutex<HashSet<GuildId>>>,
    pub commands: CommandManager,
}

#[async_trait]
impl EventHandler for DiscordHandler {
    async fn message(&self, ctx: Context, msg: Message) {
        let author_id = msg.author.id;
        let content = msg.content.trim().to_lowercase();
        if content == "brelp" || content == "bruhelp" {
            if let Err(why) = msg
                .channel_id
                .say(&ctx.http, self.commands.list_commands().await)
                .await
            {
                warn!("Error sending message: {why:?}");
            }
        } else if let Some(guild_id) = msg.guild_id {
            play_sound(
                &ctx,
                self,
                guild_id,
                author_id,
                content,
                self.connections.clone(),
            )
            .await;
        }
    }

    async fn ready(&self, ctx: Context, ready: Ready) {
        info!("{} is connected!", ready.user.name);

        Command::create_global_command(
            &ctx.http,
            CreateCommand::new("bruh")
                .description("Play a sound")
                .add_option(CreateCommandOption::new(
                    CommandOptionType::String,
                    "sound",
                    "Name of sound",
                )),
        )
        .await
        .ok();
    }

    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        if let Interaction::Command(command) = interaction {
            let content = match command.data.name.as_str() {
                "bruh" => {
                    let cdo = command.data.options.get(0);

                    match (cdo, command.member.clone()) {
                        (
                            Some(CommandDataOption {
                                value: CommandDataOptionValue::String(sound),
                                ..
                            }),
                            Some(author),
                        ) => {
                            let play_status = play_sound(
                                &ctx,
                                self,
                                author.guild_id,
                                author.user.id,
                                sound.to_string(),
                                self.connections.clone(),
                            )
                            .await;

                            match play_status {
                                true => ":sunglasses:".into(),
                                false => ":skull:".into(),
                            }
                        }
                        (_, None) => {
                            "You shouldn't be here, I shouldn't be here, we both know it...".into()
                        }
                        _ => self.commands.list_commands().await,
                    }
                }
                _ => "i donbt know dis command uwu :(".into(),
            };

            let data = CreateInteractionResponseMessage::new().content(content);

            if let Err(why) = command
                .create_response(&ctx.http, CreateInteractionResponse::Message(data))
                .await
            {
                warn!("Cannot respond to slash command: {why}");
            }
        }
    }
}

pub struct TrackEndNotifier {
    handler_lock: Arc<serenity::prelude::Mutex<Call>>,
}

impl TrackEndNotifier {
    pub fn new(handler_lock: Arc<serenity::prelude::Mutex<Call>>) -> Self {
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
            self.connections.lock().remove(&self.guild_id);
        }
        None
    }
}
